import { Link } from "@tanstack/react-router";
import { useId, useState } from "react";
import { Arabic } from "@/components/Arabic";
import { Dialog } from "@/components/Dialog";
import type { Card } from "@/lib/content";
import { trackLabels, varietyLabels } from "@/lib/contentLabels";
import { cardState } from "@/lib/review";
import { AudioBlock } from "@/pages/admin/AudioBlock";
import { RepoBadge } from "@/pages/admin/contentParts";
import type { ReviewActions, ReviewData } from "@/pages/admin/ReviewLessonPage";
import {
	CardStatePill,
	countCards,
	describeReview,
	FlagIcon,
	LessonAudio,
	VisibilityPanel,
} from "@/pages/admin/reviewParts";

/**
 * A new card's heading, or the end screen's, takes the focus as it
 * appears, so that a screen reader reads it next.
 */
function focusOnMount(node: HTMLHeadingElement | null) {
	node?.focus();
}

interface ViewProps {
	data: ReviewData;
	actions: ReviewActions;
	error: string | undefined;
}

const MAX_NOTE_CHARS = 2000;

function firstToReview(cards: Card[]): number {
	const index = cards.findIndex((card) => cardState(card) === "to_review");
	return Math.max(index, 0);
}

/** The phone's review: one card at a time, then what is left to do. */
export function ReviewPhone({ data, actions, error }: ViewProps) {
	const { cards } = data;
	const [index, setIndex] = useState(() => firstToReview(cards));
	const [done, setDone] = useState(cards.length === 0);
	const [flagging, setFlagging] = useState(false);
	const card = cards[index];

	function advance() {
		if (index + 1 < cards.length) {
			setIndex(index + 1);
		} else {
			setDone(true);
		}
	}

	if (done || !card) {
		return (
			<DoneScreen
				data={data}
				actions={actions}
				error={error}
				headingRef={focusOnMount}
				onRestart={() => {
					setIndex(0);
					setDone(false);
				}}
			/>
		);
	}

	const state = cardState(card);
	return (
		<div className="review-phone">
			<div className="review-phone-top">
				<Link
					to="/admin/review"
					className="review-close"
					aria-label="Revenir à la liste des leçons"
				>
					<CloseIcon />
				</Link>
				<span
					className="review-progress review-progress-thick"
					aria-hidden="true"
				>
					<span style={{ width: `${((index + 1) / cards.length) * 100}%` }} />
				</span>
				<span className="review-position">
					<span className="visually-hidden">Carte </span>
					{index + 1}/{cards.length}
				</span>
			</div>

			<div className="review-phone-body">
				<div className="review-phone-heading">
					<span className="review-crumb">
						{trackLabels[data.unit.track]} · {data.unit.title}
					</span>
					<h1
						key={card.id}
						ref={focusOnMount}
						tabIndex={-1}
						className="review-phone-title"
					>
						{data.lesson.title}
					</h1>
				</div>

				<article className="review-card" aria-label={`Carte ${index + 1}`}>
					<div className="review-card-badges">
						<span className="variety-pill">{varietyLabels[card.variety]}</span>
						{card.importKey ? <RepoBadge /> : null}
						<CardStatePill state={state} />
					</div>
					<p className="review-card-arabic">
						<Arabic>{card.arabic}</Arabic>
					</p>
					<dl className="review-card-text">
						<dt>Translittération</dt>
						<dd className="review-transliteration">
							{card.transliteration ?? "—"}
						</dd>
						<dt>Sens</dt>
						<dd>{card.meaningFr ?? "—"}</dd>
					</dl>
					<AudioBlock
						key={card.id}
						audio={card.audio}
						label="Prononciation"
						recordLabel="Enregistrer la prononciation"
						compact
						onUpload={actions.uploadCardAudio(card)}
						onRemove={actions.removeCardAudio(card)}
					/>
					{card.reviewNote ? (
						<div className="review-note-box">
							<span className="review-note-label">Ta remarque</span>
							<span className="review-note-text">{card.reviewNote}</span>
							<button
								type="button"
								className="review-note-edit"
								onClick={() => setFlagging(true)}
							>
								Modifier la remarque
							</button>
						</div>
					) : null}
				</article>
				{card.importKey ? (
					<p className="review-footnote">
						Texte géré dans le repo : une erreur signalée y est corrigée.
					</p>
				) : null}
				{error ? (
					<p role="alert" className="form-error">
						{error}
					</p>
				) : null}
			</div>

			<div className="review-phone-bar">
				<button
					type="button"
					className="review-bar-previous"
					aria-label="Carte précédente"
					disabled={index === 0}
					onClick={() => setIndex(index - 1)}
				>
					<PreviousIcon />
				</button>
				<button
					type="button"
					className="review-bar-flag"
					disabled={actions.busy}
					onClick={() => setFlagging(true)}
				>
					<FlagIcon />
					Signaler
				</button>
				<button
					type="button"
					className="review-bar-validate"
					disabled={actions.busy}
					onClick={async () => {
						if (state === "validated" || (await actions.validate(card))) {
							advance();
						}
					}}
				>
					<CheckIcon />
					{state === "validated" ? "Suivante" : "Valider"}
				</button>
			</div>

			{flagging ? (
				<FlagDialog
					initial={card.reviewNote ?? ""}
					busy={actions.busy}
					onCancel={() => setFlagging(false)}
					onSubmit={async (note) => {
						if (await actions.flag(card, note)) {
							setFlagging(false);
							advance();
						}
					}}
				/>
			) : null}
		</div>
	);
}

