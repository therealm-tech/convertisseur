import { notFound } from "@tanstack/react-router";
import { z } from "zod";
import { type Kind, type Status, TrackSchema } from "@/lib/content";

const StatusParamSchema = z.enum(["a-relire", "valide"]);
type StatusParam = z.infer<typeof StatusParamSchema>;

/** The filters of the content list, kept in its URL. */
export const ContentSearchSchema = z.object({
	statut: StatusParamSchema.optional().catch(undefined),
	parcours: TrackSchema.optional().catch(undefined),
	remarque: z.literal(true).optional().catch(undefined),
	q: z.string().min(1).optional().catch(undefined),
	page: z.number().int().min(2).optional().catch(undefined),
});
export type ContentSearch = z.infer<typeof ContentSearchSchema>;

export function parseContentSearch(
	search: Record<string, unknown>,
): ContentSearch {
	return ContentSearchSchema.parse(search);
}

const statusOfParam: Record<StatusParam, Status> = {
	"a-relire": "to_review",
	valide: "validated",
};

const paramOfStatus: Record<Status, StatusParam> = {
	to_review: "a-relire",
	validated: "valide",
};

export function statusOf(param: StatusParam | undefined): Status | undefined {
	return param && statusOfParam[param];
}

export function statusParam(status: Status | undefined) {
	return status && paramOfStatus[status];
}

const SectionSchema = z.enum(["unites", "lecons", "cartes"]);
export type Section = z.infer<typeof SectionSchema>;

const kindOfSection: Record<Section, Kind> = {
	unites: "unit",
	lecons: "lesson",
	cartes: "card",
};

/** The path segment under `/admin/contenu` of each kind's pages. */
export const sectionOf: Record<Kind, Section> = {
	unit: "unites",
	lesson: "lecons",
	card: "cartes",
};

export function parseSection(segment: string): Kind {
	const parsed = SectionSchema.safeParse(segment);
	if (!parsed.success) {
		throw notFound();
	}
	return kindOfSection[parsed.data];
}

declare module "@tanstack/react-router" {
	interface HistoryState {
		/** Set when an edit page is opened from the list, with its filters. */
		contentList?: ContentSearch;
	}
}

/** The list's filters an edit page was opened from, if it was. */
export function listOrigin(state: unknown): ContentSearch | undefined {
	const parsed = z
		.object({ contentList: ContentSearchSchema })
		.safeParse(state);
	return parsed.success ? parsed.data.contentList : undefined;
}
