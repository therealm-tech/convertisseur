import { Link } from "@tanstack/react-router";
import { Fragment } from "react";
import type { ContentSearch } from "@/lib/contentRoutes";

export interface Crumb {
	label: string;
	/** Where it leads, on top of the filters kept from the page. */
	search: Partial<ContentSearch>;
}

/** The filters a breadcrumb keeps: what is filtered, not where one is. */
export function keptFilters(search: ContentSearch): ContentSearch {
	return {
		...(search.status ? { status: search.status } : {}),
		...(search.note ? { note: search.note } : {}),
		...(search.audio ? { audio: search.audio } : {}),
	};
}

/** "Contenu › Arabe › L'alphabet", each step a link up the tree. */
export function ContentCrumbs({
	search,
	trail,
}: {
	search: ContentSearch;
	trail: Crumb[];
}) {
	const kept = keptFilters(search);
	return (
		<nav aria-label="Fil d'Ariane" className="review-crumb content-crumbs">
			<Link to="/admin/content" search={kept}>
				Contenu
			</Link>
			{trail.map((crumb) => (
				<Fragment key={crumb.label}>
					<span aria-hidden="true">›</span>
					<Link to="/admin/content" search={{ ...kept, ...crumb.search }}>
						{crumb.label}
					</Link>
				</Fragment>
			))}
		</nav>
	);
}

export function ChevronIcon() {
	return (
		<svg
			width="20"
			height="20"
			viewBox="0 0 24 24"
			fill="none"
			stroke="currentColor"
			strokeWidth="2.2"
			strokeLinecap="round"
			strokeLinejoin="round"
			aria-hidden="true"
		>
			<path d="m9 5 7 7-7 7" />
		</svg>
	);
}
