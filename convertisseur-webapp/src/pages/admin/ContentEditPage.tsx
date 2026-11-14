import {
	Link,
	useLocation,
	useNavigate,
	useParams,
	useRouteContext,
	useRouter,
	useSearch,
} from "@tanstack/react-router";
import { useEffect, useId, useState } from "react";
import { Dialog } from "@/components/Dialog";
import { BackIcon } from "@/components/icons";
import { LetterFormsRow } from "@/components/LetterFormsRow";
import { ApiError, problemTypes } from "@/lib/api";
import {
	type ContentItem,
	createItem,
	deleteAudio,
	deleteItem,
	type FieldsOf,
	getItem,
	type ItemOf,
	type Kind,
	listContent,
	setReviewNote,
	setStatus,
	type Track,
	updateItem,
	uploadAudio,
	type Variety,
} from "@/lib/content";
import { kindLabels, trackLabels, varietyLabels } from "@/lib/contentLabels";
import { type ContentSearch, listOrigin, sectionOf } from "@/lib/contentRoutes";
import { AudioBlock } from "@/pages/admin/AudioBlock";
import {
	ContentCrumbs,
	type Crumb,
	keptFilters,
} from "@/pages/admin/contentNav";
import {
	ItemTitle,
	RepoBadge,
	StatusPill,
	useContentFailure,
} from "@/pages/admin/contentParts";

type Loaded = { [K in Kind]: { kind: K; item: ItemOf[K] } }[Kind];
type Draft = { [K in Kind]: { kind: K; fields: FieldsOf[K] } }[Kind];

type Phase =
	| { phase: "loading" }
	| { phase: "missing" }
	| { phase: "failed" }
	| { phase: "ready"; loaded: Loaded | undefined; draft: Draft };

const UNEXPECTED = "Une erreur est survenue. Réessaie dans un moment.";

const fieldMessages: Record<string, string> = {
	title_length: "Le titre doit faire entre 1 et 200 caractères.",
	arabic_length: "Le recto doit faire entre 1 et 500 caractères.",
	arabic_script: "Le recto doit être écrit en lettres arabes.",
	text_length: "Ce texte ne doit pas dépasser 500 caractères.",
	unit_not_found: "Cette unité n'existe plus.",
	lesson_not_found: "Cette leçon n'existe plus.",
	range: "La position doit être un nombre à partir de 1.",
	review_note_length: "La remarque ne doit pas dépasser 2000 caractères.",
};

const MAX_NOTE_CHARS = 2000;

const IMPORTED = "Ce contenu se corrige dans les fichiers du dépôt.";

const notEmptyMessages: Record<Kind, string> = {
	unit: "Cette unité contient encore des leçons : déplace-les ou supprime-les d'abord.",
	lesson:
		"Cette leçon contient encore des cartes : déplace-les ou supprime-les d'abord.",
	card: UNEXPECTED,
};

/** A new item's fields, in `parentId` when given: a unit, or a lesson. */
function emptyDraft(kind: Kind, parentId = ""): Draft {
	switch (kind) {
		case "unit":
			return { kind, fields: { track: "arabe", title: "" } };
		case "lesson":
			return { kind, fields: { unitId: parentId, title: "" } };
		case "card":
			return {
				kind,
				fields: {
					lessonId: parentId,
					arabic: "",
					transliteration: "",
					meaningFr: "",
					variety: "classique",
				},
			};
	}
}

function draftOf(loaded: Loaded): Draft {
	switch (loaded.kind) {
		case "unit":
			return {
				kind: "unit",
				fields: { track: loaded.item.track, title: loaded.item.title },
			};
		case "lesson":
			return {
				kind: "lesson",
				fields: { unitId: loaded.item.unit.id, title: loaded.item.title },
			};
		case "card":
			return {
				kind: "card",
				fields: {
					lessonId: loaded.item.lesson.id,
					arabic: loaded.item.arabic,
					transliteration: loaded.item.transliteration ?? "",
					meaningFr: loaded.item.meaningFr ?? "",
					variety: loaded.item.variety,
				},
			};
	}
}

async function load(kind: Kind, id: string): Promise<Loaded> {
	switch (kind) {
		case "unit":
			return { kind, item: await getItem(kind, id) };
		case "lesson":
			return { kind, item: await getItem(kind, id) };
		case "card":
			return { kind, item: await getItem(kind, id) };
	}
}

