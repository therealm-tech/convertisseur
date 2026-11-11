import { ApiError } from "@/lib/api";
import { logger } from "@/lib/logger";
import {
	type Attempt,
	isComplete,
	isPassed,
	sendAttempt,
	sendCardView,
	type Track,
	type TrackId,
	type TrackLesson,
} from "@/lib/tracks";

/**
 * Writes the server has not taken yet, kept in memory and sent again with
 * the next write or when the browser is back online; closing the page loses
 * them.
 */
const pending = new Map<string, () => Promise<unknown>>();
let flushing = false;

/** A refusal the server will repeat: sending it again is pointless. */
function refused(error: unknown): boolean {
	return error instanceof ApiError && error.status >= 400 && error.status < 500;
}

async function flush(): Promise<void> {
	if (flushing) {
		return;
	}
	flushing = true;
	try {
		for (;;) {
			const next = pending.entries().next();
			if (next.done) {
				return;
			}
			const [key, write] = next.value;
			try {
				await write();
				pending.delete(key);
			} catch (error) {
				if (!refused(error)) {
					logger.debug("progress.deferred", { key, error });
					return;
				}
				pending.delete(key);
				logger.warn("progress.refused", { key, error });
			}
		}
	} finally {
		flushing = false;
	}
}

if (typeof window !== "undefined") {
	window.addEventListener("online", () => {
		void flush();
	});
}

/** Records that the account saw the card, without waiting for the server. */
export function recordCardView(cardId: string): void {
	pending.set(`view:${cardId}`, () => sendCardView(cardId));
	void flush();
}

/**
 * Counts an exercise attempt; `null` when the server could not be reached,
 * in which case the attempt is sent again later.
 */
export async function recordAttempt(
	lessonId: string,
	firstTry: number,
	total: number,
): Promise<Attempt | null> {
	try {
		const attempt = await sendAttempt(lessonId, firstTry, total);
		void flush();
		return attempt;
	} catch (error) {
		logger.warn("progress.attempt_failed", { lessonId, error });
		if (!refused(error)) {
			pending.set(`attempt:${lessonId}:${Date.now()}`, () =>
				sendAttempt(lessonId, firstTry, total),
			);
		}
		return null;
	}
}

/** A lesson with its place on its track. */
export interface PathLesson {
	track: TrackId;
	unitNumber: number;
	lessonNumber: number;
	lesson: TrackLesson;
}

/**
 * `start`: nothing done yet. `resume`: the last lesson touched has cards
 * left. `exercise`: its cards are seen and its exercise not passed.
 * `next`: the next lesson to open after it.
 */
export type StepKind = "start" | "resume" | "exercise" | "next";

export interface NextStep extends PathLesson {
	kind: StepKind;
}

function path(track: Track): PathLesson[] {
	return track.units.flatMap((unit, unitIndex) =>
		unit.lessons.map((lesson, lessonIndex) => ({
			track: track.id,
			unitNumber: unitIndex + 1,
			lessonNumber: lessonIndex + 1,
			lesson,
		})),
	);
}

/** When the learner last saw a card of the lesson or tried its exercise. */
function lastActivity(lesson: TrackLesson): number | null {
	const times = [
		lesson.progress.lastSeenAt,
		lesson.progress.exercise?.lastAttemptAt ?? null,
	].flatMap((time) => (time === null ? [] : [Date.parse(time)]));
	return times.length > 0 ? Math.max(...times) : null;
}

function studiable(step: PathLesson): boolean {
	return !step.lesson.locked && step.lesson.cardCount > 0;
}

/** What the learner should do next, from the last lesson it touched. */
export function nextStep(tracks: readonly Track[]): NextStep | null {
	const steps = tracks.flatMap(path);
	const latest = steps
		.filter((step) => lastActivity(step.lesson) !== null)
		.sort(
			(a, b) => (lastActivity(b.lesson) ?? 0) - (lastActivity(a.lesson) ?? 0),
		)[0];
	if (!latest) {
		const first = steps.find(studiable);
		return first ? { ...first, kind: "start" } : null;
	}
	if (!isComplete(latest.lesson)) {
		return { ...latest, kind: "resume" };
	}
	if (latest.lesson.hasExercise && !isPassed(latest.lesson.progress.exercise)) {
		return { ...latest, kind: "exercise" };
	}
	const track = steps.filter((step) => step.track === latest.track);
	const after = track
		.slice(track.indexOf(latest) + 1)
		.find((step) => studiable(step) && !isComplete(step.lesson));
	return after ? { ...after, kind: "next" } : null;
}

/** How many lessons of the track have all their cards seen. */
export function completedCount(track: Track): number {
	return path(track).filter((step) => isComplete(step.lesson)).length;
}

/** Whether the learner has opened anything on the track. */
export function isStarted(track: Track): boolean {
	return path(track).some((step) => lastActivity(step.lesson) !== null);
}
