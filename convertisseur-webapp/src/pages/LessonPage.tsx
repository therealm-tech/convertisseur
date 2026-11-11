import {
	type ErrorComponentProps,
	Link,
	useLoaderData,
	useParams,
} from "@tanstack/react-router";
import { useEffect, useMemo, useState } from "react";
import { Arabic } from "@/components/Arabic";
import { BackIcon, CloseIcon } from "@/components/icons";
import { LetterFormsRow } from "@/components/LetterFormsRow";
import { PlayButton } from "@/components/PlayButton";
import { ApiError, problemTypes } from "@/lib/api";
import { countOf, varietyLabels } from "@/lib/contentLabels";
import { recordCardView } from "@/lib/progress";
import { hasQuiz, PASS_PERCENT } from "@/lib/quiz";
import {
	type ExerciseResult,
	isPassed,
	type StudyCard,
	type StudyLesson,
} from "@/lib/tracks";
import { ErrorPage } from "@/pages/ErrorPage";

function Answer({ card }: { card: StudyCard }) {
	if (card.transliteration === null && card.meaningFr === null) {
		return (
			<span className="study-answer">
				<span className="study-meaning">
					Pas de traduction pour cette carte.
				</span>
			</span>
		);
	}
	return (
		<span className="study-answer">
			{card.transliteration !== null ? (
				<span className="study-transliteration">{card.transliteration}</span>
			) : null}
			{card.meaningFr !== null ? (
				<span className="study-meaning">{card.meaningFr}</span>
			) : null}
		</span>
	);
}

function Card({
	card,
	revealed,
	onToggle,
}: {
	card: StudyCard;
	revealed: boolean;
	onToggle: () => void;
}) {
	return (
		<article className="study-card">
			<div className="study-card-top">
				<span className="variety-pill">{varietyLabels[card.variety]}</span>
				{card.audio ? (
					<PlayButton key={card.id} url={card.audio.url} label="Écouter" />
				) : null}
			</div>
			<button
				type="button"
				className="study-face"
				aria-expanded={revealed}
				onClick={onToggle}
			>
				<span className="study-arabic">
					<Arabic>{card.arabic}</Arabic>
				</span>
				{revealed ? (
					<Answer card={card} />
				) : (
					<span className="study-hint">Voir la traduction</span>
				)}
			</button>
			{card.forms ? (
				<LetterFormsRow arabic={card.arabic} forms={card.forms} />
			) : null}
		</article>
	);
}

function ExerciseLink({
	lesson,
	className,
}: {
	lesson: StudyLesson;
	className: string;
}) {
	return (
		<Link
			to="/tracks/$track/lessons/$lessonId/exercise"
			params={{ track: lesson.track, lessonId: lesson.id }}
			className={className}
		>
			S'entraîner
		</Link>
	);
}

function ExerciseChip({ exercise }: { exercise: ExerciseResult | null }) {
	if (!exercise) {
		return null;
	}
	const score = `${exercise.best.firstTry}/${exercise.best.total}`;
	return isPassed(exercise) ? (
		<span className="chip chip-passed">Réussi · {score}</span>
	) : (
		<span className="chip">Meilleur score · {score}</span>
	);
}

function Finished({
	lesson,
	quiz,
	onRestart,
}: {
	lesson: StudyLesson;
	quiz: boolean;
	onRestart: () => void;
}) {
	return (
		<section className="lesson-end" aria-labelledby="lesson-end-title">
			<h2 id="lesson-end-title" className="lesson-end-title">
				Leçon terminée
			</h2>
			<p className="lesson-end-text">
				Tu as parcouru les {countOf(lesson.cards.length, "carte", "cartes")} de
				« {lesson.title} ».
			</p>
			{quiz && !isPassed(lesson.exercise) ? (
				<p className="lesson-end-text">
					Réussis l'exercice à {PASS_PERCENT}&#8239;% pour ouvrir la leçon
					suivante.
				</p>
			) : null}
			{quiz ? (
				<ExerciseLink lesson={lesson} className="button-primary" />
			) : null}
			<button type="button" className="button-secondary" onClick={onRestart}>
				Revoir les cartes
			</button>
			<Link
				to="/tracks/$track"
				params={{ track: lesson.track }}
				className={quiz ? "button-secondary" : "button-primary"}
			>
				Retour au parcours
			</Link>
		</section>
	);
}

