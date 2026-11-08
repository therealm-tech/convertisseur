import { Logo } from "@/components/Logo";

export function LandingPage() {
	return (
		<main className="landing">
			<Logo size={104} />
			<h1 className="landing-title">Convertisseur</h1>
			<p className="landing-tagline">
				L'arabe et les bases de l'Islam, un petit jeu à la fois.
			</p>
		</main>
	);
}