async function create(draft: Draft): Promise<Loaded> {
	switch (draft.kind) {
		case "unit":
			return { kind: "unit", item: await createItem("unit", draft.fields) };
		case "lesson":
			return {
				kind: "lesson",
				item: await createItem("lesson", draft.fields),
			};
		case "card":
			return { kind: "card", item: await createItem("card", draft.fields) };
	}
}

async function save(id: string, draft: Draft, position: number) {
	switch (draft.kind) {
		case "unit":
			await updateItem("unit", id, { ...draft.fields, position });
			return;
		case "lesson":
			await updateItem("lesson", id, { ...draft.fields, position });
			return;
		case "card":
			await updateItem("card", id, { ...draft.fields, position });
			return;
	}
}

function describeFailure(failure: unknown, kind: Kind): string {
	if (failure instanceof ApiError) {
		if (failure.type === problemTypes.notEmpty) {
			return notEmptyMessages[kind];
		}
		if (failure.type === problemTypes.importedContent) {
			return IMPORTED;
		}
		const messages = failure.fieldProblems
			.map((problem) => fieldMessages[problem.code])
			.filter((message) => message !== undefined);
		if (messages.length > 0) {
			return messages.join(" ");
		}
	}
	return UNEXPECTED;
}

/** One unit, lesson or card to review and edit. */
export function ContentEditPage() {
	const from = "/_app/admin/content/$section/$itemId";
	const { kind } = useRouteContext({ from });
	const { itemId } = useParams({ from });
	return <ContentEditor key={itemId} kind={kind} id={itemId} />;
}

/** A new unit, lesson or card to write. */
export function ContentCreatePage() {
	const from = "/_app/admin/content/$section/new";
	const { kind } = useRouteContext({ from });
	const { parent } = useSearch({ from });
	return (
		<ContentEditor key={kind} kind={kind} id={undefined} parentId={parent} />
	);
}

/**
 * Goes back to the list, to the very entry it was opened from when it was,
 * so that the list keeps its filters and its scroll.
 */
function useBackToList() {
	const router = useRouter();
	const navigate = useNavigate();
	const origin = useLocation({ select: (location) => location.state });
	const filters = listOrigin(origin);
	return {
		filters,
		/** Back to the list; `deleted` names a lesson whose page it was. */
		back: (deleted?: string) => {
			if (
				filters &&
				filters.lesson !== undefined &&
				filters.lesson === deleted
			) {
				void navigate({
					to: "/admin/content",
					search: { ...filters, lesson: undefined },
					replace: true,
				});
			} else if (filters) {
				router.history.back();
			} else {
				void navigate({ to: "/admin/content", replace: true });
			}
		},
	};
}

function BackToList() {
	const { filters, back } = useBackToList();
	return (
		<Link
			to="/admin/content"
			search={filters ?? {}}
			className="button-outline content-back-link"
			onClick={(event) => {
				if (filters) {
					event.preventDefault();
					back();
				}
			}}
		>
			<BackIcon />
			Retour à la liste
		</Link>
	);
}

/** Where the item sits: its track, then its unit or lesson. */
function EditCrumbs({
	loaded,
	search,
}: {
	loaded: Loaded;
	search: ContentSearch;
}) {
	const track: Crumb = {
		label: trackLabels[loaded.item.track],
		search: { track: loaded.item.track },
	};
	const trail: Crumb[] = [track];
	if (loaded.kind === "lesson") {
		trail.push({
			label: loaded.item.unit.title,
			search: { unit: loaded.item.unit.id },
		});
	} else if (loaded.kind === "card") {
		trail.push({
			label: loaded.item.lesson.title,
			search: { lesson: loaded.item.lesson.id },
		});
	}
	return <ContentCrumbs search={search} trail={trail} />;
}

/** The way to what a unit or lesson holds, and to a lesson's review. */
function ChildLinks({
	loaded,
	search,
}: {
	loaded: Loaded;
	search: ContentSearch;
}) {
	const kept = keptFilters(search);
	if (loaded.kind === "unit") {
		return (
			<div className="content-child-links">
				<Link
					to="/admin/content"
					search={{ ...kept, unit: loaded.item.id }}
					className="button-outline content-review-link"
				>
					Voir ses leçons
				</Link>
			</div>
		);
	}
	if (loaded.kind === "lesson") {
		return (
			<div className="content-child-links">
				<Link
					to="/admin/content"
					search={{ ...kept, lesson: loaded.item.id }}
					className="button-outline content-review-link"
				>
					Voir les cartes
				</Link>
				<Link
					to="/admin/review/lessons/$lessonId"
					params={{ lessonId: loaded.item.id }}
					className="button-outline content-review-link"
				>
					Relire cette leçon
				</Link>
			</div>
		);
	}
	return null;
}

