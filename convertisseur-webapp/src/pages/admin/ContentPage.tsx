import { Link, useNavigate, useSearch } from "@tanstack/react-router";
import { useEffect, useState } from "react";
import {
	type ContentItem,
	type Kind,
	listAllContent,
	listContent,
	type ContentPage as Page,
	type Track,
} from "@/lib/content";
import { kindLabels, trackLabels } from "@/lib/contentLabels";
import { type ContentSearch, sectionOf, statusOf } from "@/lib/contentRoutes";
import {
	type Counts,
	type Filters,
	type TreeData,
	treeCounts,
} from "@/lib/contentTree";
import { formatShortDay } from "@/lib/dates";
import { listAllLessons } from "@/lib/review";
import { ContentFilters } from "@/pages/admin/ContentFilters";
import { ContentTree } from "@/pages/admin/ContentTree";
import {
	ItemTitle,
	NoteFlag,
	StatusPill,
	useContentFailure,
} from "@/pages/admin/contentParts";
import { LessonCards } from "@/pages/admin/LessonCards";

const PER_PAGE = 50;
const SEARCH_DELAY_MS = 250;

type Loading<T> =
	| { state: "loading" }
	| { state: "failed" }
	| { state: "ready"; data: T };

type TrackFilter = Track | "";

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

function filtersOf(search: ContentSearch): Filters {
	return {
		status: statusOf(search.status),
		note: search.note === true,
		audio: search.audio === "missing",
		track: search.track,
		unit: search.unit,
	};
}

/** The lessons, units and noted items the tree and its counts come from. */
function useTreeData(): Loading<TreeData> {
	const onFailure = useContentFailure();
	const [tree, setTree] = useState<Loading<TreeData>>({ state: "loading" });
	useEffect(() => {
		let current = true;
		Promise.all([
			listAllLessons({}),
			listAllContent({ kind: "unit" }),
			listAllContent({ hasReviewNote: true }),
		]).then(
			([{ lessons }, units, noted]) => {
				if (current) {
					setTree({ state: "ready", data: { lessons, units, noted } });
				}
			},
			(failure: unknown) => {
				if (current) {
					setTree({ state: "failed" });
					onFailure("admin.content_tree_failed", failure);
				}
			},
		);
		return () => {
			current = false;
		};
	}, [onFailure]);
	return tree;
}

/**
 * The content of both tracks: their units and lessons as a tree, one
 * lesson's cards, or the items matching a search.
 */
export function ContentPage() {
	const search = useSearch({ from: "/_app/admin/content" });
	const navigate = useNavigate({ from: "/admin/content" });
	const tree = useTreeData();
	const filters = filtersOf(search);
	const treeData = tree.state === "ready" ? tree.data : undefined;

	// Filters live in the URL, so that going back from an edit page finds them;
	// they replace the entry rather than pile up in the history.
	function filter(change: Partial<ContentSearch>) {
		void navigate({
			search: (previous) => ({ ...previous, page: undefined, ...change }),
			replace: true,
		});
	}

	if (search.lesson) {
		return (
			<div className="admin-content">
				<LessonCards
					key={search.lesson}
					lessonId={search.lesson}
					search={search}
					filters={filters}
					tree={treeData}
					onFilter={filter}
				/>
			</div>
		);
	}

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
				</header>
				<div className="admin-header-actions">
					{createLinks.map((link) => (
						<Link
							key={link.kind}
							to="/admin/content/$section/new"
							params={{ section: sectionOf[link.kind] }}
							search={
								link.kind === "lesson" && search.unit
									? { parent: search.unit }
									: {}
							}
							state={{ contentList: search }}
							className={link.className}
						>
							{link.label}
						</Link>
					))}
				</div>
			</div>

			{search.q ? (
				<SearchResults
					search={search}
					filters={filters}
					tree={treeData}
					onFilter={filter}
				/>
			) : (
				<>
					<ListFilters
						search={search}
						filters={filters}
						counts={treeData ? treeCounts(treeData, filters) : undefined}
						onFilter={filter}
					/>
					{tree.state === "failed" ? (
						<p role="alert" className="form-error">
							Impossible de charger le contenu. Réessaie dans un moment.
						</p>
					) : null}
					{tree.state === "loading" ? (
						<p className="admin-table-message">Chargement…</p>
					) : null}
					{treeData ? (
						<ContentTree data={treeData} filters={filters} search={search} />
					) : null}
				</>
			)}
		</div>
	);
}

interface ListFiltersProps {
	search: ContentSearch;
	filters: Filters;
	counts: Partial<Counts> | undefined;
	onFilter: (change: Partial<ContentSearch>) => void;
}

