import { Link, useLoaderData } from "@tanstack/react-router";
import { BackIcon } from "@/components/icons";
import { LessonStar } from "@/components/TrackGlyph";
import { countOf, trackLabels, trackTaglines } from "@/lib/contentLabels";
import { mayHaveQuiz } from "@/lib/quiz";
import type { TrackId, TrackUnit } from "@/lib/tracks";

function UnitPath({
	track,
	unit,
	number,
}: {
	track: TrackId;
	unit: TrackUnit;
	number: number;
}) {
	const headingId = `unit-${unit.id}`;
	return (
		<section className="unit" aria-labelledby={headingId}>
			<div className="unit-banner">
				<span className="unit-number">Unité {number}</span>
				<h2 id={headingId} className="unit-title">
					{unit.title}
				</h2>
			</div>
			{unit.lessons.length === 0 ? (
				<p className="empty-note">
					Les leçons de cette unité arrivent bientôt.
				</p>
			) : (
				<ol className="path">
					{unit.lessons.map((lesson, index) => (
						<li key={lesson.id} className="path-step">
							<Link
								to="/tracks/$track/lessons/$lessonId"
								params={{ track, lessonId: lesson.id }}
								className="path-link"
							>
								<LessonStar number={index + 1} />
								<span className="path-text">
									<span className="path-number">Leçon {index + 1}</span>
									<span className="path-title">{lesson.title}</span>
									<span className="path-count">
										{countOf(lesson.cardCount, "carte", "cartes")}
									</span>
								</span>
							</Link>
							{mayHaveQuiz(lesson.cardCount) ? (
								<Link
									to="/tracks/$track/lessons/$lessonId/exercise"
									params={{ track, lessonId: lesson.id }}
									className="path-exercise"
									aria-label={`S'entraîner : ${lesson.title}`}
								>
									S'entraîner
								</Link>
							) : null}
						</li>
					))}
				</ol>
			)}
		</section>
	);
}

export function TrackPage() {
	const track = useLoaderData({ from: "/_app/tracks/$track" });
	return (
		<main className="track">
			<header className="track-header">
				<Link to="/" className="icon-link" aria-label="Retour à l'accueil">
					<BackIcon />
				</Link>
				<div className="track-heading">
					<h1 className="track-title">Parcours {trackLabels[track.id]}</h1>
					<p className="track-tagline">{trackTaglines[track.id]}</p>
				</div>
			</header>
			{track.units.length === 0 ? (
				<div className="empty-state">
					<p className="empty-state-title">Pas encore de leçon ici</p>
					<p className="empty-note">
						Les leçons de ce parcours apparaîtront ici dès qu'elles auront été
						relues et validées.
					</p>
					<Link to="/" className="button-primary">
						Retour à l'accueil
					</Link>
				</div>
			) : (
				track.units.map((unit, index) => (
					<UnitPath
						key={unit.id}
						track={track.id}
						unit={unit}
						number={index + 1}
					/>
				))
			)}
		</main>
	);
}