interface ContentEditorProps {
	kind: Kind;
	/** The item to edit; a new one is written when undefined. */
	id: string | undefined;
	/** The unit or lesson a new item starts in. */
	parentId?: string | undefined;
}

function ContentEditor({ kind: pageKind, id, parentId }: ContentEditorProps) {
	const navigate = useNavigate();
	const onFailure = useContentFailure();
	const { filters, back } = useBackToList();
	const titleId = useId();
	const [state, setState] = useState<Phase>(
		id === undefined
			? {
					phase: "ready",
					loaded: undefined,
					draft: emptyDraft(pageKind, parentId),
				}
			: { phase: "loading" },
	);
	const [position, setPosition] = useState("");
	const [note, setNote] = useState("");
	const [busy, setBusy] = useState(false);
	const [error, setError] = useState<string | undefined>();
	const [toast, setToast] = useState<string | undefined>();
	const [confirmingDelete, setConfirmingDelete] = useState(false);

	useEffect(() => {
		if (id === undefined) {
			return;
		}
		let current = true;
		load(pageKind, id).then(
			(loaded) => {
				if (current) {
					setState({ phase: "ready", loaded, draft: draftOf(loaded) });
					setPosition(String(loaded.item.position));
					setNote(loaded.item.reviewNote ?? "");
				}
			},
			(failure: unknown) => {
				if (current) {
					setState(
						failure instanceof ApiError && failure.status === 404
							? { phase: "missing" }
							: { phase: "failed" },
					);
					onFailure("admin.content_load_failed", failure);
				}
			},
		);
		return () => {
			current = false;
		};
	}, [pageKind, id, onFailure]);

	if (state.phase !== "ready") {
		return (
			<div className="admin-content content-edit">
				<BackToList />
				<section className="content-detail">
					{state.phase === "loading" ? (
						<p className="content-detail-empty">Chargement…</p>
					) : (
						<p role="alert" className="form-error">
							{state.phase === "missing"
								? `${kindLabels[pageKind]} introuvable.`
								: "Impossible de charger cet élément. Réessaie dans un moment."}
						</p>
					)}
				</section>
			</div>
		);
	}

	const { loaded, draft } = state;
	const kind = draft.kind;
	const imported = loaded?.item.importKey != null;

	function setDraft(next: Draft) {
		if (state.phase === "ready") {
			setState({ ...state, draft: next });
		}
	}

	function begin() {
		setBusy(true);
		setError(undefined);
		setToast(undefined);
	}

	async function submit() {
		begin();
		try {
			if (loaded) {
				const wanted = Number.parseInt(position, 10);
				await save(
					loaded.item.id,
					draft,
					Number.isFinite(wanted) ? wanted : loaded.item.position,
				);
				const reloaded = await load(kind, loaded.item.id);
				setState({
					phase: "ready",
					loaded: reloaded,
					draft: draftOf(reloaded),
				});
				setPosition(String(reloaded.item.position));
				setToast("Modifications enregistrées.");
				setBusy(false);
			} else {
				const created = await create(draft);
				await navigate({
					to: "/admin/content/$section/$itemId",
					params: { section: sectionOf[kind], itemId: created.item.id },
					replace: true,
					state: true,
				});
			}
		} catch (failure) {
			onFailure("admin.content_save_failed", failure);
			setError(describeFailure(failure, kind));
			setBusy(false);
		}
	}

	/** Reloads the item after a change of its recording. */
	async function refresh(current: Loaded, message: string) {
		const reloaded = await load(current.kind, current.item.id);
		setState({ phase: "ready", loaded: reloaded, draft });
		setToast(message);
	}

	async function toggleStatus(current: Loaded) {
		const next =
			current.item.status === "validated" ? "to_review" : "validated";
		begin();
		try {
			const change = await setStatus(current.kind, current.item.id, next);
			const item = { ...current.item, ...change };
			setState({
				phase: "ready",
				// The spread keeps the item of the same kind it came from.
				loaded: { ...current, item } as Loaded,
				draft,
			});
			setToast(
				next === "validated" ? "Contenu validé." : "Contenu repassé à relire.",
			);
		} catch (failure) {
			onFailure("admin.content_status_failed", failure);
			setError(UNEXPECTED);
		} finally {
			setBusy(false);
		}
	}

	async function saveNote(current: Loaded) {
		begin();
		try {
			const reviewNote = await setReviewNote(
				current.kind,
				current.item.id,
				note,
			);
			const item = { ...current.item, reviewNote };
			setState({
				phase: "ready",
				// The spread keeps the item of the same kind it came from.
				loaded: { ...current, item } as Loaded,
				draft,
			});
			setNote(reviewNote ?? "");
			setToast(reviewNote ? "Remarque enregistrée." : "Remarque effacée.");
		} catch (failure) {
			onFailure("admin.content_note_failed", failure);
			setError(describeFailure(failure, current.kind));
		} finally {
			setBusy(false);
		}
	}

	async function confirmDelete(current: Loaded) {
		setConfirmingDelete(false);
		begin();
		try {
			await deleteItem(current.kind, current.item.id);
			back(current.item.id);
		} catch (failure) {
			onFailure("admin.content_delete_failed", failure);
			setError(describeFailure(failure, current.kind));
			setBusy(false);
		}
	}

	return (
		<div className="admin-content content-edit">
			<BackToList />
			{loaded ? <EditCrumbs loaded={loaded} search={filters ?? {}} /> : null}
			<section aria-labelledby={titleId} className="content-detail">
				{loaded ? (
					<div className="content-detail-meta">
						<span className="content-detail-kind">
							{`${kindLabels[kind]} · ${trackLabels[loaded.item.track]}`}
						</span>
						<span className="content-detail-badges">
							{imported ? <RepoBadge /> : null}
							<StatusPill status={loaded.item.status} />
						</span>
					</div>
				) : null}
				<h1 id={titleId} className="content-detail-title">
					{loaded ? (
						<LoadedTitle loaded={loaded} />
					) : (
						`Nouvelle ${kindLabels[kind].toLowerCase()}`
					)}
				</h1>
				{loaded ? <ChildLinks loaded={loaded} search={filters ?? {}} /> : null}
				{imported ? (
					<p className="content-locked">
						Texte et place se corrigent dans le dépôt. Signale une erreur par
						une remarque.
					</p>
				) : null}

				<form
					className="content-form"
					onSubmit={(event) => {
						event.preventDefault();
						void submit();
					}}
				>
					<DraftFields draft={draft} readOnly={imported} onChange={setDraft} />
					{loaded?.kind === "card" && loaded.item.forms ? (
						<div className="field">
							<span className="field-label">Formes</span>
							<LetterFormsRow
								arabic={loaded.item.arabic}
								forms={loaded.item.forms}
							/>
						</div>
					) : null}
					{loaded ? (
						<div className="field">
							<label htmlFor="contenu-position" className="field-label">
								Position
							</label>
							<input
								id="contenu-position"
								className="field-input content-position"
								type="number"
								inputMode="numeric"
								min={1}
								readOnly={imported}
								value={position}
								onChange={(event) => setPosition(event.target.value)}
							/>
						</div>
					) : null}

					{loaded && loaded.kind !== "unit" ? (
						<AudioBlock
							audio={loaded.item.audio}
							onUpload={async (file, filename, durationMs) => {
								setToast(undefined);
								await uploadAudio(
									loaded.kind,
									loaded.item.id,
									file,
									filename,
									durationMs,
								);
								await refresh(loaded, "Audio enregistré : à relire.");
							}}
							onRemove={async () => {
								setToast(undefined);
								await deleteAudio(loaded.kind, loaded.item.id);
								await refresh(loaded, "Audio supprimé : à relire.");
							}}
						/>
					) : null}
					{!loaded && kind !== "unit" ? (
						<p className="field-hint">
							L'audio s'ajoute une fois la {kindLabels[kind].toLowerCase()}{" "}
							créée.
						</p>
					) : null}

					{loaded ? (
						<div className="field">
							<label htmlFor="contenu-remarque" className="field-label">
								Remarque de relecture
							</label>
							<textarea
								id="contenu-remarque"
								className="field-input field-textarea"
								rows={3}
								maxLength={MAX_NOTE_CHARS}
								placeholder="Ex. : vocalisation fausse sur le 2e mot"
								value={note}
								onChange={(event) => setNote(event.target.value)}
							/>
							<div className="note-actions">
								<span className="field-hint">
									{note.length} / {MAX_NOTE_CHARS}
								</span>
								<button
									type="button"
									className="button-small"
									disabled={busy || note === (loaded.item.reviewNote ?? "")}
									onClick={() => void saveNote(loaded)}
								>
									Enregistrer la remarque
								</button>
							</div>
						</div>
					) : null}

					{loaded ? <ReviewState loaded={loaded} /> : null}

					{error ? (
						<p role="alert" className="form-error">
							{error}
						</p>
					) : null}
					{toast ? (
						<p role="status" className="admin-toast">
							{toast}
						</p>
					) : null}

					<div className="content-actions">
						{imported ? null : (
							<button type="submit" className="button-outline" disabled={busy}>
								{loaded ? "Sauvegarder" : "Créer"}
							</button>
						)}
						{loaded ? (
							<button
								type="button"
								className={
									loaded.item.status === "validated"
										? "button-review"
										: "button-solid"
								}
								disabled={busy}
								onClick={() => void toggleStatus(loaded)}
							>
								{loaded.item.status === "validated"
									? "Repasser à relire"
									: "Marquer comme validé"}
							</button>
						) : null}
					</div>
					{loaded && !imported ? (
						<button
							type="button"
							className="button-small button-danger content-delete"
							disabled={busy}
							onClick={() => setConfirmingDelete(true)}
						>
							Supprimer
						</button>
					) : null}
				</form>
			</section>

			{confirmingDelete && loaded ? (
				<Dialog
					title={`Supprimer cette ${kindLabels[kind].toLowerCase()} ?`}
					onClose={() => setConfirmingDelete(false)}
				>
					<p className="dialog-text">
						Elle disparaît pour de bon, et les suivantes remontent d'une place.
					</p>
					<div className="dialog-actions">
						<button
							type="button"
							className="button-secondary"
							onClick={() => setConfirmingDelete(false)}
						>
							Annuler
						</button>
						<button
							type="button"
							className="button-danger-solid dialog-primary"
							onClick={() => void confirmDelete(loaded)}
						>
							Supprimer
						</button>
					</div>
				</Dialog>
			) : null}
		</div>
	);
}

