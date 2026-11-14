import { useRouteContext, useRouter } from "@tanstack/react-router";
import { useCallback, useEffect, useState } from "react";
import {
	ApiError,
	deleteUser,
	type ListedUser,
	listUsers,
	problemTypes,
	type Role,
	setRole,
	type User,
	type UserPage,
} from "@/lib/api";
import { formatDay, formatLastActivity } from "@/lib/dates";
import { logger } from "@/lib/logger";
import { canChangeRole, canManage, roleLabels } from "@/lib/roles";
import { DeleteUserDialog } from "@/pages/admin/DeleteUserDialog";
import { ResetPasswordDialog } from "@/pages/admin/ResetPasswordDialog";

const PER_PAGE = 50;
const SEARCH_DELAY_MS = 250;
const UNEXPECTED = "Une erreur est survenue. Réessaie dans un moment.";

type RoleFilter = Role | "";

type Pending =
	| { kind: "reset"; target: ListedUser }
	| { kind: "delete"; target: ListedUser };

type Listing =
	| { state: "loading" }
	| { state: "failed" }
	| { state: "ready"; page: UserPage };

const roleOptions: { value: RoleFilter; label: string }[] = [
	{ value: "", label: "Tous" },
	{ value: "user", label: roleLabels.user },
	{ value: "admin", label: roleLabels.admin },
	{ value: "super_admin", label: roleLabels.super_admin },
];

function useDebounced<T>(value: T, delayMs: number): T {
	const [debounced, setDebounced] = useState(value);
	useEffect(() => {
		const timer = setTimeout(() => setDebounced(value), delayMs);
		return () => clearTimeout(timer);
	}, [value, delayMs]);
	return debounced;
}

export function UsersPage() {
	const { user: viewer } = useRouteContext({ from: "/_app/admin" });
	const router = useRouter();
	const [search, setSearch] = useState("");
	const [role, setRoleFilter] = useState<RoleFilter>("");
	const [pageNumber, setPageNumber] = useState(1);
	const [listing, setListing] = useState<Listing>({ state: "loading" });
	const [reloads, setReloads] = useState(0);
	const [toast, setToast] = useState<string | undefined>();
	const [error, setError] = useState<string | undefined>();
	const [pending, setPending] = useState<Pending | undefined>();
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
		listUsers({
			page: pageNumber,
			perPage: PER_PAGE,
			q,
			role: role || undefined,
		}).then(
			(page) => {
				if (!current) {
					return;
				}
				// A deletion can empty the last page: show the new last one.
				const lastPage = Math.max(1, Math.ceil(page.total / page.perPage));
				if (page.items.length === 0 && pageNumber > lastPage) {
					setPageNumber(lastPage);
					return;
				}
				setListing({ state: "ready", page });
			},
			(failure: unknown) => {
				if (current) {
					setListing({ state: "failed" });
					onFailure("admin.users_list_failed", failure);
				}
			},
		);
		return () => {
			current = false;
		};
	}, [pageNumber, q, role, reloads, onFailure]);

	function reload(message: string) {
		setToast(message);
		setError(undefined);
		setReloads((count) => count + 1);
	}

	async function changeRole(target: ListedUser, next: "user" | "admin") {
		setToast(undefined);
		setError(undefined);
		try {
			await setRole(target.id, next);
			reload(`Rôle de ${target.username} : ${roleLabels[next]}.`);
		} catch (failure) {
			onFailure("admin.role_change_failed", failure);
			setError(UNEXPECTED);
		}
	}

	async function confirmDelete(target: ListedUser) {
		await deleteUser(target.id);
		setPending(undefined);
		reload(`Le compte de ${target.username} a été supprimé.`);
	}

	const page = listing.state === "ready" ? listing.page : undefined;
	const pageCount = page
		? Math.max(1, Math.ceil(page.total / page.perPage))
		: 1;

	return (
		<div className="admin-content">
			<header className="admin-header">
				<h1 className="admin-title">Utilisateurs et rôles</h1>
				<p className="admin-lead">
					Toute personne inscrite est « user ». Seul le super admin fait passer
					un compte de user à admin, ou l'inverse.
				</p>
			</header>

			<div className="admin-filters">
				<label htmlFor="users-recherche" className="visually-hidden">
					Rechercher un utilisateur
				</label>
				<input
					id="users-recherche"
					type="search"
					className="admin-search"
					placeholder="Pseudo ou adresse e-mail"
					autoComplete="off"
					autoCapitalize="none"
					spellCheck={false}
					value={search}
					onChange={(event) => {
						setSearch(event.target.value);
						setPageNumber(1);
					}}
				/>
				<div className="admin-filter">
					<label htmlFor="users-role" className="admin-filter-label">
						Rôle
					</label>
					<select
						id="users-role"
						className="admin-select"
						value={role}
						onChange={(event) => {
							const chosen = roleOptions.find(
								(option) => option.value === event.target.value,
							);
							setRoleFilter(chosen?.value ?? "");
							setPageNumber(1);
						}}
					>
						{roleOptions.map((option) => (
							<option key={option.value} value={option.value}>
								{option.label}
							</option>
						))}
					</select>
				</div>
			</div>

			{toast ? (
				<p role="status" className="admin-toast">
					{toast}
				</p>
			) : null}
			{error ? (
				<p role="alert" className="form-error">
					{error}
				</p>
			) : null}

			<div className="admin-columns users-columns">
				<div className="admin-table-card">
					{listing.state === "failed" ? (
						<p role="alert" className="form-error admin-table-message">
							Impossible de charger les comptes. Réessaie dans un moment.
						</p>
					) : null}
					{listing.state === "loading" ? (
						<p className="admin-table-message">Chargement…</p>
					) : null}
					{page ? (
						<UsersTable
							viewer={viewer}
							users={page.items}
							onRoleChange={changeRole}
							onReset={(target) => setPending({ kind: "reset", target })}
							onDelete={(target) => setPending({ kind: "delete", target })}
						/>
					) : null}
				</div>
				<RolesHelp />
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

			{pending?.kind === "reset" ? (
				<ResetPasswordDialog
					target={pending.target}
					onFailure={onFailure}
					onClose={(done) => {
						setPending(undefined);
						if (done) {
							reload(
								`Le mot de passe de ${pending.target.username} a été réinitialisé.`,
							);
						}
					}}
				/>
			) : null}
			{pending?.kind === "delete" ? (
				<DeleteUserDialog
					target={pending.target}
					onConfirm={() => confirmDelete(pending.target)}
					onFailure={onFailure}
					onClose={() => setPending(undefined)}
				/>
			) : null}
		</div>
	);
}

