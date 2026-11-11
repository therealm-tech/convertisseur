import { logger } from "@/lib/logger";

/** Why the device cannot record here, or `null` when it can try. */
export type RecordingObstacle = "insecure" | "unsupported";

/** What a finished recording yields. */
export interface Recorded {
	blob: Blob;
	/** Wall-clock time between the start and the stop. */
	elapsedMs: number;
}

/** A recording under way. */
export interface Recording {
	/** Stops it and gives back what was recorded, and for how long. */
	stop(): Promise<Recorded>;
	/** Stops it and drops what was recorded. */
	cancel(): void;
}

/** Why the microphone could not be opened. */
export type MicrophoneFailure = "denied" | "missing" | "failed";

export class MicrophoneError extends Error {
	readonly reason: MicrophoneFailure;

	constructor(reason: MicrophoneFailure) {
		super(`microphone unavailable: ${reason}`);
		this.name = "MicrophoneError";
		this.reason = reason;
	}
}

// What each browser records natively: Chrome WebM/Opus, Firefox Ogg/Opus
// (and WebM), Safari MP4/AAC. The server stores it as is.
const preferredTypes = [
	"audio/webm;codecs=opus",
	"audio/ogg;codecs=opus",
	"audio/mp4",
];

/**
 * Browsers expose the microphone to secure contexts only: HTTPS, or
 * `localhost` in development.
 */
export function recordingObstacle(): RecordingObstacle | null {
	if (!window.isSecureContext) {
		return "insecure";
	}
	if (
		typeof MediaRecorder === "undefined" ||
		!navigator.mediaDevices?.getUserMedia
	) {
		return "unsupported";
	}
	return null;
}

function failureOf(error: unknown): MicrophoneFailure {
	if (error instanceof DOMException) {
		if (error.name === "NotAllowedError" || error.name === "SecurityError") {
			return "denied";
		}
		if (
			error.name === "NotFoundError" ||
			error.name === "OverconstrainedError"
		) {
			return "missing";
		}
	}
	return "failed";
}

/** The file extension for a recorded MIME type. */
export function extensionOf(type: string): string {
	if (type.startsWith("audio/mp4")) {
		return "m4a";
	}
	if (type.startsWith("audio/ogg")) {
		return "ogg";
	}
	return "webm";
}

/** Asks for the microphone, and records until stopped. */
export async function startRecording(): Promise<Recording> {
	let stream: MediaStream;
	try {
		stream = await navigator.mediaDevices.getUserMedia({ audio: true });
	} catch (error) {
		logger.warn("recorder.microphone_refused", { error });
		throw new MicrophoneError(failureOf(error));
	}
	const mimeType = preferredTypes.find((type) =>
		MediaRecorder.isTypeSupported(type),
	);
	const recorder = new MediaRecorder(stream, mimeType ? { mimeType } : {});
	const chunks: Blob[] = [];
	recorder.addEventListener("dataavailable", (event) => {
		if (event.data.size > 0) {
			chunks.push(event.data);
		}
	});
	const release = () => {
		for (const track of stream.getTracks()) {
			track.stop();
		}
	};
	recorder.start();
	const startedAt = performance.now();
	logger.debug("recorder.started", { mimeType: recorder.mimeType });
	return {
		stop: () =>
			new Promise<Recorded>((resolve) => {
				recorder.addEventListener(
					"stop",
					() => {
						release();
						const type = recorder.mimeType || mimeType || "audio/webm";
						const blob = new Blob(chunks, { type });
						const elapsedMs = Math.round(performance.now() - startedAt);
						logger.debug("recorder.stopped", {
							type,
							size: blob.size,
							elapsedMs,
						});
						resolve({ blob, elapsedMs });
					},
					{ once: true },
				);
				recorder.stop();
			}),
		cancel: () => {
			if (recorder.state !== "inactive") {
				recorder.stop();
			}
			release();
		},
	};
}
