import { Link } from "@tanstack/react-router";
import { useEffect, useId, useRef, useState } from "react";
import { Arabic } from "@/components/Arabic";
import type { Card } from "@/lib/content";
import { trackLabels } from "@/lib/contentLabels";
import { cardState } from "@/lib/review";
import { AudioBlock } from "@/pages/admin/AudioBlock";
import { RepoBadge, StatusPill } from "@/pages/admin/contentParts";
import type { ReviewActions, ReviewData } from "@/pages/admin/ReviewLessonPage";
import {
	CardStatePill,
	countCards,
	describeReview,
	FlagIcon,
	LessonAudio,
	Progress,
	totalOf,
	VisibilityPanel,
} from "@/pages/admin/reviewParts";

interface ViewProps {
	data: ReviewData;
	actions: ReviewActions;
	error: string | undefined;
}

const MAX_NOTE_CHARS = 2000;

/** The desktop's review: every card of the lesson on one page. */
export function ReviewDesktop({ data, actions, error }: ViewProps) {
	const { lesson, unit, cards } = data;
	const [flagging, setFlagging] = useState<string | undefined>();
	const counts = countCards(cards);

	return (
		<div className="admin-content review-desktop">
			<header className="review-desktop-head">
				<div className="review-desktop-titles">
					<span className="review-crumb">
						<Link to="/admin/relecture">Relecture</Link> ·{" "}
						{trackLabels[unit.track]} · Unité {unit.position} · {unit.title}
					</span>
					<div className="review-desktop-title-row">
						<h1 className="admin-title">{lesson.title}</h1>
						<StatusPill status={lesson.status} />
						{lesson.importKey ? <RepoBadge /> : null}
					</div>
				</div>
				<nav aria-label="Leçons à relire" className="review-lesson-nav">
					<LessonLink id={data.previousId} label="Leçon précédente" previous />
					<LessonLink id={data.nextId} label="Leçon suivante" />
				</nav>
			</header>

			<div className="review-desktop-columns">
				<aside className="review-side">
					<section className="review-panel" aria-labelledby="lecons-unite">
						<h2
							id="lecons-unite"
							className="review-section-title review-panel-title"
						>
							Leçons de l'unité
						</h2>
						<ul className="review-side-lessons">
							{data.siblings.map((sibling) => (
								<li key={sibling.id}>
									<Link
										to="/admin/relecture/lecons/$lessonId"
										params={{ lessonId: sibling.id }}
										className="review-side-lesson"
									>
										<span className="review-lesson-title">{sibling.title}</span>
										<span className="review-lesson-meta">
											<Progress
												done={sibling.cardCounts.validated}
												total={totalOf(sibling.cardCounts)}
											/>
											<span>
												{sibling.cardCounts.validated}/
												{totalOf(sibling.cardCounts)}
											</span>
										</span>
									</Link>
								</li>
							))}
						</ul>
					</section>
					<section
						className="review-panel review-panel-padded"
						aria-labelledby="visible"
					>
						<h2 id="visible" className="review-section-title">
							Visible des apprenants ?
						</h2>
						<VisibilityPanel
							lesson={lesson}
							unit={unit}
							busy={actions.busy}
							onStatus={actions.setStatusOf}
						/>
						<LessonAudio
							lesson={lesson}
							onUpload={actions.uploadLessonAudio}
							onRemove={actions.removeLessonAudio}
						/>
					</section>
				</aside>

				<section className="review-cards" aria-label="Cartes de la leçon">
					<div className="review-summary-bar">
						<span className="review-summary">
							{cards.length === 0
								? "Cette leçon n'a aucune carte."
								: describeReview(counts)}
						</span>
						{counts.toReview > 0 ? (
							<button
								type="button"
								className="button-solid"
								disabled={actions.busy}
								onClick={() => void actions.validateRest()}
							>
								{counts.toReview === 1
									? "Valider la carte restante"
									: `Valider les ${counts.toReview} cartes restantes`}
							</button>
						) : null}
					</div>
					{error ? (
						<p role="alert" className="form-error">
							{error}
						</p>
					) : null}
					<ol className="review-card-list">
						{cards.map((card, index) => (
							<li key={card.id}>
								<CardRow
									card={card}
									number={index + 1}
									actions={actions}
									flagging={flagging === card.id}
									onFlag={() => setFlagging(card.id)}
									onFlagDone={() => setFlagging(undefined)}
								/>
							</li>
						))}
					</ol>
					{cards.some((card) => card.importKey) ? (
						<p className="review-footnote">
							Texte géré dans le repo : une carte signalée reste « à relire »
							jusqu'à sa correction.
						</p>
					) : null}
				</section>
			</div>
		</div>
	);
}

