import { Link } from "@tanstack/react-router";

export function NotFoundPage() {
	return (
		<main className="landing">
			<h1 className="landing-title">Page introuvable</h1>
			<p className="landing-tagline">Cette page n'existe pas.</p>
			<Link to="/" className="button-primary">
				Retour à l'accueil
			</Link>
		</main>
	);
}
