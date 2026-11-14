import { Link, useLoaderData } from "@tanstack/react-router";
import { useEffect, useRef, useState } from "react";
import { Arabic } from "@/components/Arabic";
import { CloseIcon } from "@/components/icons";
import { PlayButton } from "@/components/PlayButton";
import { recordAttempt } from "@/lib/progress";
import {
	buildQuiz,
	type Grade,
	gradeQuiz,
	PASS_PERCENT,
	type Question,
	type QuestionKind,
	requeue,
} from "@/lib/quiz";
import { type Attempt, isPassed, type StudyLesson } from "@/lib/tracks";

const instructions: Record<QuestionKind, string> = {
	meaning: "Quel est le sens ?",
	understand: "Quel est le sens ?",
	arabic: "Comment dit-on ?",
	form: "Quelle est cette lettre ?",
	listen: "Qu'entends-tu ?",
};

const letterInstructions: Record<QuestionKind, string> = {
	meaning: "Quel son ?",
	understand: "Quel son ?",
	arabic: "Quelle lettre ?",
	form: "Quelle est cette lettre ?",
	listen: "Quelle lettre entends-tu ?",
};

/** Whether a question's choices are Arabic rather than meanings. */
function choicesAreArabic(kind: QuestionKind): boolean {
	return kind !== "meaning" && kind !== "understand";
}

function promptIsRecording(kind: QuestionKind): boolean {
	return kind === "listen" || kind === "understand";
}

function Text({ arabic, children }: { arabic: boolean; children: string }) {
	return arabic ? <Arabic>{children}</Arabic> : children;
}

function choiceClass(
	question: Question,
	choice: string,
	picked: string | null,
): string {
	if (picked === null) {
		return "quiz-choice";
	}
	if (choice === question.answer) {
		return "quiz-choice quiz-choice-right";
	}
	return choice === picked ? "quiz-choice quiz-choice-wrong" : "quiz-choice";
}

function QuestionCard({
	question,
	picked,
	onPick,
}: {
	question: Question;
	picked: string | null;
	onPick: (choice: string) => void;
}) {
	const heading = useRef<HTMLHeadingElement>(null);
	const arabicChoices = choicesAreArabic(question.kind);
	const recording = promptIsRecording(question.kind);

	useEffect(() => {
		heading.current?.focus();
	}, []);

	return (
		<section className="study-card quiz-card" aria-labelledby="quiz-question">
			<h2
				id="quiz-question"
				ref={heading}
				tabIndex={-1}
				className="quiz-question"
			>
				{(question.ofLetter ? letterInstructions : instructions)[question.kind]}
			</h2>
			<p
				className={
					arabicChoices || recording
						? "quiz-prompt"
						: "quiz-prompt quiz-prompt-arabic"
				}
			>
				{recording ? (
					<PlayButton url={question.prompt} label="Écouter" />
				) : (
					<Text arabic={!arabicChoices}>{question.prompt}</Text>
				)}
			</p>
			<fieldset
				aria-labelledby="quiz-question"
				className={
					arabicChoices ? "quiz-choices quiz-choices-arabic" : "quiz-choices"
				}
			>
				{question.choices.map((choice) => (
					<button
						key={choice}
						type="button"
						className={choiceClass(question, choice, picked)}
						disabled={picked !== null}
						onClick={() => onPick(choice)}
					>
						<Text arabic={arabicChoices}>{choice}</Text>
						{picked !== null && choice === question.answer ? (
							<span className="visually-hidden"> (bonne réponse)</span>
						) : null}
						{picked === choice && choice !== question.answer ? (
							<span className="visually-hidden"> (ta réponse)</span>
						) : null}
					</button>
				))}
			</fieldset>
		</section>
	);
}

function Feedback({
	right,
	last,
	onContinue,
}: {
	right: boolean;
	last: boolean;
	onContinue: () => void;
}) {
	const button = useRef<HTMLButtonElement>(null);

	useEffect(() => {
		button.current?.focus();
	}, []);

	return (
		<div
			className={right ? "quiz-feedback" : "quiz-feedback quiz-feedback-wrong"}
		>
			<p className="quiz-feedback-text">
				{right ? "Bonne réponse !" : "Pas tout à fait, elle reviendra."}
			</p>
			<button
				ref={button}
				type="button"
				className="button-primary"
				onClick={onContinue}
			>
				{last ? "Terminer" : "Continuer"}
			</button>
		</div>
	);
}