interface FlagDialogProps {
	initial: string;
	busy: boolean;
	onCancel: () => void;
	onSubmit: (note: string) => void;
}

function FlagDialog({ initial, busy, onCancel, onSubmit }: FlagDialogProps) {
	const [note, setNote] = useState(initial);
	const fieldId = useId();
	const hintId = useId();
	return (
		<Dialog title="Signaler une erreur" onClose={onCancel}>
			<form
				className="review-flag-form"
				onSubmit={(event) => {
					event.preventDefault();
					onSubmit(note);
				}}
			>
				<label htmlFor={fieldId} className="field-label">
					Qu'est-ce qui ne va pas ?
				</label>
				<textarea
					id={fieldId}
					className="field-input field-textarea"
					rows={4}
					maxLength={MAX_NOTE_CHARS}
					aria-describedby={hintId}
					placeholder="Ex. : la vocalisation du 2e mot est fausse"
					value={note}
					onChange={(event) => setNote(event.target.value)}
				/>
				<span id={hintId} className="field-hint">
					La carte reste « à relire », invisible des apprenants.
				</span>
				<div className="dialog-actions">
					<button type="button" className="button-secondary" onClick={onCancel}>
						Annuler
					</button>
					<button
						type="submit"
						className="button-danger-solid"
						disabled={busy || note.trim() === ""}
					>
						Signaler et continuer
					</button>
				</div>
			</form>
		</Dialog>
	);
}

interface DoneScreenProps extends ViewProps {
	headingRef: (node: HTMLHeadingElement | null) => void;
	onRestart: () => void;
}

function DoneScreen({
	data,
	actions,
	error,
	headingRef,
	onRestart,
}: DoneScreenProps) {
	const { cards, lesson, unit } = data;
	const flagged = cards.filter((card) => cardState(card) === "flagged");
	return (
		<div className="review-phone">
			<div className="review-phone-body">
				<div className="review-done-head">
					<span className="review-done-icon" aria-hidden="true">
						<CheckIcon />
					</span>
					<h1 ref={headingRef} tabIndex={-1} className="review-phone-title">
						{cards.length === 0 ? lesson.title : "Leçon relue"}
					</h1>
					<p className="review-summary">
						{cards.length === 0
							? "Cette leçon n'a aucune carte."
							: describeReview(countCards(cards))}
					</p>
				</div>

				{flagged.length > 0 ? (
					<section className="review-section" aria-labelledby="signalees">
						<h2 id="signalees" className="review-section-title">
							Signalées
						</h2>
						<ul className="review-flagged">
							{flagged.map((card) => (
								<li key={card.id}>
									<Arabic>{card.arabic}</Arabic>
									<span>{card.reviewNote}</span>
								</li>
							))}
						</ul>
					</section>
				) : null}

				<section className="review-section" aria-labelledby="et-aussi">
					<h2 id="et-aussi" className="review-section-title">
						Et aussi
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
				{error ? (
					<p role="alert" className="form-error">
						{error}
					</p>
				) : null}
			</div>
			<div className="review-phone-bar">
				<Link to="/admin/review" className="review-bar-secondary">
					Toutes les leçons
				</Link>
				{data.nextId ? (
					<Link
						to="/admin/review/lessons/$lessonId"
						params={{ lessonId: data.nextId }}
						className="review-bar-validate"
					>
						Leçon suivante
					</Link>
				) : cards.length > 0 ? (
					<button
						type="button"
						className="review-bar-validate"
						onClick={onRestart}
					>
						Revoir les cartes
					</button>
				) : null}
			</div>
		</div>
	);
}

function Icon({
	children,
	width = 2.4,
}: {
	children: React.ReactNode;
	width?: number;
}) {
	return (
		<svg
			width="20"
			height="20"
			viewBox="0 0 24 24"
			fill="none"
			stroke="currentColor"
			strokeWidth={width}
			strokeLinecap="round"
			strokeLinejoin="round"
			aria-hidden="true"
		>
			{children}
		</svg>
	);
}

function CloseIcon() {
	return (
		<Icon>
			<path d="M6 6l12 12M18 6 6 18" />
		</Icon>
	);
}

function PreviousIcon() {
	return (
		<Icon>
			<path d="m15 5-7 7 7 7" />
		</Icon>
	);
}

function CheckIcon() {
	return (
		<Icon width={3}>
			<path d="m5 12.5 4.5 4.5L19 7.5" />
		</Icon>
	);
}