function LoadedTitle({ loaded }: { loaded: Loaded }) {
	if (loaded.kind === "card") {
		return (
			<ItemTitle
				title={loaded.item.arabic}
				transliteration={loaded.item.transliteration}
				arabic
			/>
		);
	}
	return <>{loaded.item.title}</>;
}

function ReviewState({ loaded }: { loaded: Loaded }) {
	const { status, validatedBy } = loaded.item;
	return (
		<p className="review-note">
			{status === "validated"
				? `Validé par ${validatedBy?.username ?? "un compte supprimé"}. Toute modification de contenu le repasse « à relire », pas un simple déplacement.`
				: "En attente de relecture. Invisible pour les apprenants."}
		</p>
	);
}

interface DraftFieldsProps {
	draft: Draft;
	/** An imported item's fields are shown, not edited. */
	readOnly: boolean;
	onChange: (draft: Draft) => void;
}

function DraftFields({ draft, readOnly, onChange }: DraftFieldsProps) {
	switch (draft.kind) {
		case "unit":
			return (
				<>
					<SelectField
						disabled={readOnly}
						id="contenu-parcours"
						label="Parcours"
						value={draft.fields.track}
						options={(["arabe", "islam"] as const).map((track) => ({
							value: track,
							label: trackLabels[track],
						}))}
						onChange={(track: Track) =>
							onChange({ kind: "unit", fields: { ...draft.fields, track } })
						}
					/>
					<InputField
						readOnly={readOnly}
						id="contenu-titre"
						label="Titre"
						value={draft.fields.title}
						onChange={(title) =>
							onChange({ kind: "unit", fields: { ...draft.fields, title } })
						}
					/>
				</>
			);
		case "lesson":
			return (
				<>
					<ParentField
						disabled={readOnly}
						kind="unit"
						value={draft.fields.unitId}
						onChange={(unitId) =>
							onChange({ kind: "lesson", fields: { ...draft.fields, unitId } })
						}
					/>
					<InputField
						readOnly={readOnly}
						id="contenu-titre"
						label="Titre"
						value={draft.fields.title}
						onChange={(title) =>
							onChange({ kind: "lesson", fields: { ...draft.fields, title } })
						}
					/>
				</>
			);
		case "card":
			return (
				<>
					<ParentField
						disabled={readOnly}
						kind="lesson"
						value={draft.fields.lessonId}
						onChange={(lessonId) =>
							onChange({ kind: "card", fields: { ...draft.fields, lessonId } })
						}
					/>
					<InputField
						readOnly={readOnly}
						id="contenu-arabe"
						label="Recto (arabe, avec voyelles)"
						arabic
						value={draft.fields.arabic}
						onChange={(arabic) =>
							onChange({ kind: "card", fields: { ...draft.fields, arabic } })
						}
					/>
					<div className="content-form-pair">
						<InputField
							readOnly={readOnly}
							id="contenu-translitteration"
							label="Translittération"
							value={draft.fields.transliteration}
							onChange={(transliteration) =>
								onChange({
									kind: "card",
									fields: { ...draft.fields, transliteration },
								})
							}
						/>
						<InputField
							readOnly={readOnly}
							id="contenu-sens"
							label="Sens"
							value={draft.fields.meaningFr}
							onChange={(meaningFr) =>
								onChange({
									kind: "card",
									fields: { ...draft.fields, meaningFr },
								})
							}
						/>
					</div>
					<SelectField
						disabled={readOnly}
						id="contenu-variete"
						label="Variété"
						value={draft.fields.variety}
						options={(["classique", "darija"] as const).map((variety) => ({
							value: variety,
							label: varietyLabels[variety],
						}))}
						onChange={(variety: Variety) =>
							onChange({ kind: "card", fields: { ...draft.fields, variety } })
						}
					/>
				</>
			);
	}
}

