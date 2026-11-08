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
