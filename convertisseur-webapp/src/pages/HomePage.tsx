import {
	Link,
	useLoaderData,
	useNavigate,
	useRouteContext,
} from "@tanstack/react-router";
import { useState } from "react";
import { Logo } from "@/components/Logo";
import { TrackGlyph } from "@/components/TrackGlyph";
import { logout } from "@/lib/api";
import { countOf, trackLabels } from "@/lib/contentLabels";
import { logger } from "@/lib/logger";
import { isStaff } from "@/lib/roles";
import { lessonCount, type Track } from "@/lib/tracks";

function TrackCard({ track }: { track: Track }) {
	const lessons = lessonCount(track);
	const first = track.units[0];
	return (
		<li>
			<Link
				to="/tracks/$track"
				params={{ track: track.id }}
				className={`track-card track-card-${track.id}`}
			>
				<TrackGlyph track={track.id} />
				<span className="track-card-name">{trackLabels[track.id]}</span>
				{first ? (
					<>
						<span className="track-card-detail">Unité 1 · {first.title}</span>
						<span className="track-card-count">
							{countOf(track.units.length, "unité", "unités")} ·{" "}
							{countOf(lessons, "leçon", "leçons")}
						</span>
					</>
				) : (
					<span className="track-card-detail">Bientôt disponible</span>
				)}
			</Link>
		</li>
	);
}

export function HomePage() {
	const { user } = useRouteContext({ from: "/_app/" });
	const tracks = useLoaderData({ from: "/_app/" });
	const navigate = useNavigate();
	const [failed, setFailed] = useState(false);
	const empty = tracks.every((track) => track.units.length === 0);

	async function onLogout() {
		setFailed(false);
		try {
			await logout();
			await navigate({ to: "/login" });
		} catch (error) {
			logger.warn("auth.logout_failed", { error });
			setFailed(true);
		}
	}

	return (
		<main className="home">
			<header className="home-brand">
				<Logo size={40} />
				<span className="home-brand-name">Convertisseur</span>
			</header>

			<section className="home-greeting">
				<h1 className="home-title">Salam, {user.username}</h1>
				<p className="home-lead">
					Choisis un parcours, puis une leçon à étudier.
				</p>
			</section>

			<section className="home-tracks" aria-labelledby="home-tracks-title">
				<h2 id="home-tracks-title" className="home-section-title">
					Tes parcours
				</h2>
				<ul className="track-cards">
					{tracks.map((track) => (
						<TrackCard key={track.id} track={track} />
					))}
				</ul>
				{empty ? (
					<p className="empty-note">
						Aucune leçon n'est encore disponible. Les leçons apparaîtront ici
						dès qu'elles auront été relues et validées.
					</p>
				) : null}
			</section>

			<div className="home-account">
				{failed ? (
					<p role="alert" className="form-error">
						La déconnexion a échoué. Réessaie dans un moment.
					</p>
				) : null}
				<nav aria-label="Mon compte" className="home-links">
					{isStaff(user.role) ? (
						<Link to="/admin/content" className="home-link">
							Administration
						</Link>
					) : null}
					<Link to="/password" className="home-link">
						Changer mon mot de passe
					</Link>
				</nav>
				<button type="button" className="button-secondary" onClick={onLogout}>
					Se déconnecter
				</button>
			</div>
		</main>
	);
}
