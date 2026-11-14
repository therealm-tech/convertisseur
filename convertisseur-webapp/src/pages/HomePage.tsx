import { Link, useLoaderData, useRouteContext } from "@tanstack/react-router";
import { useState } from "react";
import { Logo } from "@/components/Logo";
import { TrackGlyph } from "@/components/TrackGlyph";
import { countOf, trackLabels } from "@/lib/contentLabels";
import {
	completedCount,
	isStarted,
	type NextStep,
	nextStep,
	type StepKind,
} from "@/lib/progress";
import { PASS_PERCENT } from "@/lib/quiz";
import { isStaff } from "@/lib/roles";
import { lessonCount, type Track } from "@/lib/tracks";
import { LOGOUT_FAILED, useLogout } from "@/lib/useLogout";

const stepLabels: Record<StepKind, string> = {
	start: "Commencer",
	resume: "Reprendre",
	exercise: "Réussis l'exercice",
	next: "Leçon suivante",
};

function Meter({ value, max }: { value: number; max: number }) {
	return (
		<span className="meter" aria-hidden="true">
			<span style={{ width: `${max > 0 ? (value / max) * 100 : 0}%` }} />
		</span>
	);
}

function StepDetail({ step }: { step: NextStep }) {
	const { lesson } = step;
	if (step.kind === "resume") {
		return (
			<>
				<Meter value={lesson.progress.seenCardCount} max={lesson.cardCount} />
				<span className="resume-meta">
					{countOf(lesson.progress.seenCardCount, "carte vue", "cartes vues")}{" "}
					sur {lesson.cardCount}
				</span>
			</>
		);
	}
	if (step.kind === "exercise") {
		return (
			<span className="resume-meta">
				Il faut {PASS_PERCENT}&#8239;% pour ouvrir la leçon suivante.
			</span>
		);
	}
	return (
		<span className="resume-meta">
			{countOf(lesson.cardCount, "carte", "cartes")}
		</span>
	);
}

function ResumeCard({ step }: { step: NextStep }) {
	const where = `${trackLabels[step.track]} · Unité ${step.unitNumber} · Leçon ${step.lessonNumber}`;
	const params = { track: step.track, lessonId: step.lesson.id };
	return (
		<section className="resume-card" aria-labelledby="resume-title">
			<span className="resume-label">{stepLabels[step.kind]}</span>
			<span className="resume-where">{where}</span>
			<h2 id="resume-title" className="resume-title">
				{step.lesson.title}
			</h2>
			<StepDetail step={step} />
			{step.kind === "exercise" ? (
				<Link
					to="/tracks/$track/lessons/$lessonId/exercise"
					params={params}
					className="button-primary"
				>
					S'entraîner
				</Link>
			) : (
				<Link
					to="/tracks/$track/lessons/$lessonId"
					params={params}
					className="button-primary"
				>
					{step.kind === "resume" ? "Continuer" : "Commencer"}
				</Link>
			)}
		</section>
	);
}

function TrackCard({ track }: { track: Track }) {
	const lessons = lessonCount(track);
	const first = track.units[0];
	const completed = completedCount(track);
	return (
		<li>
			<Link
				to="/tracks/$track"
				params={{ track: track.id }}
				className={`track-card track-card-${track.id}`}
			>
				<TrackGlyph track={track.id} />
				<span className="track-card-name">{trackLabels[track.id]}</span>
				{first && isStarted(track) ? (
					<>
						<span className="track-card-detail">
							{countOf(completed, "leçon terminée", "leçons terminées")} sur{" "}
							{lessons}
						</span>
						<Meter value={completed} max={lessons} />
						<span className="track-card-count">
							{countOf(track.units.length, "unité", "unités")} ·{" "}
							{countOf(lessons, "leçon", "leçons")}
						</span>
					</>
				) : first ? (
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
	const [failed, setFailed] = useState(false);
	const empty = tracks.every((track) => track.units.length === 0);
	const step = nextStep(tracks);

	const logOut = useLogout(() => setFailed(true));

	async function onLogout() {
		setFailed(false);
		await logOut();
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

			{step ? <ResumeCard step={step} /> : null}

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
						{LOGOUT_FAILED}
					</p>
				) : null}
				<nav aria-label="Mon compte" className="home-links">
					{isStaff(user.role) ? (
						<Link to="/admin/content" className="home-link">
							Administration
						</Link>
					) : null}
					<Link to="/email" className="home-link">
						Changer mon adresse e-mail
					</Link>
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
