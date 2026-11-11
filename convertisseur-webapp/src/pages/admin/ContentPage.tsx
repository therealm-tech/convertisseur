import { useRouter } from "@tanstack/react-router";
import { useCallback, useEffect, useState } from "react";
import { ApiError, problemTypes } from "@/lib/api";
import {
	type ContentItem,
	type Kind,
	listContent,
	type ContentPage as Page,
	type Status,
	type Track,
} from "@/lib/content";
import { kindLabels, statusLabels, trackLabels } from "@/lib/contentLabels";
import { formatShortDay } from "@/lib/dates";
import { logger } from "@/lib/logger";
import { ContentDetail, type Selection } from "@/pages/admin/ContentDetail";
import { ItemTitle, StatusPill } from "@/pages/admin/contentParts";

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
	const router = useRouter();
	const [status, setStatusFilter] = useState<StatusFilter>("");
	const [track, setTrack] = useState<TrackFilter>("");
	const [search, setSearch] = useState("");
	const [pageNumber, setPageNumber] = useState(1);
	const [listing, setListing] = useState<Listing>({ state: "loading" });
	const [reloads, setReloads] = useState(0);
	const [selection, setSelection] = useState<Selection | undefined>();
	const [toast, setToast] = useState<string | undefined>();
	const q = useDebounced(search, SEARCH_DELAY_MS);

	// A session that ended, lost its role or must change its password
	// answers 401 or 403: the route guards decide where to go.
	const onFailure = useCallback(
		(event: string, failure: unknown) => {
			logger.warn(event, { error: failure });
			if (
				failure instanceof ApiError &&
				(failure.status === 401 ||
					failure.type === problemTypes.passwordChangeRequired ||
					failure.type === problemTypes.forbidden)
			) {
				void router.invalidate();
			}
		},
		[router],
	);

	useEffect(() => {
		// The reload counter is a dependency on purpose: bumping it fetches the
		// page again after a change.
		void reloads;
		let current = true;
		listContent({
			page: pageNumber,
			perPage: PER_PAGE,
			status: status || undefined,
			track: track || undefined,
			q,
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
	}, [pageNumber, status, track, q, reloads, onFailure]);

	function reload(message?: string) {
		setToast(message);
		setReloads((count) => count + 1);
	}

	function create(kind: Kind) {
		setToast(undefined);
		setSelection({ mode: "create", kind });
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
					<button
						type="button"
						className="button-outline"
						onClick={() => create("unit")}
					>
						Nouvelle unité
					</button>
					<button
						type="button"
						className="button-outline"
						onClick={() => create("card")}
					>
						Nouvelle carte
					</button>
					<button
						type="button"
						className="button-solid"
						onClick={() => create("lesson")}
					>
						Nouvelle leçon
					</button>
				</div>
			</div>

			<div className="admin-filters">
				<fieldset className="status-filter">
					<legend className="visually-hidden">Filtrer par statut</legend>
					{statusOptions.map((option) => (
						<button
							key={option.value}
							type="button"
							aria-pressed={status === option.value}
							onClick={() => {
								setStatusFilter(option.value);
								setPageNumber(1);
							}}
						>
							{option.label}
							{option.count === undefined ? null : ` · ${option.count}`}
						</button>
					))}
				</fieldset>
				<div className="admin-filter">
					<label htmlFor="content-parcours" className="admin-filter-label">
						Parcours
					</label>
					<select
						id="content-parcours"
						className="admin-select"
						value={track}
						onChange={(event) => {
							const chosen = trackOptions.find(
								(option) => option.value === event.target.value,
							);
							setTrack(chosen?.value ?? "");
							setPageNumber(1);
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
					onChange={(event) => {
						setSearch(event.target.value);
						setPageNumber(1);
					}}
				/>
			</div>

			{toast ? (
				<p role="status" className="admin-toast">
					{toast}
				</p>
			) : null}

			<div className="admin-columns content-columns">
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
						<ContentTable
							items={page.items}
							selected={selection?.mode === "edit" ? selection.id : undefined}
							onSelect={(item) => {
								setToast(undefined);
								setSelection({ mode: "edit", kind: item.kind, id: item.id });
							}}
						/>
					) : null}
				</div>
				{selection ? (
					<ContentDetail
						key={
							selection.mode === "edit"
								? selection.id
								: `nouveau-${selection.kind}`
						}
						selection={selection}
						onFailure={onFailure}
						onChanged={(message, next) => {
							if (next !== undefined) {
								setSelection(next ?? undefined);
							}
							reload(message);
						}}
					/>
				) : (
					<section aria-label="Détail" className="content-detail">
						<p className="content-detail-empty">
							Choisis une ligne pour la relire ou la modifier.
						</p>
					</section>
				)}
			</div>

			{page && pageCount > 1 ? (
				<nav aria-label="Pages" className="admin-pagination">
					<button
						type="button"
						className="button-secondary"
						disabled={pageNumber <= 1}
						onClick={() => setPageNumber((n) => n - 1)}
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
						onClick={() => setPageNumber((n) => n + 1)}
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
	selected: string | undefined;
	onSelect: (item: ContentItem) => void;
}

function ContentTable({ items, selected, onSelect }: ContentTableProps) {
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
							className={
								item.id === selected ? "content-row-selected" : undefined
							}
						>
							<th scope="row">
								<button
									type="button"
									className="content-title-button"
									aria-pressed={item.id === selected}
									onClick={() => onSelect(item)}
								>
									<ItemTitle
										title={item.title}
										transliteration={item.transliteration}
										arabic={item.kind === "card"}
									/>
								</button>
								{item.parentTitle ? (
									<span className="content-parent">{item.parentTitle}</span>
								) : null}
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
