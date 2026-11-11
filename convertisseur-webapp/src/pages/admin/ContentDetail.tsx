import { useEffect, useId, useState } from "react";
import { Dialog } from "@/components/Dialog";
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
	setStatus,
	type Track,
	updateItem,
	uploadAudio,
	type Variety,
} from "@/lib/content";
import { kindLabels, trackLabels, varietyLabels } from "@/lib/contentLabels";
import { AudioBlock } from "@/pages/admin/AudioBlock";
import { ItemTitle, StatusPill } from "@/pages/admin/contentParts";

export type Selection =
	| { mode: "edit"; kind: Kind; id: string }
	| { mode: "create"; kind: Kind };

type Loaded = { [K in Kind]: { kind: K; item: ItemOf[K] } }[Kind];
type Draft = { [K in Kind]: { kind: K; fields: FieldsOf[K] } }[Kind];

type Phase =
	| { phase: "loading" }
	| { phase: "failed" }
	| { phase: "ready"; loaded: Loaded | undefined; draft: Draft };

interface ContentDetailProps {
	selection: Selection;
	onFailure: (event: string, failure: unknown) => void;
	/**
	 * Something changed: the list reloads, shows `message`, and moves the
	 * selection to `next` when given (`null` clears it).
	 */
	onChanged: (message: string | undefined, next?: Selection | null) => void;
}

const UNEXPECTED = "Une erreur est survenue. Réessaie dans un moment.";

const fieldMessages: Record<string, string> = {
	title_length: "Le titre doit faire entre 1 et 200 caractères.",
	arabic_length: "Le recto doit faire entre 1 et 500 caractères.",
	arabic_script: "Le recto doit être écrit en lettres arabes.",
	text_length: "Ce texte ne doit pas dépasser 500 caractères.",
	unit_not_found: "Cette unité n'existe plus.",
	lesson_not_found: "Cette leçon n'existe plus.",
	range: "La position doit être un nombre à partir de 1.",
};

const notEmptyMessages: Record<Kind, string> = {
	unit: "Cette unité contient encore des leçons : déplace-les ou supprime-les d'abord.",
	lesson:
		"Cette leçon contient encore des cartes : déplace-les ou supprime-les d'abord.",
	card: UNEXPECTED,
};

