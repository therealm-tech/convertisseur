import { type ReactNode, useState } from "react";
import type { Status } from "@/lib/content";
import { statusLabels } from "@/lib/contentLabels";
import { type ContentSearch, statusParam } from "@/lib/contentRoutes";
import type { Counts, Filters } from "@/lib/contentTree";
import { NoteIcon } from "@/pages/admin/contentParts";

interface ContentFiltersProps {
	filters: Filters;
	/** Unknown while loading; "Sans audio" shows only when counted. */
	counts: Partial<Counts> | undefined;
	onFilter: (change: Partial<ContentSearch>) => void;
	/** Kept in the folded panel on a phone, beside note and audio. */
	extra?: ReactNode;
	/** Always in view, after the filters. */
	children?: ReactNode;
}

function label(text: string, count: number | undefined) {
	return count === undefined ? text : `${text} · ${count}`;
}

/**
 * The status, note and audio filters. On a phone, all but the status fold
 * behind a button, which says how many of them are on.
 */
export function ContentFilters({
	filters,
	counts,
	onFilter,
	extra,
	children,
}: ContentFiltersProps) {
	const [open, setOpen] = useState(false);
	const active =
		Number(filters.note) +
		Number(filters.audio) +
		Number(filters.track !== undefined);
	const statuses: {
		value: Status | undefined;
		text: string;
		count?: number;
	}[] = [
		{ value: undefined, text: "Tout", ...withCount(counts?.all) },
		{
			value: "to_review",
			text: statusLabels.to_review,
			...withCount(counts?.toReview),
		},
		{
			value: "validated",
			text: statusLabels.validated,
			...withCount(counts?.validated),
		},
	];
	return (
		<div className="admin-filters content-filters">
			<fieldset className="status-filter">
				<legend className="visually-hidden">Filtrer par statut</legend>
				{statuses.map((option) => (
					<button
						key={option.text}
						type="button"
						aria-pressed={filters.status === option.value}
						onClick={() => onFilter({ status: statusParam(option.value) })}
					>
						{label(option.text, option.count)}
					</button>
				))}
			</fieldset>
			<button
				type="button"
				className="content-filters-toggle"
				aria-expanded={open}
				onClick={() => setOpen(!open)}
			>
				{active > 0 ? `Filtres · ${active}` : "Filtres"}
			</button>
			<div className="content-filters-more" data-open={open}>
				<button
					type="button"
					className="note-filter"
					aria-pressed={filters.note}
					onClick={() => onFilter({ note: filters.note ? undefined : true })}
				>
					<NoteIcon />
					{label("Avec remarque", counts?.noted)}
				</button>
				{counts?.withoutAudio === undefined && !filters.audio ? null : (
					<button
						type="button"
						className="note-filter"
						aria-pressed={filters.audio}
						onClick={() =>
							onFilter({ audio: filters.audio ? undefined : "missing" })
						}
					>
						<NoAudioIcon />
						{label("Sans audio", counts?.withoutAudio)}
					</button>
				)}
				{extra}
			</div>
			{children}
		</div>
	);
}

function withCount(count: number | undefined): { count?: number } {
	return count === undefined ? {} : { count };
}

function NoAudioIcon() {
	return (
		<svg
			width="16"
			height="16"
			viewBox="0 0 24 24"
			fill="none"
			stroke="currentColor"
			strokeWidth="2.2"
			strokeLinecap="round"
			strokeLinejoin="round"
			aria-hidden="true"
		>
			<path d="M11 5 6 9H3v6h3l5 4z" />
			<path d="m16 9 5 6M21 9l-5 6" />
		</svg>
	);
}