interface UsersTableProps {
	viewer: User;
	users: ListedUser[];
	onRoleChange: (target: ListedUser, role: "user" | "admin") => void;
	onReset: (target: ListedUser) => void;
	onDelete: (target: ListedUser) => void;
}

function UsersTable({
	viewer,
	users,
	onRoleChange,
	onReset,
	onDelete,
}: UsersTableProps) {
	if (users.length === 0) {
		return <p className="admin-table-message">Aucun compte ne correspond.</p>;
	}
	return (
		<div className="admin-table-scroll">
			<table className="admin-table">
				<thead>
					<tr>
						<th scope="col">Pseudo</th>
						<th scope="col">Adresse e-mail</th>
						<th scope="col">Rôle</th>
						<th scope="col">Inscription</th>
						<th scope="col">Dernière activité</th>
						<th scope="col">Actions</th>
					</tr>
				</thead>
				<tbody>
					{users.map((target) => (
						<tr
							key={target.id}
							className={
								target.role === "super_admin" ? "admin-row-locked" : undefined
							}
						>
							<th scope="row" className="admin-cell-username">
								{target.username}
								{target.id === viewer.id ? (
									<span className="admin-me">toi</span>
								) : null}
							</th>
							<td className="admin-cell-muted">{target.email ?? "—"}</td>
							<td>
								<RoleCell
									viewer={viewer}
									target={target}
									onChange={(role) => onRoleChange(target, role)}
								/>
							</td>
							<td className="admin-cell-muted">
								{formatDay(target.createdAt)}
							</td>
							<td className="admin-cell-muted">
								{formatLastActivity(target.lastActiveAt)}
							</td>
							<td>
								{canManage(viewer, target) ? (
									<div className="admin-actions">
										<button
											type="button"
											className="button-small"
											aria-label={`Réinitialiser le mot de passe de ${target.username}`}
											onClick={() => onReset(target)}
										>
											Réinitialiser le mot de passe
										</button>
										<button
											type="button"
											className="button-small button-danger"
											aria-label={`Supprimer ${target.username}`}
											onClick={() => onDelete(target)}
										>
											Supprimer
										</button>
									</div>
								) : (
									<span className="admin-cell-muted">Verrouillé</span>
								)}
							</td>
						</tr>
					))}
				</tbody>
			</table>
		</div>
	);
}

function LockIcon() {
	return (
		<svg
			width="14"
			height="14"
			viewBox="0 0 24 24"
			fill="none"
			stroke="currentColor"
			strokeWidth="2.6"
			strokeLinecap="round"
			strokeLinejoin="round"
			aria-hidden="true"
		>
			<rect x="5" y="11" width="14" height="10" rx="2.5" />
			<path d="M8 11V8a4 4 0 0 1 8 0v3" />
		</svg>
	);
}

interface RoleCellProps {
	viewer: User;
	target: ListedUser;
	onChange: (role: "user" | "admin") => void;
}

function RoleCell({ viewer, target, onChange }: RoleCellProps) {
	if (target.role === "super_admin") {
		return (
			<span
				className="role-badge role-badge-super_admin"
				title="Défini au démarrage du serveur, non modifiable ici"
			>
				<LockIcon />
				super admin · verrouillé
			</span>
		);
	}
	if (!canChangeRole(viewer, target)) {
		return (
			<span className={`role-badge role-badge-${target.role}`}>
				{roleLabels[target.role]}
			</span>
		);
	}
	const id = `role-${target.id}`;
	return (
		<>
			<label htmlFor={id} className="visually-hidden">
				Rôle de {target.username}
			</label>
			<select
				id={id}
				className={`admin-select role-select role-select-${target.role}`}
				value={target.role}
				onChange={(event) =>
					onChange(event.target.value === "admin" ? "admin" : "user")
				}
			>
				<option value="user">{roleLabels.user}</option>
				<option value="admin">{roleLabels.admin}</option>
			</select>
		</>
	);
}

function RolesHelp() {
	return (
		<section aria-labelledby="roles-titre" className="admin-help">
			<h2 id="roles-titre" className="admin-help-title">
				Les rôles
			</h2>
			<div className="admin-help-item">
				<span className="role-badge role-badge-user">user</span>
				<span>
					Suit les parcours et joue. Rôle de toute nouvelle inscription.
				</span>
			</div>
			<div className="admin-help-item">
				<span className="role-badge role-badge-admin">admin</span>
				<span>
					Ouvre l'administration. Réinitialise le mot de passe des users ou
					supprime leur compte.
				</span>
			</div>
			<div className="admin-help-item">
				<span className="role-badge role-badge-super_admin">super admin</span>
				<span>
					Créé au démarrage à partir de la configuration du serveur. Gère aussi
					les admins et change les rôles. Ne se modifie pas depuis l'interface.
				</span>
			</div>
		</section>
	);
}