interface InputFieldProps {
	id: string;
	label: string;
	value: string;
	arabic?: boolean;
	readOnly: boolean;
	onChange: (value: string) => void;
}

function InputField({
	id,
	label,
	value,
	arabic,
	readOnly,
	onChange,
}: InputFieldProps) {
	return (
		<div className="field">
			<label htmlFor={id} className="field-label">
				{label}
			</label>
			<input
				id={id}
				className={arabic ? "field-input field-input-arabic" : "field-input"}
				type="text"
				autoComplete="off"
				{...(arabic ? { lang: "ar", dir: "rtl" } : {})}
				readOnly={readOnly}
				value={value}
				onChange={(event) => onChange(event.target.value)}
			/>
		</div>
	);
}

interface SelectFieldProps<T extends string> {
	id: string;
	label: string;
	value: T;
	options: readonly { value: T; label: string }[];
	disabled: boolean;
	onChange: (value: T) => void;
}

function SelectField<T extends string>({
	id,
	label,
	value,
	options,
	disabled,
	onChange,
}: SelectFieldProps<T>) {
	return (
		<div className="field">
			<label htmlFor={id} className="field-label">
				{label}
			</label>
			<select
				id={id}
				className="admin-select"
				value={value}
				disabled={disabled}
				onChange={(event) => {
					const chosen = options.find(
						(option) => option.value === event.target.value,
					);
					if (chosen) {
						onChange(chosen.value);
					}
				}}
			>
				{options.map((option) => (
					<option key={option.value} value={option.value}>
						{option.label}
					</option>
				))}
			</select>
		</div>
	);
}

