import type { Audio, Kind, Status, Track, Variety } from "@/lib/content";

export const trackLabels: Record<Track, string> = {
	arabe: "Arabe",
	islam: "Islam",
};

export const kindLabels: Record<Kind, string> = {
	unit: "Unité",
	lesson: "Leçon",
	card: "Carte",
};

export const statusLabels: Record<Status, string> = {
	to_review: "À relire",
	validated: "Validé",
};

export const varietyLabels: Record<Variety, string> = {
	classique: "Arabe classique",
	darija: "Darija algérienne",
};

const formats: Record<string, string> = {
	"audio/mpeg": "MP3",
	"audio/mp4": "M4A",
	"audio/ogg": "OGG",
	"audio/webm": "WebM",
	"audio/wav": "WAV",
};

const sizeFormat = new Intl.NumberFormat("fr-FR", {
	maximumFractionDigits: 1,
});

/** "1:05" from milliseconds. */
export function formatDuration(ms: number): string {
	const seconds = Math.round(ms / 1000);
	return `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, "0")}`;
}

function formatSize(bytes: number): string {
	if (bytes < 1_000_000) {
		return `${sizeFormat.format(Math.max(1, Math.round(bytes / 1000)))} ko`;
	}
	return `${sizeFormat.format(bytes / 1_000_000)} Mo`;
}

/** "MP3 · 1,2 Mo · 0:03": the format, the size, and the length if known. */
export function describeAudio(audio: Audio): string {
	const format = formats[audio.contentType.split(";")[0] ?? ""] ?? "Audio";
	const parts = [format, formatSize(audio.sizeBytes)];
	if (audio.durationMs !== null) {
		parts.push(formatDuration(audio.durationMs));
	}
	return parts.join(" · ");
}
