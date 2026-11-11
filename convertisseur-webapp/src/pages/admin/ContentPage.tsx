import { Link, useNavigate, useSearch } from "@tanstack/react-router";
import { useEffect, useState } from "react";
import {
	type ContentItem,
	type Kind,
	listContent,
	type ContentPage as Page,
	type Status,
	type Track,
} from "@/lib/content";
import { kindLabels, statusLabels, trackLabels } from "@/lib/contentLabels";
import {
	type ContentSearch,
	sectionOf,
	statusOf,
	statusParam,
} from "@/lib/contentRoutes";
import { formatShortDay } from "@/lib/dates";
import {
	ItemTitle,
	NoteFlag,
	NoteIcon,
	StatusPill,
	useContentFailure,
} from "@/pages/admin/contentParts";

const PER_PAGE = 50;
const SEARCH_DELAY_MS = 250;

type StatusFilter = Status | "";
type TrackFilter = Track | "";

type Listing =
	| { state: "loading" }
	| { state: "failed" }
	| { state: "ready"; page: Page };

const trackOptions: { value: TrackFilter; label: string }[] = [
	{ value: "", label: "Tous" },
	{ value: "arabe", label: trackLabels.arabe },
	{ value: "islam", label: trackLabels.islam },
];

function useDebounced<T>(value: T, delayMs: number): T {
	const [debounced, setDebounced] = useState(value);
	useEffect(() => {
		const timer = setTimeout(() => setDebounced(value), delayMs);
		return () => clearTimeout(timer);
	}, [value, delayMs]);
	return debounced;
}

export function ContentPage() {
	const filters = useSearch({ from: "/_app/admin/content" });
	const navigate = useNavigate({ from: "/admin/content" });
	const onFailure = useContentFailure();
	const status = statusOf(filters.status);
	const track = filters.track;
	const noted = filters.note === true;
	const q = filters.q ?? "";
	const pageNumber = filters.page ?? 1;
	const [search, setSearch] = useState(q);
	const [listing, setListing] = useState<Listing>({ state: "loading" });
	const typed = useDebounced(search, SEARCH_DELAY_MS);

	// Filters live in the URL, so that going back from an edit page finds them;
	// they replace the entry rather than pile up in the history.
	function filter(change: Partial<ContentSearch>) {
		void navigate({
			search: (previous) => ({ ...previous, page: undefined, ...change }),
			replace: true,
		});
	}

	useEffect(() => {
		if (typed !== q) {
			void navigate({
				search: (previous) => ({
					...previous,
					q: typed || undefined,
					page: undefined,
				}),
				replace: true,
			});
		}
	}, [typed, q, navigate]);

	useEffect(() => {
		let current = true;
		listContent({
			page: pageNumber,
			perPage: PER_PAGE,
			status,
			track,
			q,
			hasReviewNote: noted || undefined,
		}).then(
			(page) => {
				if (current) {
					setListing({ state: "ready", page });
				}
			},
			(failure: unknown) => {
				if (current) {
					setListing({ state: "failed" });
					onFailure("admin.content_list_failed", failure);
				}
			},
		);
		return () => {
			current = false;
		};
	}, [pageNumber, status, track, noted, q, onFailure]);

	function open(item: ContentItem) {
		void navigate({
			to: "/admin/content/$section/$itemId",
			params: { section: sectionOf[item.kind], itemId: item.id },
			state: { contentList: filters },
		});
	}

	const page = listing.state === "ready" ? listing.page : undefined;
	const counts = page?.statusCounts;
	const pageCount = page
		? Math.max(1, Math.ceil(page.total / page.perPage))
		: 1;
	const statusOptions: {
		value: StatusFilter;
		label: string;
		count?: number;
	}[] = [
		{
			value: "",
			label: "Tout",
			...(counts ? { count: counts.toReview + counts.validated } : {}),
		},
		{
			value: "to_review",
			label: statusLabels.to_review,
			...(counts ? { count: counts.toReview } : {}),
		},
		{
			value: "validated",
			label: statusLabels.validated,
			...(counts ? { count: counts.validated } : {}),
		},
	];
	const createLinks: { kind: Kind; label: string; className: string }[] = [
		{ kind: "unit", label: "Nouvelle unité", className: "button-outline" },
		{ kind: "card", label: "Nouvelle carte", className: "button-outline" },
		{ kind: "lesson", label: "Nouvelle leçon", className: "button-solid" },
	];

	return (
		<div className="admin-content">
			<div className="admin-header-row">
				<header className="admin-header">
					<h1 className="admin-title">Contenu</h1>
					<p className="admin-lead">
						Unités, leçons et cartes des deux parcours. Un contenu « à relire »
						n'est pas montré aux apprenants tant qu'il n'est pas validé.
					</p>
				</header>
				<div className="admin-header-actions">
					{createLinks.map((link) => (
						<Link
							key={link.kind}
							to="/admin/content/$section/new"
							params={{ section: sectionOf[link.kind] }}
							state={{ contentList: filters }}
							className={link.className}
						>
							{link.label}
						</Link>
					))}
				</div>
			</div>

			<div className="admin-filters">
				<fieldset className="status-filter">
					<legend className="visually-hidden">Filtrer par statut</legend>
					{statusOptions.map((option) => (
						<button
							key={option.value}
							type="button"
							aria-pressed={(status ?? "") === option.value}
							onClick={() =>
								filter({ status: statusParam(option.value || undefined) })
							}
						>
							{option.label}
							{option.count === undefined ? null : ` · ${option.count}`}
						</button>
					))}
				</fieldset>
				<button
					type="button"
					className="note-filter"
					aria-pressed={noted}
					onClick={() => filter({ note: noted ? undefined : true })}
				>
					<NoteIcon />
					Avec remarque
					{page ? ` · ${page.reviewNoteCount}` : null}
				</button>
				<div className="admin-filter">
					<label htmlFor="content-parcours" className="admin-filter-label">
						Parcours
					</label>
					<select
						id="content-parcours"
						className="admin-select"
						value={track ?? ""}
						onChange={(event) => {
							const chosen = trackOptions.find(
								(option) => option.value === event.target.value,
							);
							filter({ track: chosen?.value || undefined });
						}}
					>
						{trackOptions.map((option) => (
							<option key={option.value} value={option.value}>
								{option.label}
							</option>
						))}
					</select>
				</div>
				<label htmlFor="content-recherche" className="visually-hidden">
					Rechercher du contenu
				</label>
				<input
					id="content-recherche"
					type="search"
					className="admin-search"
					placeholder="Rechercher un titre ou un mot"
					autoComplete="off"
					value={search}
					onChange={(event) => setSearch(event.target.value)}
				/>
			</div>

			<div className="admin-table-card">
				{listing.state === "failed" ? (
					<p role="alert" className="form-error admin-table-message">
						Impossible de charger le contenu. Réessaie dans un moment.
					</p>
				) : null}
				{listing.state === "loading" ? (
					<p className="admin-table-message">Chargement…</p>
				) : null}
				{page ? (
					<ContentTable items={page.items} filters={filters} onOpen={open} />
				) : null}
			</div>

			{page && pageCount > 1 ? (
				<nav aria-label="Pages" className="admin-pagination">
					<button
						type="button"
						className="button-secondary"
						disabled={pageNumber <= 1}
						onClick={() =>
							filter({ page: pageNumber > 2 ? pageNumber - 1 : undefined })
						}
					>
						Précédent
					</button>
					<span>
						Page {page.page} sur {pageCount}
					</span>
					<button
						type="button"
						className="button-secondary"
						disabled={pageNumber >= pageCount}
						onClick={() => filter({ page: pageNumber + 1 })}
					>
						Suivant
					</button>
				</nav>
			) : null}
		</div>
	);
}