export function LessonPage() {
	const lesson = useLoaderData({
		from: "/_app/tracks/$track/lessons/$lessonId",
	});
	const quiz = useMemo(() => hasQuiz(lesson.cards), [lesson.cards]);
	const [resumeAt] = useState(() => {
		const unseen = lesson.cards.findIndex((card) => !card.seen);
		return Math.max(unseen, 0);
	});
	const [index, setIndex] = useState(resumeAt);
	const [resumed, setResumed] = useState(resumeAt > 0);
	const [seen, setSeen] = useState<ReadonlySet<string>>(
		() => new Set(lesson.cards.filter((c) => c.seen).map((c) => c.id)),
	);
	const [revealed, setRevealed] = useState(false);
	const [finished, setFinished] = useState(false);
	const total = lesson.cards.length;
	const card = lesson.cards[index];
	const shownId = finished ? undefined : card?.id;

	useEffect(() => {
		if (shownId === undefined) {
			return;
		}
		recordCardView(shownId);
		setSeen((ids) => (ids.has(shownId) ? ids : new Set(ids).add(shownId)));
	}, [shownId]);

	function goTo(next: number) {
		setResumed(false);
		if (next >= total) {
			setFinished(true);
			return;
		}
		setIndex(Math.max(0, next));
		setRevealed(false);
	}

	useEffect(() => {
		if (finished || total === 0) {
			return;
		}
		function onKey(event: KeyboardEvent) {
			if (event.key === "ArrowRight") {
				goTo(index + 1);
			} else if (event.key === "ArrowLeft") {
				goTo(index - 1);
			}
		}
		window.addEventListener("keydown", onKey);
		return () => window.removeEventListener("keydown", onKey);
	});

	return (
		<main className="lesson">
			<header className="lesson-header">
				<Link
					to="/tracks/$track"
					params={{ track: lesson.track }}
					className="icon-link"
					aria-label="Quitter la leçon"
				>
					<CloseIcon />
				</Link>
				{total > 0 ? (
					<>
						<div
							className="lesson-progress"
							role="progressbar"
							aria-label="Cartes vues"
							aria-valuemin={1}
							aria-valuemax={total}
							aria-valuenow={finished ? total : index + 1}
							aria-valuetext={`Carte ${finished ? total : index + 1} sur ${total}`}
						>
							<span
								className="lesson-progress-bar"
								style={{
									width: `${((finished ? total : index + 1) / total) * 100}%`,
								}}
							/>
						</div>
						<span className="lesson-count" aria-hidden="true">
							{finished ? total : index + 1}/{total}
						</span>
					</>
				) : null}
			</header>

			<div className="lesson-intro">
				<p className="lesson-unit">{lesson.unit.title}</p>
				<h1 className="lesson-title">{lesson.title}</h1>
				{lesson.audio ? (
					<PlayButton url={lesson.audio.url} label="Écouter la leçon" />
				) : null}
				{total > 0 && !finished ? (
					<div className="lesson-status">
						<span className="chip">
							{seen.size >= total
								? "Terminée"
								: `${seen.size}/${total} cartes vues`}
						</span>
						<ExerciseChip exercise={lesson.exercise} />
						{quiz ? (
							<ExerciseLink
								lesson={lesson}
								className="button-secondary lesson-exercise"
							/>
						) : null}
					</div>
				) : null}
			</div>
			{resumed ? (
				<p className="resume-note">
					On reprend à la carte {resumeAt + 1}, la première que tu n'as pas
					encore vue.{" "}
					<button type="button" className="link-button" onClick={() => goTo(0)}>
						Recommencer au début
					</button>
				</p>
			) : null}

			{total === 0 ? (
				<div className="empty-state">
					<p className="empty-state-title">Pas encore de carte</p>
					<p className="empty-note">
						Les cartes de cette leçon apparaîtront ici dès qu'elles auront été
						relues et validées.
					</p>
					<Link
						to="/tracks/$track"
						params={{ track: lesson.track }}
						className="button-primary"
					>
						Retour au parcours
					</Link>
				</div>
			) : finished || !card ? (
				<Finished
					lesson={lesson}
					quiz={quiz}
					onRestart={() => {
						setFinished(false);
						setIndex(0);
						setRevealed(false);
					}}
				/>
			) : (
				<>
					<p className="visually-hidden" aria-live="polite">
						Carte {index + 1} sur {total}
					</p>
					<Card
						card={card}
						revealed={revealed}
						onToggle={() => setRevealed((shown) => !shown)}
					/>
					<nav className="lesson-nav" aria-label="Cartes">
						<button
							type="button"
							className="button-secondary lesson-previous"
							disabled={index === 0}
							onClick={() => goTo(index - 1)}
						>
							<BackIcon />
							Précédente
						</button>
						<button
							type="button"
							className="button-primary lesson-next"
							onClick={() => goTo(index + 1)}
						>
							{index + 1 === total ? "Terminer" : "Suivante"}
						</button>
					</nav>
				</>
			)}
		</main>
	);
}

export function LessonNotFound() {
	return (
		<main className="landing">
			<h1 className="landing-title">Leçon introuvable</h1>
			<p className="landing-tagline">
				Cette leçon n'existe pas ou n'est plus disponible.
			</p>
			<Link to="/" className="button-primary">
				Retour à l'accueil
			</Link>
		</main>
	);
}

/** A lesson, or its exercise, the learner has not opened yet. */
export function LessonError({ error }: ErrorComponentProps) {
	const { track } = useParams({ strict: false });
	if (
		!(error instanceof ApiError) ||
		error.type !== problemTypes.lessonLocked ||
		track === undefined
	) {
		return <ErrorPage />;
	}
	return (
		<main className="landing">
			<h1 className="landing-title">Leçon verrouillée</h1>
			<p className="landing-tagline">
				Réussis l'exercice de la leçon d'avant pour l'ouvrir.
			</p>
			<Link to="/tracks/$track" params={{ track }} className="button-primary">
				Retour au parcours
			</Link>
		</main>
	);
}
