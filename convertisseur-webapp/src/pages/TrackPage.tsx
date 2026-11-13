import { Link, useLoaderData } from "@tanstack/react-router";
import { BackIcon } from "@/components/icons";
import { LessonStar, type LessonStarState } from "@/components/TrackGlyph";
import { countOf, trackLabels, trackTaglines } from "@/lib/contentLabels";
import {
	isComplete,
	isPassed,
	type TrackId,
	type TrackLesson,
	type TrackUnit,
} from "@/lib/tracks";

function starState(lesson: TrackLesson): LessonStarState {
	if (lesson.locked) {
		return "locked";
	}
	if (isComplete(lesson)) {
		return "done";
	}
	return lesson.progress.seenCardCount > 0 ? "current" : "new";
}

function LessonStatus({
	lesson,
	blocker,
}: {
	lesson: TrackLesson;
	blocker: TrackLesson | undefined;
}) {
	const cards = countOf(lesson.cardCount, "carte", "cartes");
	if (lesson.locked) {
		return (
			<span className="path-status">
				{blocker
					? `Verrouillée · réussis l'exercice de « ${blocker.title} »`
					: "Verrouillée"}
			</span>
		);
	}
	if (isComplete(lesson)) {
		return (
			<span className="path-status path-status-done">Terminée · {cards}</span>
		);
	}
	const seen = lesson.progress.seenCardCount;
	if (seen > 0) {
		return (
			<>
				<span className="path-status path-status-current">
					En cours · {seen}/{lesson.cardCount} cartes
				</span>
				<span className="meter" aria-hidden="true">
					<span style={{ width: `${(seen / lesson.cardCount) * 100}%` }} />
				</span>
			</>
		);
	}
	return <span className="path-status">{cards}</span>;
}

function ExerciseLink({
	track,
	lesson,
}: {
	track: TrackId;
	lesson: TrackLesson;
}) {
	const exercise = lesson.progress.exercise;
	const passed = isPassed(exercise);
	const score = exercise
		? `${exercise.best.firstTry}/${exercise.best.total}`
		: "";
	const label = passed
		? `Réussi · ${score}`
		: exercise
			? `S'entraîner · ${score}`
			: "S'entraîner";
	return (
		<Link
			to="/tracks/$track/lessons/$lessonId/exercise"
			params={{ track, lessonId: lesson.id }}
			className={
				passed ? "path-exercise path-exercise-passed" : "path-exercise"
			}
			aria-label={`${label} : ${lesson.title}`}
		>
			{label}
		</Link>
	);
}

function UnitPath({
	track,
	unit,
	number,
	before,
}: {
	track: TrackId;
	unit: TrackUnit;
	number: number;
	/** The last lesson of the units before, on the track's path. */
	before: TrackLesson | undefined;
}) {
	const headingId = `unit-${unit.id}`;
	const done = unit.lessons.filter(isComplete).length;
	return (
		<section className="unit" aria-labelledby={headingId}>
			<div className="unit-banner">
				<span className="unit-number">
					Unité {number}
					{done > 0
						? ` · ${countOf(done, "leçon terminée", "leçons terminées")} sur ${unit.lessons.length}`
						: null}
				</span>
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
					{unit.lessons.map((lesson, index) => {
						const previous = index > 0 ? unit.lessons[index - 1] : before;
						const blocker =
							lesson.locked && previous && !previous.locked
								? previous
								: undefined;
						const text = (
							<>
								<LessonStar number={index + 1} state={starState(lesson)} />
								<span className="path-text">
									<span className="path-number">Leçon {index + 1}</span>
									<span className="path-title">{lesson.title}</span>
									<LessonStatus lesson={lesson} blocker={blocker} />
								</span>
							</>
						);
						return (
							<li key={lesson.id} className="path-step">
								{lesson.locked ? (
									<div className="path-link path-link-locked">{text}</div>
								) : (
									<Link
										to="/tracks/$track/lessons/$lessonId"
										params={{ track, lessonId: lesson.id }}
										className="path-link"
									>
										{text}
									</Link>
								)}
								{lesson.hasExercise && !lesson.locked ? (
									<ExerciseLink track={track} lesson={lesson} />
								) : null}
							</li>
						);
					})}
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
						before={track.units
							.slice(0, index)
							.flatMap((earlier) => earlier.lessons)
							.at(-1)}
					/>
				))
			)}
		</main>
	);
}