function emptyDraft(kind: Kind): Draft {
	switch (kind) {
		case "unit":
			return { kind, fields: { track: "arabe", title: "" } };
		case "lesson":
			return { kind, fields: { unitId: "", title: "" } };
		case "card":
			return {
				kind,
				fields: {
					lessonId: "",
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
		const messages = failure.fieldProblems
			.map((problem) => fieldMessages[problem.code])
			.filter((message) => message !== undefined);
		if (messages.length > 0) {
			return messages.join(" ");
		}
	}
	return UNEXPECTED;
}

/** The detail panel: one item to review and edit, or a new one to write. */
export function ContentDetail({
	selection,
	onFailure,
	onChanged,
}: ContentDetailProps) {
	const [state, setState] = useState<Phase>(
		selection.mode === "create"
			? {
					phase: "ready",
					loaded: undefined,
					draft: emptyDraft(selection.kind),
				}
			: { phase: "loading" },
	);
	const [position, setPosition] = useState("");
	const [busy, setBusy] = useState(false);
	const [error, setError] = useState<string | undefined>();
	const [confirmingDelete, setConfirmingDelete] = useState(false);

	useEffect(() => {
		if (selection.mode !== "edit") {
			return;
		}
		let current = true;
		load(selection.kind, selection.id).then(
			(loaded) => {
				if (current) {
					setState({ phase: "ready", loaded, draft: draftOf(loaded) });
					setPosition(String(loaded.item.position));
				}
			},
			(failure: unknown) => {
				if (current) {
					setState({ phase: "failed" });
					onFailure("admin.content_load_failed", failure);
				}
			},
		);
		return () => {
			current = false;
		};
	}, [selection, onFailure]);

	if (state.phase === "loading") {
		return (
			<section aria-label="Détail" className="content-detail">
				<p className="content-detail-empty">Chargement…</p>
			</section>
		);
	}
	if (state.phase === "failed") {
		return (
			<section aria-label="Détail" className="content-detail">
				<p role="alert" className="form-error">
					Impossible de charger cet élément. Réessaie dans un moment.
				</p>
			</section>
		);
	}

	const { loaded, draft } = state;
	const kind = draft.kind;

	function setDraft(next: Draft) {
		if (state.phase === "ready") {
			setState({ ...state, draft: next });
		}
	}

	async function submit() {
		setBusy(true);
		setError(undefined);
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
				onChanged("Modifications enregistrées.");
			} else {
				const created = await create(draft);
				onChanged(`${kindLabels[kind]} créée.`, {
					mode: "edit",
					kind,
					id: created.item.id,
				});
			}
		} catch (failure) {
			onFailure("admin.content_save_failed", failure);
			setError(describeFailure(failure, kind));
		} finally {
			setBusy(false);
		}
	}

	/** Reloads the item after a change of its recording. */
	async function refresh(current: Loaded, message: string) {
		const reloaded = await load(current.kind, current.item.id);
		setState({ phase: "ready", loaded: reloaded, draft });
		onChanged(message);
	}

	async function toggleStatus(current: Loaded) {
		const next =
			current.item.status === "validated" ? "to_review" : "validated";
		setBusy(true);
		setError(undefined);
		try {
			const change = await setStatus(current.kind, current.item.id, next);
			const item = { ...current.item, ...change };
			setState({
				phase: "ready",
				// The spread keeps the item of the same kind it came from.
				loaded: { ...current, item } as Loaded,
				draft,
			});
			onChanged(
				next === "validated" ? "Contenu validé." : "Contenu repassé à relire.",
			);
		} catch (failure) {
			onFailure("admin.content_status_failed", failure);
			setError(UNEXPECTED);
		} finally {
			setBusy(false);
		}
	}

	async function confirmDelete(current: Loaded) {
		setConfirmingDelete(false);
		setBusy(true);
		setError(undefined);
		try {
			await deleteItem(current.kind, current.item.id);
			onChanged(`${kindLabels[current.kind]} supprimée.`, null);
		} catch (failure) {
			onFailure("admin.content_delete_failed", failure);
			setError(describeFailure(failure, current.kind));
			setBusy(false);
		}
	}

	return (
		<section aria-label="Détail" className="content-detail">
			<div className="content-detail-meta">
				<span className="content-detail-kind">
					{loaded
						? `${kindLabels[kind]} · ${trackLabels[loaded.item.track]}`
						: `Nouvelle ${kindLabels[kind].toLowerCase()}`}
				</span>
				{loaded ? <StatusPill status={loaded.item.status} /> : null}
			</div>
			{loaded ? (
				<h2 className="content-detail-title">
					<LoadedTitle loaded={loaded} />
				</h2>
			) : null}

			<form
				className="content-form"
				onSubmit={(event) => {
					event.preventDefault();
					void submit();
				}}
			>
				<DraftFields draft={draft} onChange={setDraft} />
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
							value={position}
							onChange={(event) => setPosition(event.target.value)}
						/>
					</div>
				) : null}

				{loaded && loaded.kind !== "unit" ? (
					<AudioBlock
						audio={loaded.item.audio}
						onUpload={async (file, filename) => {
							await uploadAudio(loaded.kind, loaded.item.id, file, filename);
							await refresh(loaded, "Audio enregistré : à relire.");
						}}
						onRemove={async () => {
							await deleteAudio(loaded.kind, loaded.item.id);
							await refresh(loaded, "Audio supprimé : à relire.");
						}}
					/>
				) : null}
				{!loaded && kind !== "unit" ? (
					<p className="field-hint">
						L'audio s'ajoute une fois la {kindLabels[kind].toLowerCase()} créée.
					</p>
				) : null}

				{loaded ? <ReviewNote loaded={loaded} /> : null}

				{error ? (
					<p role="alert" className="form-error">
						{error}
					</p>
				) : null}

				<div className="content-actions">
					<button type="submit" className="button-outline" disabled={busy}>
						{loaded ? "Sauvegarder" : "Créer"}
					</button>
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
				{loaded ? (
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
		</section>
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

function ReviewNote({ loaded }: { loaded: Loaded }) {
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
	onChange: (draft: Draft) => void;
}

function DraftFields({ draft, onChange }: DraftFieldsProps) {
	switch (draft.kind) {
		case "unit":
			return (
				<>
					<SelectField
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
						kind="unit"
						value={draft.fields.unitId}
						onChange={(unitId) =>
							onChange({ kind: "lesson", fields: { ...draft.fields, unitId } })
						}
					/>
					<InputField
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
						kind="lesson"
						value={draft.fields.lessonId}
						onChange={(lessonId) =>
							onChange({ kind: "card", fields: { ...draft.fields, lessonId } })
						}
					/>
					<InputField
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
	onChange: (value: string) => void;
}

function InputField({ id, label, value, arabic, onChange }: InputFieldProps) {
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
	onChange: (value: T) => void;
}

function SelectField<T extends string>({
	id,
	label,
	value,
	options,
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
	onChange,
}: {
	kind: "unit" | "lesson";
	value: string;
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
				disabled={parents === undefined}
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
