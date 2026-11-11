import { notFound } from "@tanstack/react-router";
import { z } from "zod";
import { ApiError, expectOk, send } from "@/lib/api";
import { LetterFormsSchema } from "@/lib/letterForms";

const TrackIdSchema = z.enum(["arabe", "islam"]);
export type TrackId = z.infer<typeof TrackIdSchema>;

const PlaybackSchema = z.object({
	url: z.string(),
	durationMs: z.number().nullable(),
});
export type Playback = z.infer<typeof PlaybackSchema>;

const ExerciseResultSchema = z.object({
	best: z.object({ firstTry: z.number(), total: z.number(), at: z.string() }),
	passedAt: z.string().nullable(),
	lastAttemptAt: z.string(),
});
export type ExerciseResult = z.infer<typeof ExerciseResultSchema>;

const TrackLessonSchema = z.object({
	id: z.string(),
	title: z.string(),
	cardCount: z.number(),
	hasExercise: z.boolean(),
	locked: z.boolean(),
	progress: z.object({
		seenCardCount: z.number(),
		lastSeenAt: z.string().nullable(),
		exercise: ExerciseResultSchema.nullable(),
	}),
});
export type TrackLesson = z.infer<typeof TrackLessonSchema>;

const TrackUnitSchema = z.object({
	id: z.string(),
	title: z.string(),
	lessons: z.array(TrackLessonSchema),
});
export type TrackUnit = z.infer<typeof TrackUnitSchema>;

const TrackSchema = z.object({
	id: TrackIdSchema,
	units: z.array(TrackUnitSchema),
});
export type Track = z.infer<typeof TrackSchema>;

const TrackListSchema = z.object({ items: z.array(TrackSchema) });

const StudyCardSchema = z.object({
	id: z.string(),
	arabic: z.string(),
	transliteration: z.string().nullable(),
	meaningFr: z.string().nullable(),
	variety: z.enum(["classique", "darija"]),
	forms: LetterFormsSchema.nullable(),
	audio: PlaybackSchema.nullable(),
	seen: z.boolean(),
});
export type StudyCard = z.infer<typeof StudyCardSchema>;

const StudyLessonSchema = z.object({
	id: z.string(),
	track: TrackIdSchema,
	unit: z.object({ id: z.string(), title: z.string() }),
	title: z.string(),
	audio: PlaybackSchema.nullable(),
	cards: z.array(StudyCardSchema),
	exercise: ExerciseResultSchema.nullable(),
});
export type StudyLesson = z.infer<typeof StudyLessonSchema>;

/** The track a path segment names; anything else is a page not found. */
export function parseTrackId(segment: string): TrackId {
	const parsed = TrackIdSchema.safeParse(segment);
	if (!parsed.success) {
		throw notFound();
	}
	return parsed.data;
}

const AttemptSchema = z.object({
	exercise: ExerciseResultSchema,
	nextLesson: z.object({ id: z.string(), title: z.string() }).nullable(),
});
export type Attempt = z.infer<typeof AttemptSchema>;

/** Turns the API's 404 into the router's page not found. */
async function expectFound(response: Response): Promise<void> {
	try {
		await expectOk(response);
	} catch (failure) {
		if (failure instanceof ApiError && failure.status === 404) {
			throw notFound();
		}
		throw failure;
	}
}

export async function fetchTracks(): Promise<Track[]> {
	const response = await send("GET", "/api/v1/tracks");
	await expectOk(response);
	return TrackListSchema.parse(await response.json()).items;
}

export async function fetchTrack(track: TrackId): Promise<Track> {
	const response = await send("GET", `/api/v1/tracks/${track}`);
	await expectFound(response);
	return TrackSchema.parse(await response.json());
}

export async function fetchLesson(
	track: TrackId,
	lessonId: string,
): Promise<StudyLesson> {
	const response = await send(
		"GET",
		`/api/v1/tracks/${track}/lessons/${encodeURIComponent(lessonId)}`,
	);
	await expectFound(response);
	return StudyLessonSchema.parse(await response.json());
}

export async function sendCardView(cardId: string): Promise<void> {
	const response = await send(
		"PUT",
		`/api/v1/card-views/${encodeURIComponent(cardId)}`,
	);
	await expectOk(response);
}

export async function sendAttempt(
	lessonId: string,
	firstTry: number,
	total: number,
): Promise<Attempt> {
	const response = await send("POST", "/api/v1/exercise-attempts", {
		lessonId,
		firstTry,
		total,
	});
	await expectOk(response);
	return AttemptSchema.parse(await response.json());
}

/** Whether every card of the lesson has been seen. */
export function isComplete(lesson: TrackLesson): boolean {
	return (
		lesson.cardCount > 0 && lesson.progress.seenCardCount >= lesson.cardCount
	);
}

export function isPassed(exercise: ExerciseResult | null): boolean {
	return exercise?.passedAt != null;
}

export function lessonCount(track: Track): number {
	return track.units.reduce((sum, unit) => sum + unit.lessons.length, 0);
}
