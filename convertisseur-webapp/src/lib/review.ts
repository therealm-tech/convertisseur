import { z } from "zod";
import { expectOk, send } from "@/lib/api";
import {
	type Card,
	CardSchema,
	type Status,
	StatusSchema,
	TrackSchema,
} from "@/lib/content";

const LessonSummarySchema = z.object({
	id: z.string(),
	position: z.number(),
	title: z.string(),
	status: StatusSchema,
	reviewNote: z.string().nullable(),
	hasAudio: z.boolean(),
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
	}),
});
export type LessonSummary = z.infer<typeof LessonSummarySchema>;

const LessonSummaryPageSchema = z.object({
	items: z.array(LessonSummarySchema),
	page: z.number(),
	perPage: z.number(),
	total: z.number(),
	itemsToReview: z.number(),
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

/** Which lessons to list; a filter left out matches every lesson. */
export interface LessonFilter {
	unitId?: string;
	pendingReview?: boolean;
	missingAudio?: boolean;
}

/** What the queue's header and the admin navigation count. */
export interface ReviewTotals {
	itemsToReview: number;
	cardsWithoutAudio: number;
}

const MAX_PER_PAGE = 100;

/** Every page of `fetchPage`, which the API caps at 100 items. */
async function allPages<T, P extends { items: T[]; total: number }>(
	fetchPage: (page: number) => Promise<P>,
): Promise<{ first: P; items: T[] }> {
	const first = await fetchPage(1);
	const items = [...first.items];
	for (let page = 2; items.length < first.total; page += 1) {
		const next = await fetchPage(page);
		if (next.items.length === 0) {
			break;
		}
		items.push(...next.items);
	}
	return { first, items };
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
	return {
		lessons: items,
		totals: {
			itemsToReview: first.itemsToReview,
			cardsWithoutAudio: first.cardsWithoutAudio,
		},
	};
}

/** The counts alone, for the navigation's badge. */
export async function fetchReviewTotals(): Promise<ReviewTotals> {
	const page = await listLessons({ pendingReview: true }, 1, 1);
	return {
		itemsToReview: page.itemsToReview,
		cardsWithoutAudio: page.cardsWithoutAudio,
	};
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