function LessonLink({
	id,
	label,
	previous = false,
}: {
	id: string | undefined;
	label: string;
	previous?: boolean;
}) {
	const content = (
		<>
			{previous ? <Chevron previous /> : null}
			{label}
			{previous ? null : <Chevron />}
		</>
	);
	if (!id) {
		return (
			<button
				type="button"
				className="button-secondary review-lesson-link"
				disabled
			>
				{content}
			</button>
		);
	}
	return (
		<Link
			to="/admin/relecture/lecons/$lessonId"
			params={{ lessonId: id }}
			className="button-secondary review-lesson-link"
		>
			{content}
		</Link>
	);
}

interface CardRowProps {
	card: Card;
	number: number;
	actions: ReviewActions;
	flagging: boolean;
	onFlag: () => void;
	onFlagDone: () => void;
}

function CardRow({
	card,
	number,
	actions,
	flagging,
	onFlag,
	onFlagDone,
}: CardRowProps) {
	const state = cardState(card);
	const flagButton = useRef<HTMLButtonElement>(null);
	const field = useRef<HTMLTextAreaElement>(null);
	const [note, setNote] = useState(card.reviewNote ?? "");
	const fieldId = useId();
	const wasFlagging = useRef(false);

	// The form takes the focus when it opens, and gives it back to the flag
	// button when it closes.
	useEffect(() => {
		if (flagging) {
			setNote(card.reviewNote ?? "");
			field.current?.focus();
		} else if (wasFlagging.current) {
			flagButton.current?.focus();
		}
		wasFlagging.current = flagging;
	}, [flagging, card.reviewNote]);

	return (
		<article
			className={`review-row review-row-${state}`}
			aria-label={`Carte ${number}`}
		>
			<div className="review-row-main">
				<span className="review-row-number" aria-hidden="true">
					{number}
				</span>
				<span className="review-row-arabic">
					<Arabic>{card.arabic}</Arabic>
				</span>
				<span className="review-row-text">
					<span className="review-transliteration">
						{card.transliteration ?? "—"}
					</span>
					<span>{card.meaningFr ?? "—"}</span>
				</span>
				<span className="review-row-actions">
					<CardStatePill state={state} />
					<button
						ref={flagButton}
						type="button"
						className="review-row-flag"
						aria-label={`Signaler la carte ${number}`}
						aria-expanded={flagging}
						onClick={onFlag}
					>
						<FlagIcon />
					</button>
					<button
						type="button"
						className={state === "validated" ? "button-review" : "button-solid"}
						disabled={actions.busy}
						onClick={() =>
							void (state === "validated"
								? actions.sendBack(card)
								: actions.validate(card))
						}
					>
						{state === "validated" ? "Repasser à relire" : "Valider"}
					</button>
				</span>
			</div>
			<AudioBlock
				audio={card.audio}
				label="Prononciation"
				recordLabel="Enregistrer la prononciation"
				compact
				onUpload={actions.uploadCardAudio(card)}
				onRemove={actions.removeCardAudio(card)}
			/>
			{card.reviewNote && !flagging ? (
				<div className="review-note-box review-note-inline">
					<span className="review-note-label">Remarque</span>
					<span className="review-note-text">{card.reviewNote}</span>
				</div>
			) : null}
			{flagging ? (
				<form
					className="review-flag-form"
					onSubmit={async (event) => {
						event.preventDefault();
						if (await actions.flag(card, note)) {
							onFlagDone();
						}
					}}
					onKeyDown={(event) => {
						if (event.key === "Escape") {
							onFlagDone();
						}
					}}
				>
					<label htmlFor={fieldId} className="field-label">
						Qu'est-ce qui ne va pas ?
					</label>
					<textarea
						ref={field}
						id={fieldId}
						className="field-input field-textarea"
						rows={2}
						maxLength={MAX_NOTE_CHARS}
						placeholder="Ex. : la vocalisation du 2e mot est fausse"
						value={note}
						onChange={(event) => setNote(event.target.value)}
					/>
					<div className="dialog-actions">
						<button type="button" className="button-small" onClick={onFlagDone}>
							Annuler
						</button>
						<button
							type="submit"
							className="button-danger-solid"
							disabled={actions.busy || note.trim() === ""}
						>
							Signaler
						</button>
					</div>
				</form>
			) : null}
		</article>
	);
}

function Chevron({ previous = false }: { previous?: boolean }) {
	return (
		<svg
			width="18"
			height="18"
			viewBox="0 0 24 24"
			fill="none"
			stroke="currentColor"
			strokeWidth="2.4"
			strokeLinecap="round"
			strokeLinejoin="round"
			aria-hidden="true"
		>
			<path d={previous ? "m15 5-7 7 7 7" : "m9 5 7 7-7 7"} />
		</svg>
	);
}