/** The filters, with the track and the search the tree and results share. */
function ListFilters({ search, filters, counts, onFilter }: ListFiltersProps) {
	const navigate = useNavigate({ from: "/admin/content" });
	const q = search.q ?? "";
	const [text, setText] = useState(q);
	const typed = useDebounced(text, SEARCH_DELAY_MS);

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

	return (
		<ContentFilters
			filters={filters}
			counts={counts}
			onFilter={onFilter}
			extra={
				<div className="admin-filter">
					<label htmlFor="content-parcours" className="admin-filter-label">
						Parcours
					</label>
					<select
						id="content-parcours"
						className="admin-select"
						value={search.track ?? ""}
						onChange={(event) => {
							const chosen = trackOptions.find(
								(option) => option.value === event.target.value,
							);
							onFilter({ track: chosen?.value || undefined });
						}}
					>
						{trackOptions.map((option) => (
							<option key={option.value} value={option.value}>
								{option.label}
							</option>
						))}
					</select>
				</div>
			}
		>
			<label htmlFor="content-recherche" className="visually-hidden">
				Rechercher du contenu
			</label>
			<input
				id="content-recherche"
				type="search"
				className="admin-search"
				placeholder="Rechercher un titre ou un mot"
				autoComplete="off"
				value={text}
				onChange={(event) => setText(event.target.value)}
			/>
		</ContentFilters>
	);
}

interface SearchResultsProps {
	search: ContentSearch;
	filters: Filters;
	tree: TreeData | undefined;
	onFilter: (change: Partial<ContentSearch>) => void;
}

/** The items matching the search, the latest edit first. */
function SearchResults({
	search,
	filters,
	tree,
	onFilter,
}: SearchResultsProps) {
	const onFailure = useContentFailure();
	const [listing, setListing] = useState<Loading<Page>>({ state: "loading" });
	const pageNumber = search.page ?? 1;
	const { status, track } = filters;
	const noted = filters.note;
	const q = search.q;

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
			(data) => {
				if (current) {
					setListing({ state: "ready", data });
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

	const page = listing.state === "ready" ? listing.data : undefined;
	const pageCount = page
		? Math.max(1, Math.ceil(page.total / page.perPage))
		: 1;
	const counts = page
		? {
				all: page.statusCounts.toReview + page.statusCounts.validated,
				toReview: page.statusCounts.toReview,
				validated: page.statusCounts.validated,
				noted: page.reviewNoteCount,
			}
		: undefined;
	// The search spans every unit, and the API has no audio filter: both
	// belong to the tree alone.
	const searched: Filters = { ...filters, audio: false, unit: undefined };

	return (
		<>
			<ListFilters
				search={search}
				filters={searched}
				counts={counts}
				onFilter={onFilter}
			/>
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
					<ResultTable items={page.items} search={search} tree={tree} />
				) : null}
			</div>
			{page && pageCount > 1 ? (
				<nav aria-label="Pages" className="admin-pagination">
					<button
						type="button"
						className="button-secondary"
						disabled={pageNumber <= 1}
						onClick={() =>
							onFilter({ page: pageNumber > 2 ? pageNumber - 1 : undefined })
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
						onClick={() => onFilter({ page: pageNumber + 1 })}
					>
						Suivant
					</button>
				</nav>
			) : null}
		</>
	);
}

interface ResultTableProps {
	items: ContentItem[];
	/** The page's filters, which the edit pages lead back to. */
	search: ContentSearch;
	tree: TreeData | undefined;
}

function ResultTable({ items, search, tree }: ResultTableProps) {
	const navigate = useNavigate();
	if (items.length === 0) {
		return <p className="admin-table-message">Aucun contenu ne correspond.</p>;
	}
	return (
		<div className="admin-table-scroll">
			<table className="admin-table content-table">
				<thead>
					<tr>
						<th scope="col">Titre</th>
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
								if (!onLink && !window.getSelection()?.toString()) {
									void navigate({
										to: "/admin/content/$section/$itemId",
										params: { section: sectionOf[item.kind], itemId: item.id },
										state: { contentList: search },
									});
								}
							}}
						>
							<th scope="row">
								<Link
									to="/admin/content/$section/$itemId"
									params={{ section: sectionOf[item.kind], itemId: item.id }}
									state={{ contentList: search }}
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
								<ItemPath item={item} tree={tree} />
								{item.reviewNote ? <NoteFlag /> : null}
							</th>
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

/** "Arabe › L'alphabet › Les lettres": where an item sits, each step a link. */
function ItemPath({
	item,
	tree,
}: {
	item: ContentItem;
	tree: TreeData | undefined;
}) {
	const unitTitle =
		item.kind === "card"
			? tree?.lessons.find((lesson) => lesson.id === item.lessonId)?.unit.title
			: item.parentTitle;
	return (
		<span className="content-parent content-path">
			{trackLabels[item.track]}
			{item.unitId && unitTitle ? (
				<>
					{" › "}
					<Link to="/admin/content" search={{ unit: item.unitId }}>
						{unitTitle}
					</Link>
				</>
			) : null}
			{item.lessonId && item.parentTitle ? (
				<>
					{" › "}
					<Link to="/admin/content" search={{ lesson: item.lessonId }}>
						{item.parentTitle}
					</Link>
				</>
			) : null}
		</span>
	);
}
