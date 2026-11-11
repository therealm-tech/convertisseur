import { z } from "zod";
import { expectOk, send, sendForm } from "@/lib/api";

const TrackSchema = z.enum(["arabe", "islam"]);
export type Track = z.infer<typeof TrackSchema>;

const StatusSchema = z.enum(["to_review", "validated"]);
export type Status = z.infer<typeof StatusSchema>;

const VarietySchema = z.enum(["classique", "darija"]);
export type Variety = z.infer<typeof VarietySchema>;

const KindSchema = z.enum(["unit", "lesson", "card"]);
export type Kind = z.infer<typeof KindSchema>;

const timestamp = z.iso.datetime({ offset: true });

const AuthorSchema = z.object({ id: z.string(), username: z.string() });
export type Author = z.infer<typeof AuthorSchema>;

const ReviewSchema = z.object({
	status: StatusSchema,
	createdAt: timestamp,
	createdBy: AuthorSchema.nullable(),
	updatedAt: timestamp,
	updatedBy: AuthorSchema.nullable(),
	validatedAt: timestamp.nullable(),
	validatedBy: AuthorSchema.nullable(),
});
export type Review = z.infer<typeof ReviewSchema>;

const UnitSchema = ReviewSchema.extend({
	id: z.string(),
	track: TrackSchema,
	position: z.number(),
	title: z.string(),
});
export type Unit = z.infer<typeof UnitSchema>;

const AudioSchema = z.object({
	id: z.string(),
	contentType: z.string(),
	sizeBytes: z.number(),
	durationMs: z.number().nullable(),
	createdAt: timestamp,
});
export type Audio = z.infer<typeof AudioSchema>;

const LessonSchema = ReviewSchema.extend({
	id: z.string(),
	unit: z.object({ id: z.string(), position: z.number(), title: z.string() }),
	track: TrackSchema,
	position: z.number(),
	title: z.string(),
	audio: AudioSchema.nullable(),
});
export type Lesson = z.infer<typeof LessonSchema>;

const CardSchema = ReviewSchema.extend({
	id: z.string(),
	lesson: z.object({ id: z.string(), title: z.string() }),
	track: TrackSchema,
	position: z.number(),
	arabic: z.string(),
	transliteration: z.string().nullable(),
	meaningFr: z.string().nullable(),
	variety: VarietySchema,
	audio: AudioSchema.nullable(),
});
export type Card = z.infer<typeof CardSchema>;

const ContentItemSchema = z.object({
	kind: KindSchema,
	id: z.string(),
	track: TrackSchema,
	title: z.string(),
	transliteration: z.string().nullable(),
	parentTitle: z.string().nullable(),
	status: StatusSchema,
	updatedAt: timestamp,
});
export type ContentItem = z.infer<typeof ContentItemSchema>;

const ContentPageSchema = z.object({
	items: z.array(ContentItemSchema),
	page: z.number(),
	perPage: z.number(),
	total: z.number(),
	statusCounts: z.object({ toReview: z.number(), validated: z.number() }),
});
export type ContentPage = z.infer<typeof ContentPageSchema>;

const StatusChangeSchema = z.object({
	status: StatusSchema,
	validatedAt: timestamp.nullable(),
	validatedBy: AuthorSchema.nullable(),
});
export type StatusChange = z.infer<typeof StatusChangeSchema>;

export interface ContentQuery {
	page: number;
	perPage: number;
	status?: Status | undefined;
	track?: Track | undefined;
	kind?: Kind | undefined;
	q?: string | undefined;
}

export interface UnitFields {
	track: Track;
	title: string;
}

export interface LessonFields {
	unitId: string;
	title: string;
}

export interface CardFields {
	lessonId: string;
	arabic: string;
	transliteration: string;
	meaningFr: string;
	variety: Variety;
}

/** The fields of a kind, as its forms edit them. */
export interface FieldsOf {
	unit: UnitFields;
	lesson: LessonFields;
	card: CardFields;
}

/** An item of a kind, as the API answers it. */
export interface ItemOf {
	unit: Unit;
	lesson: Lesson;
	card: Card;
}

const collections: Record<Kind, string> = {
	unit: "/api/v1/units",
	lesson: "/api/v1/lessons",
	card: "/api/v1/cards",
};

const schemas = {
	unit: UnitSchema,
	lesson: LessonSchema,
	card: CardSchema,
} as const;

function parseItem<K extends Kind>(kind: K, body: unknown): ItemOf[K] {
	// The schema picked by `kind` infers exactly ItemOf[K]; TypeScript cannot
	// follow K through the lookup.
	return schemas[kind].parse(body) as ItemOf[K];
}

function itemPath(kind: Kind, id: string): string {
	return `${collections[kind]}/${encodeURIComponent(id)}`;
}

export async function listContent(query: ContentQuery): Promise<ContentPage> {
	const params = new URLSearchParams({
		page: String(query.page),
		perPage: String(query.perPage),
	});
	for (const key of ["status", "track", "kind"] as const) {
		const value = query[key];
		if (value) {
			params.set(key, value);
		}
	}
	const q = query.q?.trim();
	if (q) {
		params.set("q", q);
	}
	const response = await send("GET", `/api/v1/content-items?${params}`);
	await expectOk(response);
	return ContentPageSchema.parse(await response.json());
}

export async function getItem<K extends Kind>(
	kind: K,
	id: string,
): Promise<ItemOf[K]> {
	const response = await send("GET", itemPath(kind, id));
	await expectOk(response);
	return parseItem(kind, await response.json());
}

export async function createItem<K extends Kind>(
	kind: K,
	fields: FieldsOf[K],
): Promise<ItemOf[K]> {
	const response = await send("POST", collections[kind], fields);
	await expectOk(response);
	return parseItem(kind, await response.json());
}

export async function updateItem<K extends Kind>(
	kind: K,
	id: string,
	fields: FieldsOf[K] & { position: number },
): Promise<ItemOf[K]> {
	const response = await send("PUT", itemPath(kind, id), fields);
	await expectOk(response);
	return parseItem(kind, await response.json());
}

export async function setStatus(
	kind: Kind,
	id: string,
	status: Status,
): Promise<StatusChange> {
	const response = await send("PUT", `${itemPath(kind, id)}/status`, {
		status,
	});
	await expectOk(response);
	return StatusChangeSchema.parse(await response.json());
}

/** The address an `<audio>` element plays a recording from. */
export function audioUrl(audioId: string): string {
	return `/api/v1/audio/${encodeURIComponent(audioId)}`;
}

/** Uploads a recording for a card or a lesson, in place of its own. */
export async function uploadAudio(
	kind: "card" | "lesson",
	id: string,
	file: Blob,
	filename: string,
): Promise<Audio> {
	const body = new FormData();
	body.append("file", file, filename);
	const response = await sendForm("PUT", `${itemPath(kind, id)}/audio`, body);
	await expectOk(response);
	return AudioSchema.parse(await response.json());
}

export async function deleteAudio(
	kind: "card" | "lesson",
	id: string,
): Promise<void> {
	await expectOk(await send("DELETE", `${itemPath(kind, id)}/audio`));
}

export async function deleteItem(kind: Kind, id: string): Promise<void> {
	await expectOk(await send("DELETE", itemPath(kind, id)));
}
