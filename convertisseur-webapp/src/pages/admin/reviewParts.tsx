import type { Card, Lesson, Status, Unit } from "@/lib/content";
import { countOf } from "@/lib/contentLabels";
import { type CardState, cardState, type LessonSummary } from "@/lib/review";
import { AudioBlock } from "@/pages/admin/AudioBlock";

const stateLabels: Record<CardState, string> = {
	validated: "Validée",
	flagged: "Signalée",
	to_review: "À relire",
};

export function CardStatePill({ state }: { state: CardState }) {
	return (
		<span className={`status-pill status-pill-${state}`}>
			{stateLabels[state]}
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

/** How the cards stand, counted in the browser. */
export function countCards(cards: Card[]): CardCounts {
	const counts = { validated: 0, flagged: 0, toReview: 0, withoutAudio: 0 };
	for (const card of cards) {
		const state = cardState(card);
		if (state === "validated") {
			counts.validated += 1;
		} else if (state === "flagged") {
			counts.flagged += 1;
		} else {
			counts.toReview += 1;
		}
		if (card.audio === null) {
			counts.withoutAudio += 1;
		}
	}
	return counts;
}

export function totalOf(counts: CardCounts): number {
	return counts.validated + counts.flagged + counts.toReview;
}

/** "3/7 validées · 3 à relire · 1 signalée · 2 sans audio", in short. */
export function describeCounts(counts: CardCounts): string {
	const parts = [`${counts.validated}/${totalOf(counts)} validées`];
	if (counts.toReview > 0) {
		parts.push(`${counts.toReview} à relire`);
	}
	if (counts.flagged > 0) {
		parts.push(countOf(counts.flagged, "signalée", "signalées"));
	}
	if (counts.withoutAudio > 0) {
		parts.push(`${counts.withoutAudio} sans audio`);
	}
	return parts.join(" · ");
}

/** "2 validées · 1 signalée · 4 à relire", for a lesson's summary. */
export function describeReview(counts: CardCounts): string {
	return [
		countOf(counts.validated, "validée", "validées"),
		countOf(counts.flagged, "signalée", "signalées"),
		`${counts.toReview} à relire`,
	].join(" · ");
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
					? "Les cartes validées de cette leçon sont visibles des apprenants."
					: "Les apprenants ne voient une carte que si sa leçon et son unité sont validées aussi."}
			</p>
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
}

/** The lesson's own recording, which learners may do without. */
export function LessonAudio({ lesson, onUpload, onRemove }: LessonAudioProps) {
	return (
		<AudioBlock
			audio={lesson.audio}
			label="Audio de la leçon (facultatif)"
			compact
			onUpload={onUpload}
			onRemove={onRemove}
		/>
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
