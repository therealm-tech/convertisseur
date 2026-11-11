import { logger } from "@/lib/logger";

const PROBE_TIMEOUT_MS = 3_000;

/**
 * The length of an audio file in milliseconds, as the browser decodes it, or
 * `null` when it cannot tell: no metadata in time, or the `Infinity` Chrome
 * reports for a `MediaRecorder` WebM, which has no length in its header.
 */
export function readDuration(blob: Blob): Promise<number | null> {
	return new Promise((resolve) => {
		const url = URL.createObjectURL(blob);
		const element = new Audio();
		const settle = (ms: number | null) => {
			clearTimeout(timer);
			element.removeAttribute("src");
			URL.revokeObjectURL(url);
			logger.debug("media_duration.read", { ms });
			resolve(ms);
		};
		const timer = setTimeout(() => settle(null), PROBE_TIMEOUT_MS);
		element.preload = "metadata";
		element.addEventListener(
			"loadedmetadata",
			() => {
				const seconds = element.duration;
				settle(
					Number.isFinite(seconds) && seconds > 0
						? Math.round(seconds * 1000)
						: null,
				);
			},
			{ once: true },
		);
		element.addEventListener("error", () => settle(null), { once: true });
		element.src = url;
	});
}
