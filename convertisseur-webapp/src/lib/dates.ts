const dayFormat = new Intl.DateTimeFormat("fr-FR", {
	day: "numeric",
	month: "short",
	year: "numeric",
});
const relative = new Intl.RelativeTimeFormat("fr", { numeric: "auto" });

const MINUTE = 60_000;
const HOUR = 60 * MINUTE;
const DAY = 24 * HOUR;

export function formatDay(iso: string): string {
	return dayFormat.format(new Date(iso));
}

/** "à l'instant", "il y a 5 minutes", "hier", then the date past a week. */
export function formatLastActivity(
	iso: string | null,
	now: Date = new Date(),
): string {
	if (iso === null) {
		return "Jamais";
	}
	const elapsed = now.getTime() - new Date(iso).getTime();
	if (elapsed < MINUTE) {
		return "à l'instant";
	}
	if (elapsed < HOUR) {
		return relative.format(-Math.floor(elapsed / MINUTE), "minute");
	}
	if (elapsed < DAY) {
		return relative.format(-Math.floor(elapsed / HOUR), "hour");
	}
	if (elapsed < 7 * DAY) {
		return relative.format(-Math.floor(elapsed / DAY), "day");
	}
	return formatDay(iso);
}

const shortDayFormat = new Intl.DateTimeFormat("fr-FR", {
	day: "numeric",
	month: "short",
});

/** "7 nov.": the day and month, for recent edits. */
export function formatShortDay(iso: string): string {
	return shortDayFormat.format(new Date(iso));
}
