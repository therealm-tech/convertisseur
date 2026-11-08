import { Link, useNavigate, useRouteContext } from "@tanstack/react-router";
import { useState } from "react";
import { Logo } from "@/components/Logo";
import { logout } from "@/lib/api";
import { logger } from "@/lib/logger";
import { isStaff } from "@/lib/roles";

export function HomePage() {
	const { user } = useRouteContext({ from: "/_app/" });
	const navigate = useNavigate();
	const [failed, setFailed] = useState(false);

	async function onLogout() {
		setFailed(false);
		try {
			await logout();
			await navigate({ to: "/connexion" });
		} catch (error) {
			logger.warn("auth.logout_failed", { error });
			setFailed(true);
		}
	}

	return (
		<main className="landing">
			<Logo size={72} />
			<h1 className="landing-title">Salam, {user.username}</h1>
			{failed ? (
				<p role="alert" className="form-error">
					La déconnexion a échoué. Réessaie dans un moment.
				</p>
			) : null}
			<nav aria-label="Mon compte" className="home-links">
				{isStaff(user.role) ? (
					<Link to="/admin/utilisateurs" className="button-primary">
						Administration
					</Link>
				) : null}
				<Link to="/mot-de-passe" className="home-link">
					Changer mon mot de passe
				</Link>
			</nav>
			<button type="button" className="button-secondary" onClick={onLogout}>
				Se déconnecter
			</button>
		</main>
	);
}
