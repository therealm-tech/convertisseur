import { Link, useNavigate } from "@tanstack/react-router";
import { useEffect, useState } from "react";
import { ApiError } from "@/lib/api";
import { type Card, getItem, type Lesson } from "@/lib/content";
import { countOf, trackLabels } from "@/lib/contentLabels";
import type { ContentSearch } from "@/lib/contentRoutes";
import {
	cardCounts,
	type Filters,
	filterCards,
	neighbours,
	type TreeData,
} from "@/lib/contentTree";
import { cardState, fetchLessonCards } from "@/lib/review";
import { ContentFilters } from "@/pages/admin/ContentFilters";
import { ContentCrumbs } from "@/pages/admin/contentNav";
import {
	ItemTitle,
	NoteFlag,
	StatusPill,
	useContentFailure,
} from "@/pages/admin/contentParts";
import {
	CardStatePill,
	countCards,
	Progress,
	totalOf,
} from "@/pages/admin/reviewParts";

type Loading =
	| { state: "loading" }
	| { state: "missing" }
	| { state: "failed" }
	| { state: "ready"; lesson: Lesson; cards: Card[] };

interface LessonCardsProps {
	lessonId: string;
	search: ContentSearch;
	filters: Filters;
	/** For the previous and next lessons; undefined until loaded. */
	tree: TreeData | undefined;
	onFilter: (change: Partial<ContentSearch>) => void;
}

/** One lesson's cards in order, filtered, with the ways to work on it. */
export function LessonCards({
	lessonId,
	search,
	filters,
	tree,
	onFilter,
}: LessonCardsProps) {
	const onFailure = useContentFailure();
	const [loading, setLoading] = useState<Loading>({ state: "loading" });

	useEffect(() => {
		let current = true;
		Promise.all([getItem("lesson", lessonId), fetchLessonCards(lessonId)]).then(
			([lesson, cards]) => {
				if (current) {
					setLoading({ state: "ready", lesson, cards });
				}
			},
			(failure: unknown) => {
				if (current) {
					setLoading(
						failure instanceof ApiError && failure.status === 404
							? { state: "missing" }
							: { state: "failed" },
					);
					onFailure("admin.lesson_cards_failed", failure);
				}
			},
		);
		return () => {
			current = false;
		};
	}, [lessonId, onFailure]);

	if (loading.state !== "ready") {
		return (
			<>
				<ContentCrumbs search={search} trail={[]} />
				{loading.state === "loading" ? (
					<p className="admin-table-message">Chargement…</p>
				) : (
					<p role="alert" className="form-error">
						{loading.state === "missing"
							? "Leçon introuvable."
							: "Impossible de charger cette leçon. Réessaie dans un moment."}
					</p>
				)}
			</>
		);
	}

	const { lesson, cards } = loading;
	const counts = countCards(cards);
	const total = totalOf(counts);
	const shown = filterCards(cards, filters);
	const around = tree ? neighbours(tree, filters, lesson.id) : undefined;
	return (
		<>
			<div className="content-lesson-head">
				<ContentCrumbs
					search={search}
					trail={[
						{
							label: trackLabels[lesson.track],
							search: { track: lesson.track },
						},
						{ label: lesson.unit.title, search: { unit: lesson.unit.id } },
					]}
				/>
				<div className="admin-header-row">
					<h1 className="admin-title">{lesson.title}</h1>
					<div className="admin-header-actions">
						<Link
							to="/admin/content/$section/$itemId"
							params={{ section: "lessons", itemId: lesson.id }}
							state={{ contentList: search }}
							className="button-outline"
						>
							Modifier la leçon
						</Link>
						<Link
							to="/admin/content/$section/new"
							params={{ section: "cards" }}
							search={{ parent: lesson.id }}
							state={{ contentList: search }}
							className="button-outline"
						>
							Nouvelle carte
						</Link>
						<Link
							to="/admin/review/lessons/$lessonId"
							params={{ lessonId: lesson.id }}
							className="button-solid"
						>
							Relire
						</Link>
					</div>
				</div>
				<p className="review-lesson-meta content-lesson-stats">
					<StatusPill status={lesson.status} />
					{lesson.reviewNote ? <NoteFlag /> : null}
					<span>{countOf(total, "carte", "cartes")}</span>
					{total > 0 ? (
						<>
							<Progress done={counts.validated} total={total} />
							<span>{countOf(counts.validated, "validée", "validées")}</span>
						</>
					) : null}
					{counts.withoutAudio > 0 ? (
						<span>{`${counts.withoutAudio} sans audio`}</span>
					) : null}
				</p>
			</div>

			<ContentFilters
				filters={filters}
				counts={cardCounts(cards, filters)}
				onFilter={onFilter}
			/>

			<div className="admin-table-card">
				{shown.length === 0 ? (
					<p className="admin-table-message">
						{cards.length === 0
							? "Cette leçon n'a pas encore de carte."
							: "Aucune carte ne correspond."}
					</p>
				) : (
					<CardTable cards={shown} search={search} />
				)}
			</div>

			{around?.previous || around?.next ? (
				<nav aria-label="Leçons" className="content-lesson-pager">
					{around.previous ? (
						<Link
							to="/admin/content"
							search={{ ...search, lesson: around.previous.id }}
						>
							{`‹ ${around.previous.title}`}
						</Link>
					) : (
						<span />
					)}
					{around.next ? (
						<Link
							to="/admin/content"
							search={{ ...search, lesson: around.next.id }}
						>
							{`${around.next.title} ›`}
						</Link>
					) : null}
				</nav>
			) : null}
		</>
	);
}

function CardTable({
	cards,
	search,
}: {
	cards: Card[];
	search: ContentSearch;
}) {
	const navigate = useNavigate();
	return (
		<table className="admin-table lesson-cards">
			<thead>
				<tr>
					<th scope="col" className="lesson-cards-position">
						#
					</th>
					<th scope="col">Carte</th>
					<th scope="col">Sens</th>
					<th scope="col">Audio</th>
					<th scope="col">Relecture</th>
				</tr>
			</thead>
			<tbody>
				{cards.map((card) => (
					<tr
						key={card.id}
						className="content-row"
						onClick={(event) => {
							const target = event.target;
							const onLink =
								target instanceof Element && target.closest("a") !== null;
							if (!onLink && !window.getSelection()?.toString()) {
								void navigate({
									to: "/admin/content/$section/$itemId",
									params: { section: "cards", itemId: card.id },
									state: { contentList: search },
								});
							}
						}}
					>
						<td className="lesson-cards-position">{card.position}</td>
						<th scope="row">
							<Link
								to="/admin/content/$section/$itemId"
								params={{ section: "cards", itemId: card.id }}
								state={{ contentList: search }}
								className="content-title-link"
							>
								<span>
									<ItemTitle
										title={card.arabic}
										transliteration={card.transliteration}
										arabic
									/>
								</span>
							</Link>
							{card.reviewNote ? <NoteFlag /> : null}
						</th>
						<td className="lesson-cards-meaning">{card.meaningFr}</td>
						<td className="lesson-cards-audio">{card.audio ? "Oui" : "Non"}</td>
						<td className="lesson-cards-state">
							<CardStatePill state={cardState(card)} />
						</td>
					</tr>
				))}
			</tbody>
		</table>
	);
}
