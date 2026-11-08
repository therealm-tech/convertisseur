import { useRouter } from "@tanstack/react-router";

export function ErrorPage() {
	const router = useRouter();
	return (
		<main className="landing">
			<h1 className="landing-title">Oups</h1>
			<p className="landing-tagline">
				Impossible de joindre le serveur. Vérifie ta connexion.
			</p>
			<button
				type="button"
				className="button-primary"
				onClick={() => router.invalidate()}
			>
				Réessayer
			</button>
		</main>
	);
}