interface ContentTableProps {
	items: ContentItem[];
	/** The list's filters, which the edit pages lead back to. */
	filters: ContentSearch;
	onOpen: (item: ContentItem) => void;
}

function ContentTable({ items, filters, onOpen }: ContentTableProps) {
	if (items.length === 0) {
		return <p className="admin-table-message">Aucun contenu ne correspond.</p>;
	}
	return (
		<div className="admin-table-scroll">
			<table className="admin-table content-table">
				<thead>
					<tr>
						<th scope="col">Titre</th>
						<th scope="col">Parcours</th>
						<th scope="col">Type</th>
						<th scope="col">Relecture</th>
						<th scope="col">Modifié</th>
					</tr>
				</thead>
				<tbody>
					{items.map((item) => (
						<tr
							key={item.id}
							className="content-row"
							onClick={(event) => {
								const target = event.target;
								const onLink =
									target instanceof Element && target.closest("a") !== null;
								const selecting = Boolean(window.getSelection()?.toString());
								if (!onLink && !selecting) {
									onOpen(item);
								}
							}}
						>
							<th scope="row">
								<Link
									to="/admin/content/$section/$itemId"
									params={{ section: sectionOf[item.kind], itemId: item.id }}
									state={{ contentList: filters }}
									className="content-title-link"
								>
									<span>
										<ItemTitle
											title={item.title}
											transliteration={item.transliteration}
											arabic={item.kind === "card"}
										/>
									</span>
								</Link>
								{item.parentTitle ? (
									<span className="content-parent">{item.parentTitle}</span>
								) : null}
								{item.reviewNote ? <NoteFlag /> : null}
							</th>
							<td>{trackLabels[item.track]}</td>
							<td>{kindLabels[item.kind]}</td>
							<td>
								<StatusPill status={item.status} />
							</td>
							<td className="admin-cell-muted">
								{formatShortDay(item.updatedAt)}
							</td>
						</tr>
					))}
				</tbody>
			</table>
		</div>
	);
}
