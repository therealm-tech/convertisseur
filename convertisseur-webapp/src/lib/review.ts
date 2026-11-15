import { z } from "zod";
import { allPages, expectOk, MAX_PER_PAGE, send } from "@/lib/api";
import {
	type Card,
	CardSchema,
	type Status,
	StatusSchema,
	TrackSchema,
} from "@/lib/content";

const timestamp = z.iso.datetime({ offset: true });

const LessonSummarySchema = z.object({
	id: z.string(),
	position: z.number(),
	title: z.string(),
	status: StatusSchema,
	reviewNote: z.string().nullable(),
	hasAudio: z.boolean(),
	/** The status of the lesson's own recording; `null` when it has none. */
	audioStatus: StatusSchema.nullable(),
	unit: z.object({
		id: z.string(),
		track: TrackSchema,
		position: z.number(),
		title: z.string(),
		status: StatusSchema,
		reviewNote: z.string().nullable(),
	}),
	cardCounts: z.object({
		validated: z.number(),
		flagged: z.number(),
		toReview: z.number(),
		withoutAudio: z.number(),
		/** The cards' recordings, reviewed apart from the cards' text. */
		recordings: z.object({
			validated: z.number(),
			flagged: z.number(),
			toReview: z.number(),
		}),
	}),
});
export type LessonSummary = z.infer<typeof LessonSummarySchema>;

const LessonSummaryPageSchema = z.object({
	items: z.array(LessonSummarySchema),
	page: z.number(),
	perPage: z.number(),
	total: z.number(),
	itemsToReview: z.number(),
	recordingsToReview: z.number(),
	cardsWithoutAudio: z.number(),
});
export type LessonSummaryPage = z.infer<typeof LessonSummaryPageSchema>;

const CardPageSchema = z.object({
	items: z.array(CardSchema),
	page: z.number(),
	perPage: z.number(),
	total: z.number(),
});

const CardValidationsSchema = z.object({
	cards: z.array(z.object({ id: z.string(), status: StatusSchema })),
});

const RecordingReviewSchema = z.object({
	cardId: z.string(),
	id: z.string(),
	status: StatusSchema,
	validatedAt: timestamp.nullable(),
	reviewNote: z.string().nullable(),
});
export type RecordingReview = z.infer<typeof RecordingReviewSchema>;

const RecordingValidationsSchema = z.object({
	recordings: z.array(RecordingReviewSchema),
});

/** Which lessons to list; a filter left out matches every lesson. */
export interface LessonFilter {
	unitId?: string;
	pendingReview?: boolean;
	missingAudio?: boolean;
	/** A recording to review, the lesson's own or a card's. */
	pendingAudio?: boolean;
}

/** What the queue's header and the admin navigation count. */
export type ReviewTotals = Pick<
	LessonSummaryPage,
	"itemsToReview" | "recordingsToReview" | "cardsWithoutAudio"
>;

function totalsOf(page: LessonSummaryPage): ReviewTotals {
	return {
		itemsToReview: page.itemsToReview,
		recordingsToReview: page.recordingsToReview,
		cardsWithoutAudio: page.cardsWithoutAudio,
	};
}

export async function listLessons(
	filter: LessonFilter,
	page = 1,
	perPage = MAX_PER_PAGE,
): Promise<LessonSummaryPage> {
	const params = new URLSearchParams({
		page: String(page),
		perPage: String(perPage),
	});
	if (filter.unitId) {
		params.set("unitId", filter.unitId);
	}
	if (filter.pendingReview) {
		params.set("pendingReview", "true");
	}
	if (filter.missingAudio) {
		params.set("missingAudio", "true");
	}
	if (filter.pendingAudio) {
		params.set("pendingAudio", "true");
	}
	const response = await send("GET", `/api/v1/lessons?${params}`);
	await expectOk(response);
	return LessonSummaryPageSchema.parse(await response.json());
}

/** Every lesson matching `filter`, in the order of the tracks. */
export async function listAllLessons(
	filter: LessonFilter,
): Promise<{ lessons: LessonSummary[]; totals: ReviewTotals }> {
	const { first, items } = await allPages<LessonSummary, LessonSummaryPage>(
		(page) => listLessons(filter, page),
	);
	return { lessons: items, totals: totalsOf(first) };
}

/** The counts alone, for the navigation's badge. */
export async function fetchReviewTotals(): Promise<ReviewTotals> {
	return totalsOf(await listLessons({ pendingReview: true }, 1, 1));
}

/** Every card of the lesson, in order, whatever its status. */
export async function fetchLessonCards(lessonId: string): Promise<Card[]> {
	const { items } = await allPages<Card, z.infer<typeof CardPageSchema>>(
		async (page) => {
			const response = await send(
				"GET",
				`/api/v1/lessons/${encodeURIComponent(lessonId)}/cards?page=${page}&perPage=${MAX_PER_PAGE}`,
			);
			await expectOk(response);
			return CardPageSchema.parse(await response.json());
		},
	);
	return items;
}

/**
 * Validates the cards among `cardIds` that are to review without a review
 * note, at once; returns the status of each.
 */
export async function validateCards(
	lessonId: string,
	cardIds: string[],
): Promise<Map<string, Status>> {
	const response = await send(
		"POST",
		`/api/v1/lessons/${encodeURIComponent(lessonId)}/card-validations`,
		{ cardIds },
	);
	await expectOk(response);
	const { cards } = CardValidationsSchema.parse(await response.json());
	return new Map(cards.map((card) => [card.id, card.status]));
}

/**
 * Validates the recordings of the cards among `cardIds` that are to review
 * without a review note, at once; returns the review state of each card's
 * recording.
 */
export async function validateRecordings(
	lessonId: string,
	cardIds: string[],
): Promise<RecordingReview[]> {
	const response = await send(
		"POST",
		`/api/v1/lessons/${encodeURIComponent(lessonId)}/audio-validations`,
		{ cardIds },
	);
	await expectOk(response);
	return RecordingValidationsSchema.parse(await response.json()).recordings;
}

/** How a card stands in review: flagged is to review with a note. */
export type CardState = "validated" | "flagged" | "to_review";

export function cardState(card: {
	status: Status;
	reviewNote: string | null;
}): CardState {
	if (card.status === "validated") {
		return "validated";
	}
	return card.reviewNote ? "flagged" : "to_review";
}

/** Orders lessons as the tracks do: track, unit, then lesson. */
export function lessonOrder(a: LessonSummary, b: LessonSummary): number {
	return (
		a.unit.track.localeCompare(b.unit.track) ||
		a.unit.position - b.unit.position ||
		a.position - b.position
	);
}
