import { useNavigate } from "@tanstack/react-router";
import { logout } from "@/lib/api";
import { logger } from "@/lib/logger";

export const LOGOUT_FAILED =
	"La déconnexion a échoué. Réessaie dans un moment.";

/** Logs out and goes to the login screen; `onFailure` runs when it fails. */
export function useLogout(onFailure: () => void): () => Promise<void> {
	const navigate = useNavigate();
	return async () => {
		try {
			await logout();
			await navigate({ to: "/login" });
		} catch (error) {
			logger.warn("auth.logout_failed", { error });
			onFailure();
		}
	};
}
