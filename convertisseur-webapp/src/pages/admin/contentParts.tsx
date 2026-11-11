import { Arabic } from "@/components/Arabic";
import type { Status } from "@/lib/content";
import { statusLabels } from "@/lib/contentLabels";

export function StatusPill({ status }: { status: Status }) {
	return (
		<span className={`status-pill status-pill-${status}`}>
			{status === "validated" ? <CheckIcon /> : <ClockIcon />}
			{statusLabels[status]}
		</span>
	);
}

function CheckIcon() {
	return (
		<svg
			width="14"
			height="14"
			viewBox="0 0 24 24"
			fill="none"
			stroke="currentColor"
			strokeWidth="3"
			strokeLinecap="round"
			strokeLinejoin="round"
			aria-hidden="true"
		>
			<path d="m5 12.5 4.5 4.5L19 7.5" />
		</svg>
	);
}

function ClockIcon() {
	return (
		<svg
			width="14"
			height="14"
			viewBox="0 0 24 24"
			fill="none"
			stroke="currentColor"
			strokeWidth="2.6"
			strokeLinecap="round"
			aria-hidden="true"
		>
			<circle cx="12" cy="12" r="8.5" />
			<path d="M12 7.5V12l3 2" />
		</svg>
	);
}

export function NoteIcon() {
	return (
		<svg
			width="16"
			height="16"
			viewBox="0 0 24 24"
			fill="none"
			stroke="currentColor"
			strokeWidth="2.4"
			strokeLinecap="round"
			strokeLinejoin="round"
			aria-hidden="true"
		>
			<path d="M5 5h14v10H10l-5 4z" />
			<path d="M9 9h6M9 12h4" />
		</svg>
	);
}

/** Marks a row of the list whose item carries a review note. */
export function NoteFlag() {
	return (
		<span className="note-flag">
			<NoteIcon />
			Remarque
		</span>
	);
}

/** Marks an item whose text and place live in the repository's files. */
export function RepoBadge() {
	return (
		<span className="repo-badge">
			<svg
				width="14"
				height="14"
				viewBox="0 0 24 24"
				fill="none"
				stroke="currentColor"
				strokeWidth="2.6"
				strokeLinecap="round"
				strokeLinejoin="round"
				aria-hidden="true"
			>
				<rect x="5" y="11" width="14" height="9" rx="2" />
				<path d="M8 11V8a4 4 0 0 1 8 0v3" />
			</svg>
			Géré dans le repo
		</span>
	);
}

/** A card's Arabic, then its transliteration; any other item's title. */
export function ItemTitle({
	title,
	transliteration,
	arabic,
}: {
	title: string;
	transliteration: string | null;
	arabic: boolean;
}) {
	if (!arabic) {
		return <>{title}</>;
	}
	return (
		<>
			<Arabic>{title}</Arabic>
			{transliteration ? ` · ${transliteration}` : null}
		</>
	);
}