function Finished({
	lesson,
	grade,
	firstTry,
	total,
	onRestart,
}: {
	lesson: StudyLesson;
	grade: Grade;
	firstTry: number;
	total: number;
	onRestart: () => void;
}) {
	const wasPassed = isPassed(lesson.exercise);
	const [saved, setSaved] = useState<Attempt | null | undefined>(undefined);
	const sent = useRef(false);

	useEffect(() => {
		if (sent.current) {
			return;
		}
		sent.current = true;
		void recordAttempt(lesson.id, firstTry, total).then(setSaved);
	}, [lesson.id, firstTry, total]);

	const best = saved?.exercise.best;
	const next = saved?.nextLesson;
	return (
		<section className="lesson-end" aria-labelledby="quiz-end-title">
			<h2 id="quiz-end-title" className="lesson-end-title">
				{grade.passed ? "Exercice réussi" : "Exercice à refaire"}
			</h2>
			<p className="quiz-grade">{grade.percent}&#8239;%</p>
			<p className="lesson-end-text">
				{firstTry} sur {total} du premier coup
			</p>
			{grade.passed ? null : (
				<p className="lesson-end-text">
					Il faut {PASS_PERCENT}&#8239;% pour réussir.
				</p>
			)}
			<div aria-live="polite" className="quiz-saved">
				{saved === null ? (
					<p className="lesson-end-text">
						Ton score n'est pas encore enregistré : il le sera dès que la
						connexion reviendra.
					</p>
				) : null}
				{next && !wasPassed && grade.passed ? (
					<span className="chip chip-passed">
						Leçon suivante ouverte : « {next.title} »
					</span>
				) : null}
				{best && (best.firstTry !== firstTry || best.total !== total) ? (
					<span className="chip">
						Ton meilleur score : {best.firstTry}/{best.total}
					</span>
				) : null}
			</div>
			<button
				type="button"
				className={grade.passed ? "button-secondary" : "button-primary"}
				onClick={onRestart}
			>
				Recommencer
			</button>
			{next ? (
				<Link
					to="/tracks/$track/lessons/$lessonId"
					params={{ track: lesson.track, lessonId: next.id }}
					className={grade.passed ? "button-primary" : "button-secondary"}
				>
					Leçon suivante
				</Link>
			) : null}
			<Link
				to="/tracks/$track"
				params={{ track: lesson.track }}
				className={
					grade.passed && !next ? "button-primary" : "button-secondary"
				}
			>
				Retour au parcours
			</Link>
		</section>
	);
}

function questionKey(question: Question): string {
	return `${question.id}:${question.kind}`;
}

/**
 * Asks the questions in turn; one answered wrong comes back near the end,
 * with its choices reshuffled, until it is answered right.
 */
function Quiz({
	lesson,
	questions,
	onRestart,
}: {
	lesson: StudyLesson;
	questions: Question[];
	onRestart: () => void;
}) {
	const [queue, setQueue] = useState(questions);
	const [picked, setPicked] = useState<string | null>(null);
	const [missed, setMissed] = useState<ReadonlySet<string>>(new Set());
	const [turn, setTurn] = useState(0);
	const total = questions.length;
	const current = queue[0];
	const done = total - queue.length;

	function pick(choice: string) {
		if (!current || picked !== null) {
			return;
		}
		setPicked(choice);
		if (choice !== current.answer) {
			setMissed((keys) => new Set(keys).add(questionKey(current)));
		}
	}

	function next() {
		if (!current) {
			return;
		}
		const rest = queue.slice(1);
		setQueue(picked === current.answer ? rest : requeue(rest, current));
		setPicked(null);
		setTurn((count) => count + 1);
	}

	return (
		<>
			<header className="lesson-header">
				<Link
					to="/tracks/$track"
					params={{ track: lesson.track }}
					className="icon-link"
					aria-label="Quitter l'exercice"
				>
					<CloseIcon />
				</Link>
				<div
					className="lesson-progress"
					role="progressbar"
					aria-label="Questions réussies"
					aria-valuemin={0}
					aria-valuemax={total}
					aria-valuenow={done}
					aria-valuetext={`${done} sur ${total}`}
				>
					<span
						className="lesson-progress-bar"
						style={{ width: `${(done / total) * 100}%` }}
					/>
				</div>
				<span className="lesson-count" aria-hidden="true">
					{done}/{total}
				</span>
			</header>

			<div className="lesson-intro">
				<p className="lesson-unit">Exercice</p>
				<h1 className="lesson-title">{lesson.title}</h1>
			</div>

			{current ? (
				<>
					<QuestionCard
						key={turn}
						question={current}
						picked={picked}
						onPick={pick}
					/>
					<div aria-live="polite" className="quiz-feedback-slot">
						{picked !== null ? (
							<Feedback
								right={picked === current.answer}
								last={queue.length === 1 && picked === current.answer}
								onContinue={next}
							/>
						) : null}
					</div>
				</>
			) : (
				<Finished
					lesson={lesson}
					grade={gradeQuiz(total, total - missed.size)}
					firstTry={total - missed.size}
					total={total}
					onRestart={onRestart}
				/>
			)}
		</>
	);
}

export function QuizPage() {
	const lesson = useLoaderData({
		from: "/_app/tracks/$track/lessons/$lessonId/exercise",
	});
	const [round, setRound] = useState(() => ({
		id: 0,
		questions: buildQuiz(lesson.cards),
	}));

	return (
		<main className="lesson">
			{round.questions.length > 0 ? (
				<Quiz
					key={round.id}
					lesson={lesson}
					questions={round.questions}
					onRestart={() =>
						setRound(({ id }) => ({
							id: id + 1,
							questions: buildQuiz(lesson.cards),
						}))
					}
				/>
			) : (
				<div className="empty-state">
					<p className="empty-state-title">Pas d'exercice pour cette leçon</p>
					<p className="empty-note">Il faut au moins trois cartes.</p>
					<Link
						to="/tracks/$track/lessons/$lessonId"
						params={{ track: lesson.track, lessonId: lesson.id }}
						className="button-primary"
					>
						Retour à la leçon
					</Link>
				</div>
			)}
		</main>
	);
}
