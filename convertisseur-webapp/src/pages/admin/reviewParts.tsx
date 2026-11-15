import { type RefObject, useEffect, useId, useRef, useState } from "react";
import type { Audio, Card, Lesson, Status, Unit } from "@/lib/content";
import { countOf } from "@/lib/contentLabels";
import { type CardState, cardState, type LessonSummary } from "@/lib/review";
import { AudioBlock } from "@/pages/admin/AudioBlock";

const stateLabels: Record<CardState, string> = {
	validated: "Validée",
	flagged: "Signalée",
	to_review: "À relire",
};

const audioStateLabels: Record<CardState, string> = {
	validated: "Validé",
	flagged: "Signalé",
	to_review: "À écouter",
};

export function CardStatePill({ state }: { state: CardState }) {
	return (
		<span className={`status-pill status-pill-${state}`}>
			{stateLabels[state]}
		</span>
	);
}

export function AudioStatePill({ state }: { state: CardState }) {
	return (
		<span className={`status-pill status-pill-${state}`}>
			{audioStateLabels[state]}
		</span>
	);
}

/** A bar filled in proportion of the validated cards. */
export function Progress({ done, total }: { done: number; total: number }) {
	const percent = total === 0 ? 0 : Math.round((done / total) * 100);
	return (
		<span className="review-progress" aria-hidden="true">
			<span style={{ width: `${percent}%` }} />
		</span>
	);
}

export type CardCounts = LessonSummary["cardCounts"];
type StateCounts = CardCounts["recordings"];

const stateKeys: Record<CardState, keyof StateCounts> = {
	validated: "validated",
	flagged: "flagged",
	to_review: "toReview",
};

/** How the cards and their recordings stand, counted in the browser. */
export function countCards(cards: Card[]): CardCounts {
	const counts = { validated: 0, flagged: 0, toReview: 0, withoutAudio: 0 };
	const recordings = { validated: 0, flagged: 0, toReview: 0 };
	for (const card of cards) {
		counts[stateKeys[cardState(card)]] += 1;
		if (card.audio === null) {
			counts.withoutAudio += 1;
		} else {
			recordings[stateKeys[cardState(card.audio)]] += 1;
		}
	}
	return { ...counts, recordings };
}

export function totalOf(counts: StateCounts): number {
	return counts.validated + counts.flagged + counts.toReview;
}

/** "Texte : 2 validées · 1 signalée · 4 à relire", for a lesson's summary. */
export function describeReview(counts: CardCounts): string {
	return `Texte : ${[
		countOf(counts.validated, "validée", "validées"),
		countOf(counts.flagged, "signalée", "signalées"),
		`${counts.toReview} à relire`,
	].join(" · ")}`;
}

/** "Audio : 2 validés · 1 signalé · 3 à écouter · 1 manquant". */
export function describeRecordings(counts: CardCounts): string {
	const { recordings } = counts;
	return `Audio : ${[
		countOf(recordings.validated, "validé", "validés"),
		countOf(recordings.flagged, "signalé", "signalés"),
		`${recordings.toReview} à écouter`,
		countOf(counts.withoutAudio, "manquant", "manquants"),
	].join(" · ")}`;
}

interface StatusSwitchProps {
	label: string;
	title: string;
	status: Status;
	validateLabel: string;
	busy: boolean;
	onChange: (status: Status) => void;
}

function StatusSwitch({
	label,
	title,
	status,
	validateLabel,
	busy,
	onChange,
}: StatusSwitchProps) {
	const validated = status === "validated";
	return (
		<div className="review-switch">
			<span className="review-switch-text">
				<span className="review-switch-label">{label}</span>
				<span className="review-switch-title">{title}</span>
			</span>
			<button
				type="button"
				className={validated ? "button-review" : "button-solid"}
				disabled={busy}
				onClick={() => onChange(validated ? "to_review" : "validated")}
			>
				{validated ? "Repasser à relire" : validateLabel}
			</button>
		</div>
	);
}

interface VisibilityPanelProps {
	lesson: Lesson;
	unit: Unit;
	busy: boolean;
	onStatus: (kind: "lesson" | "unit", status: Status) => void;
}

/** Whether learners see the lesson: its status, its unit's, and the rule. */
export function VisibilityPanel({
	lesson,
	unit,
	busy,
	onStatus,
}: VisibilityPanelProps) {
	const visible = lesson.status === "validated" && unit.status === "validated";
	return (
		<div className="review-visibility">
			<StatusSwitch
				label="Leçon"
				title={lesson.title}
				status={lesson.status}
				validateLabel="Valider la leçon"
				busy={busy}
				onChange={(status) => onStatus("lesson", status)}
			/>
			<StatusSwitch
				label="Unité"
				title={unit.title}
				status={unit.status}
				validateLabel="Valider l'unité"
				busy={busy}
				onChange={(status) => onStatus("unit", status)}
			/>
			<p className="review-rule">
				{visible
					? "Les apprenants voient les cartes validées de cette leçon, et entendent leur audio une fois validé."
					: "Les apprenants ne voient une carte que si sa leçon et son unité sont validées aussi, et n'entendent son audio qu'une fois validé."}
			</p>
		</div>
	);
}

