import type { Card, ContentItem, Status, Track } from "@/lib/content";
import { type LessonSummary, lessonOrder } from "@/lib/review";

/** The filters the tree, a lesson's cards and their counts apply. */
export interface Filters {
	status: Status | undefined;
	note: boolean;
	audio: boolean;
	track: Track | undefined;
	unit: string | undefined;
}

/** Everything the tree is drawn from, loaded once. */
export interface TreeData {
	lessons: LessonSummary[];
	units: ContentItem[];
	/** The units, lessons and cards with a review note. */
	noted: ContentItem[];
}

export interface UnitNode {
	unit: ContentItem;
	/** How many lessons the unit holds, whatever the filters. */
	lessonCount: number;
	/** Its lessons the filters keep, in order. */
	lessons: LessonSummary[];
}

export interface TrackNode {
	track: Track;
	units: UnitNode[];
}

/** How many items each filter would show, the others applied. */
export interface Counts {
	all: number;
	toReview: number;
	validated: number;
	noted: number;
	withoutAudio: number;
}

const tracks: Track[] = ["arabe", "islam"];

/** The lesson, its unit or one of its cards waits for a reviewer. */
export function needsReview(lesson: LessonSummary): boolean {
	const cards = lesson.cardCounts;
	return (
		lesson.status === "to_review" ||
		lesson.unit.status === "to_review" ||
		cards.toReview + cards.flagged > 0
	);
}

function filtering(filters: Filters): boolean {
	return filters.status !== undefined || filters.note || filters.audio;
}

function inScope(
	filters: Filters,
	track: Track,
	unitId: string | null,
): boolean {
	return (
		(filters.track === undefined || filters.track === track) &&
		(filters.unit === undefined || filters.unit === unitId)
	);
}

/** The lessons holding a note, on the lesson itself or on one of its cards. */
function notedLessons(data: TreeData): Set<string> {
	const ids = new Set<string>();
	for (const item of data.noted) {
		if (item.kind === "lesson") {
			ids.add(item.id);
		} else if (item.lessonId) {
			ids.add(item.lessonId);
		}
	}
	return ids;
}

function lessonMatcher(data: TreeData, filters: Filters) {
	const noted = notedLessons(data);
	return (lesson: LessonSummary) =>
		inScope(filters, lesson.unit.track, lesson.unit.id) &&
		(filters.status === undefined ||
			needsReview(lesson) === (filters.status === "to_review")) &&
		(!filters.note || noted.has(lesson.id)) &&
		(!filters.audio || lesson.cardCounts.withoutAudio > 0);
}

/**
 * A unit with no lesson kept still shows when nothing is filtered, or when
 * it is itself to review or noted as the filters ask.
 */
function unitShows(unit: ContentItem, filters: Filters): boolean {
	if (!filtering(filters)) {
		return true;
	}
	if (filters.audio || filters.status === "validated") {
		return false;
	}
	return (
		(filters.status === undefined || unit.status === filters.status) &&
		(!filters.note || unit.reviewNote !== null)
	);
}

/** The tracks, their units and their lessons the filters keep, in order. */
export function buildTree(data: TreeData, filters: Filters): TrackNode[] {
	const matches = lessonMatcher(data, filters);
	const ordered = [...data.lessons].sort(lessonOrder);
	const positions = new Map(
		ordered.map((lesson) => [lesson.unit.id, lesson.unit.position]),
	);
	const position = (unit: ContentItem) =>
		positions.get(unit.id) ?? Number.POSITIVE_INFINITY;
	return tracks
		.map((track) => ({
			track,
			units: data.units
				.filter(
					(unit) => unit.track === track && inScope(filters, track, unit.id),
				)
				.sort(
					(a, b) => position(a) - position(b) || a.title.localeCompare(b.title),
				)
				.map((unit) => {
					const all = ordered.filter((lesson) => lesson.unit.id === unit.id);
					return {
						unit,
						lessonCount: all.length,
						lessons: all.filter(matches),
					};
				})
				.filter(
					(node) => node.lessons.length > 0 || unitShows(node.unit, filters),
				),
		}))
		.filter((node) => node.units.length > 0);
}

/** The lessons before and after `lessonId` among those the filters keep. */
export function neighbours(
	data: TreeData,
	filters: Filters,
	lessonId: string,
): { previous: LessonSummary | undefined; next: LessonSummary | undefined } {
	const matches = lessonMatcher(data, filters);
	const kept = [...data.lessons]
		.sort(lessonOrder)
		.filter((lesson) => lesson.id === lessonId || matches(lesson));
	const index = kept.findIndex((lesson) => lesson.id === lessonId);
	if (index < 0) {
		return { previous: undefined, next: undefined };
	}
	return { previous: kept[index - 1], next: kept[index + 1] };
}

/**
 * The units, lessons and cards in the tree's scope, per status under the
 * note filter, and the noted ones under the status filter. Cards without
 * audio are counted whatever the status: the lesson summaries do not split
 * them by status.
 */
export function treeCounts(data: TreeData, filters: Filters): Counts {
	const units = data.units.filter((unit) =>
		inScope(filters, unit.track, unit.id),
	);
	const lessons = data.lessons.filter((lesson) =>
		inScope(filters, lesson.unit.track, lesson.unit.id),
	);
	const noted = data.noted.filter((item) =>
		inScope(filters, item.track, item.kind === "unit" ? item.id : item.unitId),
	);
	let all: number;
	let toReview: number;
	if (filters.note) {
		all = noted.length;
		toReview = noted.filter((item) => item.status === "to_review").length;
	} else {
		all = units.length + lessons.length;
		toReview =
			units.filter((unit) => unit.status === "to_review").length +
			lessons.filter((lesson) => lesson.status === "to_review").length;
		for (const lesson of lessons) {
			const cards = lesson.cardCounts;
			all += cards.validated + cards.toReview + cards.flagged;
			toReview += cards.toReview + cards.flagged;
		}
	}
	return {
		all,
		toReview,
		validated: all - toReview,
		noted: noted.filter(
			(item) => filters.status === undefined || item.status === filters.status,
		).length,
		withoutAudio: lessons.reduce(
			(sum, lesson) => sum + lesson.cardCounts.withoutAudio,
			0,
		),
	};
}

type CardFilter = "status" | "note" | "audio";

function cardMatches(card: Card, filters: Filters, skip?: CardFilter) {
	return (
		(skip === "status" ||
			filters.status === undefined ||
			card.status === filters.status) &&
		(skip === "note" || !filters.note || card.reviewNote !== null) &&
		(skip === "audio" || !filters.audio || card.audio === null)
	);
}

/** The cards of a lesson the filters keep. */
export function filterCards(cards: Card[], filters: Filters): Card[] {
	return cards.filter((card) => cardMatches(card, filters));
}

/** A lesson's cards per filter, each counted under the other filters. */
export function cardCounts(cards: Card[], filters: Filters): Counts {
	const under = (skip: CardFilter) =>
		cards.filter((card) => cardMatches(card, filters, skip));
	const byStatus = under("status");
	const toReview = byStatus.filter(
		(card) => card.status === "to_review",
	).length;
	return {
		all: byStatus.length,
		toReview,
		validated: byStatus.length - toReview,
		noted: under("note").filter((card) => card.reviewNote !== null).length,
		withoutAudio: under("audio").filter((card) => card.audio === null).length,
	};
}
