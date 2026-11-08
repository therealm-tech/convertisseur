import { Link, Outlet, useRouteContext } from "@tanstack/react-router";
import { roleLabels } from "@/lib/roles";

function UsersIcon() {
	return (
		<svg
			width="20"
			height="20"
			viewBox="0 0 24 24"
			fill="none"
			stroke="currentColor"
			strokeWidth="2"
			strokeLinecap="round"
			strokeLinejoin="round"
			aria-hidden="true"
		>
			<circle cx="9" cy="8.5" r="3.5" />
			<path d="M2.5 20a6.5 6.5 0 0 1 13 0" />
			<path d="M16 5a3.5 3.5 0 0 1 0 7M18 14.5a6.5 6.5 0 0 1 3.5 5.5" />
		</svg>
	);
}

function BackIcon() {
	return (
		<svg
			width="20"
			height="20"
			viewBox="0 0 24 24"
			fill="none"
			stroke="currentColor"
			strokeWidth="2"
			strokeLinecap="round"
			strokeLinejoin="round"
			aria-hidden="true"
		>
			<path d="m15 5-7 7 7 7" />
		</svg>
	);
}

/** The admin area's frame: the sidebar, and the page beside it. */
export function AdminLayout() {
	const { user } = useRouteContext({ from: "/_app/admin" });
	return (
		<div className="admin">
			<aside className="admin-sidebar">
				<div className="admin-brand">
					<img src="/logo.svg" alt="" width={40} height={40} />
					<div className="admin-brand-text">
						<span className="admin-brand-name">Convertisseur</span>
						<span className="admin-brand-area">Administration</span>
					</div>
				</div>
				<nav aria-label="Administration" className="admin-nav">
					<Link to="/admin/utilisateurs" className="admin-nav-link">
						<UsersIcon />
						Utilisateurs et rôles
					</Link>
					<Link
						to="/"
						className="admin-nav-link"
						activeOptions={{ exact: true }}
					>
						<BackIcon />
						Retour à l'app
					</Link>
				</nav>
				<div className="admin-whoami">
					<span className="admin-whoami-label">Connecté en tant que</span>
					<span className="admin-whoami-name">
						{user.username} · {roleLabels[user.role]}
					</span>
				</div>
			</aside>
			<main className="admin-main">
				<Outlet />
			</main>
		</div>
	);
}
