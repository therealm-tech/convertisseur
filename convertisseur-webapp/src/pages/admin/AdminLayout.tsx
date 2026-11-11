import {
	Link,
	Outlet,
	useMatchRoute,
	useRouteContext,
} from "@tanstack/react-router";
import { roleLabels } from "@/lib/roles";
import { DESKTOP_QUERY, useMediaQuery } from "@/lib/useMediaQuery";
import {
	ReviewTotalsProvider,
	useReviewTotals,
} from "@/pages/admin/reviewTotals";

function ReviewIcon() {
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
			<path d="M9 11.5 11.5 14 16 9.5" />
			<rect x="4" y="4" width="16" height="16" rx="3" />
		</svg>
	);
}

function ContentIcon() {
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
			<path d="M4 5.5A1.5 1.5 0 0 1 5.5 4H10l2 2h6.5A1.5 1.5 0 0 1 20 7.5v11a1.5 1.5 0 0 1-1.5 1.5h-13A1.5 1.5 0 0 1 4 18.5z" />
		</svg>
	);
}

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

/** The review entry, with how many items wait for a reviewer. */
function ReviewLink() {
	const { totals } = useReviewTotals();
	const count = totals?.itemsToReview;
	return (
		<Link
			to="/admin/relecture"
			className="admin-nav-link"
			aria-label={count ? `Relecture, ${count} à relire` : undefined}
		>
			<ReviewIcon />
			Relecture
			{count ? (
				<span className="admin-nav-count" aria-hidden="true">
					{count}
				</span>
			) : null}
		</Link>
	);
}

/** The admin area's frame: a top bar on a phone, a sidebar on a wide screen. */
export function AdminLayout() {
	return (
		<ReviewTotalsProvider>
			<AdminFrame />
		</ReviewTotalsProvider>
	);
}

function AdminFrame() {
	const { user } = useRouteContext({ from: "/_app/admin" });
	const matchRoute = useMatchRoute();
	const desktop = useMediaQuery(DESKTOP_QUERY);
	// On a phone, reviewing a lesson takes the whole screen; its close
	// button leads back to the queue.
	const fullScreen =
		!desktop &&
		Boolean(matchRoute({ to: "/admin/relecture/lecons/$lessonId" }));
	if (fullScreen) {
		return (
			<div className="admin">
				<main className="admin-main admin-main-full">
					<Outlet />
				</main>
			</div>
		);
	}
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
					<ReviewLink />
					<Link to="/admin/contenu" className="admin-nav-link">
						<ContentIcon />
						Contenu
					</Link>
					<Link
						to="/admin/utilisateurs"
						className="admin-nav-link"
						aria-label="Utilisateurs et rôles"
					>
						<UsersIcon />
						<span>
							Utilisateurs<span className="admin-nav-long"> et rôles</span>
						</span>
					</Link>
				</nav>
				<Link to="/" className="admin-back" activeOptions={{ exact: true }}>
					<BackIcon />
					Retour à l'app
				</Link>
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
