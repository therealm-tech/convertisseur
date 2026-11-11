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

const TrackLessonSchema = z.object({
	id: z.string(),
	title: z.string(),
	cardCount: z.number(),
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
});
export type StudyCard = z.infer<typeof StudyCardSchema>;

const StudyLessonSchema = z.object({
	id: z.string(),
	track: TrackIdSchema,
	unit: z.object({ id: z.string(), title: z.string() }),
	title: z.string(),
	audio: PlaybackSchema.nullable(),
	cards: z.array(StudyCardSchema),
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

export function lessonCount(track: Track): number {
	return track.units.reduce((sum, unit) => sum + unit.lessons.length, 0);
}
