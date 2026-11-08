import { useNavigate, useRouteContext } from "@tanstack/react-router";
import { useState } from "react";
import { Logo } from "@/components/Logo";
import { logout } from "@/lib/api";
import { logger } from "@/lib/logger";

export function HomePage() {
	const { user } = useRouteContext({ from: "/" });
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
			<button type="button" className="button-secondary" onClick={onLogout}>
				Se déconnecter
			</button>
		</main>
	);
}
