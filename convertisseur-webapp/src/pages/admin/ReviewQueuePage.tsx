import { Link } from "@tanstack/react-router";
import { useEffect, useState } from "react";
import { countOf, trackLabels } from "@/lib/contentLabels";
import { logger } from "@/lib/logger";
import {
	type LessonSummary,
	listAllLessons,
	type ReviewTotals,
} from "@/lib/review";
import { describeCounts, Progress, totalOf } from "@/pages/admin/reviewParts";

type Filter = "pending" | "silent";

type Listing =
	| { state: "loading" }
	| { state: "failed" }
	| { state: "ready"; lessons: LessonSummary[]; totals: ReviewTotals };

interface UnitGroup {
	unit: LessonSummary["unit"];
	lessons: LessonSummary[];
}

function groupByUnit(lessons: LessonSummary[]): UnitGroup[] {
	const groups: UnitGroup[] = [];
	for (const lesson of lessons) {
		const last = groups[groups.length - 1];
		if (last?.unit.id === lesson.unit.id) {
			last.lessons.push(lesson);
		} else {
			groups.push({ unit: lesson.unit, lessons: [lesson] });
		}
	}
	return groups;
}

const emptyMessages: Record<Filter, string> = {
	pending: "Rien à relire.",
	silent: "Toutes les cartes ont leur audio.",
};

/** The lessons with something to review or to record, by unit. */
export function ReviewQueuePage() {
	const [filter, setFilter] = useState<Filter>("pending");
	const [listing, setListing] = useState<Listing>({ state: "loading" });

	useEffect(() => {
		let current = true;
		setListing({ state: "loading" });
		listAllLessons(
			filter === "pending" ? { pendingReview: true } : { missingAudio: true },
		).then(
			({ lessons, totals }) => {
				if (current) {
					setListing({ state: "ready", lessons, totals });
				}
			},
			(failure: unknown) => {
				logger.warn("admin.review_queue_failed", { error: failure });
				if (current) {
					setListing({ state: "failed" });
				}
			},
		);
		return () => {
			current = false;
		};
	}, [filter]);

	return (
		<div className="admin-content review-queue">
			<header className="admin-header">
				<h1 className="admin-title">Relecture</h1>
				{listing.state === "ready" ? (
					<p className="admin-lead">
						{countOf(
							listing.totals.itemsToReview,
							"élément à relire",
							"éléments à relire",
						)}{" "}
						·{" "}
						{countOf(
							listing.totals.cardsWithoutAudio,
							"carte sans audio",
							"cartes sans audio",
						)}
					</p>
				) : null}
			</header>

			<fieldset className="status-filter review-filter">
				<legend className="visually-hidden">Leçons</legend>
				<button
					type="button"
					aria-pressed={filter === "pending"}
					onClick={() => setFilter("pending")}
				>
					À relire
				</button>
				<button
					type="button"
					aria-pressed={filter === "silent"}
					onClick={() => setFilter("silent")}
				>
					Sans audio
				</button>
			</fieldset>

			{listing.state === "loading" ? (
				<p role="status" className="admin-table-message">
					Chargement…
				</p>
			) : null}
			{listing.state === "failed" ? (
				<p role="alert" className="form-error">
					La liste n'a pas pu être chargée. Réessaie dans un moment.
				</p>
			) : null}
			{listing.state === "ready" && listing.lessons.length === 0 ? (
				<div className="review-empty">
					<p>{emptyMessages[filter]}</p>
					<Link to="/admin/contenu" className="button-outline">
						Voir le contenu
					</Link>
				</div>
			) : null}
			{listing.state === "ready" ? (
				<div className="review-groups">
					{groupByUnit(listing.lessons).map((group) => (
						<UnitSection key={group.unit.id} group={group} />
					))}
				</div>
			) : null}
		</div>
	);
}

function UnitSection({ group }: { group: UnitGroup }) {
	const { unit, lessons } = group;
	return (
		<section className="review-group" aria-label={unit.title}>
			<div className="review-group-head">
				<h2 className="review-group-title">
					{trackLabels[unit.track]} · {unit.title}
				</h2>
				<span className="review-group-status">
					{unit.status === "validated" ? "Unité validée" : "Unité à relire"}
				</span>
			</div>
			<ul className="review-lessons">
				{lessons.map((lesson) => (
					<li key={lesson.id}>
						<Link
							to="/admin/relecture/lecons/$lessonId"
							params={{ lessonId: lesson.id }}
							className="review-lesson"
						>
							<span className="review-lesson-text">
								<span className="review-lesson-title">{lesson.title}</span>
								<span className="review-lesson-meta">
									<Progress
										done={lesson.cardCounts.validated}
										total={totalOf(lesson.cardCounts)}
									/>
									<span>{describeLesson(lesson)}</span>
								</span>
							</span>
							<ChevronIcon />
						</Link>
					</li>
				))}
			</ul>
		</section>
	);
}

function describeLesson(lesson: LessonSummary): string {
	if (totalOf(lesson.cardCounts) === 0) {
		return lesson.status === "validated"
			? "Aucune carte"
			: "Aucune carte · leçon à relire";
	}
	return describeCounts(lesson.cardCounts);
}

function ChevronIcon() {
	return (
		<svg
			width="20"
			height="20"
			viewBox="0 0 24 24"
			fill="none"
			stroke="currentColor"
			strokeWidth="2.2"
			strokeLinecap="round"
			strokeLinejoin="round"
			aria-hidden="true"
		>
			<path d="m9 5 7 7-7 7" />
		</svg>
	);
}
