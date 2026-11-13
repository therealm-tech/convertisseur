import { Link, useLoaderData } from "@tanstack/react-router";
import { useEffect, useMemo, useState } from "react";
import { Arabic } from "@/components/Arabic";
import { BackIcon, CloseIcon } from "@/components/icons";
import { LetterFormsRow } from "@/components/LetterFormsRow";
import { PlayButton } from "@/components/PlayButton";
import { countOf, varietyLabels } from "@/lib/contentLabels";
import { hasQuiz } from "@/lib/quiz";
import type { StudyCard, StudyLesson } from "@/lib/tracks";

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
	const [index, setIndex] = useState(0);
	const [revealed, setRevealed] = useState(false);
	const [finished, setFinished] = useState(false);
	const quiz = useMemo(() => hasQuiz(lesson.cards), [lesson.cards]);
	const total = lesson.cards.length;
	const card = lesson.cards[index];

	function goTo(next: number) {
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
				{lesson.audio || (quiz && !finished) ? (
					<div className="lesson-actions">
						{lesson.audio ? (
							<PlayButton url={lesson.audio.url} label="Écouter la leçon" />
						) : null}
						{quiz && !finished ? (
							<ExerciseLink
								lesson={lesson}
								className="button-secondary lesson-exercise"
							/>
						) : null}
					</div>
				) : null}
			</div>

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