const PARENT_PAGE = 100;

/** The unit of a lesson or the lesson of a card, picked among the first 100. */
function ParentField({
	kind,
	value,
	disabled,
	onChange,
}: {
	kind: "unit" | "lesson";
	value: string;
	disabled: boolean;
	onChange: (id: string) => void;
}) {
	const id = useId();
	const [parents, setParents] = useState<ContentItem[] | undefined>();

	useEffect(() => {
		let current = true;
		listContent({ page: 1, perPage: PARENT_PAGE, kind }).then(
			(page) => {
				if (current) {
					setParents(page.items);
				}
			},
			() => {
				if (current) {
					setParents([]);
				}
			},
		);
		return () => {
			current = false;
		};
	}, [kind]);

	const label = kind === "unit" ? "Unité" : "Leçon";
	return (
		<div className="field">
			<label htmlFor={id} className="field-label">
				{label}
			</label>
			<select
				id={id}
				className="admin-select"
				value={value}
				disabled={disabled || parents === undefined}
				onChange={(event) => onChange(event.target.value)}
			>
				<option value="" disabled>
					{parents === undefined
						? "Chargement…"
						: `Choisir une ${label.toLowerCase()}`}
				</option>
				{(parents ?? []).map((parent) => (
					<option key={parent.id} value={parent.id}>
						{[trackLabels[parent.track], parent.parentTitle, parent.title]
							.filter(Boolean)
							.join(" · ")}
					</option>
				))}
			</select>
		</div>
	);
}