/** What a reviewer does to a recording; each one resolves once it is saved. */
export interface RecordingActions {
	busy: boolean;
	validateAudio: (audio: Audio) => Promise<boolean>;
	sendBackAudio: (audio: Audio) => Promise<boolean>;
	flagAudio: (audio: Audio, note: string) => Promise<boolean>;
}

interface RecordingReviewProps {
	audio: Audio;
	/** Completes the flag button's name: "de la carte 3". */
	of: string;
	actions: RecordingActions;
}

/** A recording's own review: its state, its note, validating or flagging it. */
export function RecordingReview({ audio, of, actions }: RecordingReviewProps) {
	const state = cardState(audio);
	const [flagging, setFlagging] = useState(false);
	const flagButton = useRef<HTMLButtonElement>(null);
	return (
		<>
			<span className="review-row-actions">
				<AudioStatePill state={state} />
				<button
					ref={flagButton}
					type="button"
					className="review-row-flag"
					aria-label={`Signaler l'audio ${of}`}
					aria-expanded={flagging}
					onClick={() => setFlagging(true)}
				>
					<FlagIcon />
				</button>
				<button
					type="button"
					className={state === "validated" ? "button-review" : "button-solid"}
					disabled={actions.busy}
					onClick={() =>
						void (state === "validated"
							? actions.sendBackAudio(audio)
							: actions.validateAudio(audio))
					}
				>
					{state === "validated" ? "Repasser à écouter" : "Valider l'audio"}
				</button>
			</span>
			{audio.reviewNote && !flagging ? (
				<div className="review-note-box review-note-inline">
					<span className="review-note-label">Remarque</span>
					<span className="review-note-text">{audio.reviewNote}</span>
				</div>
			) : null}
			{flagging ? (
				<NoteForm
					initial={audio.reviewNote ?? ""}
					busy={actions.busy}
					placeholder="Ex. : le 2e mot est mal prononcé"
					returnFocus={flagButton}
					onCancel={() => setFlagging(false)}
					onSubmit={async (note) => {
						if (await actions.flagAudio(audio, note)) {
							setFlagging(false);
						}
					}}
				/>
			) : null}
		</>
	);
}

const MAX_NOTE_CHARS = 2000;

interface NoteFormProps {
	initial: string;
	busy: boolean;
	placeholder: string;
	/** The button the focus goes back to when the form closes. */
	returnFocus: RefObject<HTMLButtonElement | null>;
	onCancel: () => void;
	onSubmit: (note: string) => void;
}

/**
 * What is wrong, inline. Not a `<form>`: it may sit in the edit page's own
 * form, which cannot hold another.
 */
export function NoteForm({
	initial,
	busy,
	placeholder,
	returnFocus,
	onCancel,
	onSubmit,
}: NoteFormProps) {
	const [note, setNote] = useState(initial);
	const field = useRef<HTMLTextAreaElement>(null);
	const fieldId = useId();

	useEffect(() => {
		field.current?.focus();
		const back = returnFocus.current;
		return () => back?.focus();
	}, [returnFocus]);

	return (
		<div className="review-flag-form">
			<label htmlFor={fieldId} className="field-label">
				Qu'est-ce qui ne va pas ?
			</label>
			<textarea
				ref={field}
				id={fieldId}
				className="field-input field-textarea"
				rows={2}
				maxLength={MAX_NOTE_CHARS}
				placeholder={placeholder}
				value={note}
				onChange={(event) => setNote(event.target.value)}
				onKeyDown={(event) => {
					if (event.key === "Escape") {
						onCancel();
					}
				}}
			/>
			<div className="dialog-actions">
				<button type="button" className="button-small" onClick={onCancel}>
					Annuler
				</button>
				<button
					type="button"
					className="button-danger-solid"
					disabled={busy || note.trim() === ""}
					onClick={() => onSubmit(note)}
				>
					Signaler
				</button>
			</div>
		</div>
	);
}

interface LessonAudioProps {
	lesson: Lesson;
	onUpload: (
		file: Blob,
		filename: string,
		durationMs: number | undefined,
	) => Promise<void>;
	onRemove: () => Promise<void>;
	actions: RecordingActions;
}

/** The lesson's own recording, which learners may do without. */
export function LessonAudio({
	lesson,
	onUpload,
	onRemove,
	actions,
}: LessonAudioProps) {
	return (
		<AudioBlock
			audio={lesson.audio}
			label="Audio de la leçon (facultatif)"
			compact
			onUpload={onUpload}
			onRemove={onRemove}
		>
			{lesson.audio ? (
				<RecordingReview
					audio={lesson.audio}
					of="de la leçon"
					actions={actions}
				/>
			) : null}
		</AudioBlock>
	);
}

export function FlagIcon() {
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
			<path d="M5.5 21V4M5.5 4h11l-2.2 4 2.2 4h-11" />
		</svg>
	);
}
