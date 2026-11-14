import type { StudyCard } from "@/lib/tracks";

/**
 * `meaning`: the Arabic, pick its meaning. `understand`: the card's
 * recording, pick its meaning. `arabic`: the meaning, pick the Arabic.
 * `form`: a letter's initial, medial or final form, pick the isolated letter.
 * `listen`: the card's recording, pick the Arabic. The prompt of `understand`
 * and `listen` is the recording's URL.
 */
export type QuestionKind =
	| "meaning"
	| "understand"
	| "arabic"
	| "form"
	| "listen";

export interface Question {
	id: string;
	kind: QuestionKind;
	/** Whether the card is a letter of the alphabet, whose meaning is a sound. */
	ofLetter: boolean;
	prompt: string;
	choices: string[];
	answer: string;
}

export interface Grade {
	/** The share of questions answered right the first time, rounded down. */
	percent: number;
	passed: boolean;
}

export type Random = () => number;

export const QUIZ_LENGTH = 12;
export const PASS_PERCENT = 90;

const MAX_CHOICES = 4;
const MIN_CHOICES = 3;

export function shuffle<T>(items: readonly T[], random: Random): T[] {
	const pool = [...items];
	const shuffled: T[] = [];
	while (pool.length > 0) {
		shuffled.push(...pool.splice(Math.floor(random() * pool.length), 1));
	}
	return shuffled;
}

function pick<T>(items: readonly T[], random: Random): T | undefined {
	return items[Math.floor(random() * items.length)];
}

/**
 * The answer and up to three distractors drawn from the other cards, never
 * one that reads like the answer; null below three choices.
 */
function choicesFor(
	answer: string,
	others: readonly (string | null)[],
	random: Random,
): string[] | null {
	const distractors = [
		...new Set(
			others.filter(
				(other): other is string => other !== null && other !== answer,
			),
		),
	];
	const drawn = shuffle(distractors, random).slice(0, MAX_CHOICES - 1);
	if (drawn.length + 1 < MIN_CHOICES) {
		return null;
	}
	return shuffle([answer, ...drawn], random);
}

function letterForms(card: StudyCard): string[] {
	if (!card.forms) {
		return [];
	}
	return card.forms.kind === "joining"
		? [card.forms.initial, card.forms.medial, card.forms.final]
		: [card.forms.final];
}

/**
 * Every question the card can make, one per kind. A word with a recording is
 * asked its meaning from the recording rather than from the written Arabic; a
 * letter keeps the written one, its meaning being the sound it makes.
 */
function questionsFor(
	card: StudyCard,
	others: readonly StudyCard[],
	random: Random,
): Question[] {
	const questions: Question[] = [];
	const ofLetter = card.forms !== null;
	function ask(
		kind: QuestionKind,
		prompt: string,
		answer: string,
		distractors: readonly (string | null)[],
	) {
		const choices = choicesFor(answer, distractors, random);
		if (choices) {
			questions.push({ id: card.id, kind, ofLetter, prompt, choices, answer });
		}
	}

	if (card.meaningFr !== null) {
		const meanings = others
			.filter((other) => other.arabic !== card.arabic)
			.map((other) => other.meaningFr);
		if (card.audio !== null && !ofLetter) {
			ask("understand", card.audio.url, card.meaningFr, meanings);
		} else {
			ask("meaning", card.arabic, card.meaningFr, meanings);
		}
		ask(
			"arabic",
			card.meaningFr,
			card.arabic,
			others
				.filter((other) => other.meaningFr !== card.meaningFr)
				.map((other) => other.arabic),
		);
	}
	const form = pick(letterForms(card), random);
	if (form !== undefined) {
		ask(
			"form",
			form,
			card.arabic,
			others.filter((other) => other.forms).map((other) => other.arabic),
		);
	}
	if (card.audio !== null) {
		ask(
			"listen",
			card.audio.url,
			card.arabic,
			others.map((other) => other.arabic),
		);
	}
	return questions;
}

/**
 * Keeps the first of the questions that share an answer: `arabic`, `form` and `listen` all have
 * the card's Arabic for answer and the same distractors, so a second one
 * reads like the first asked again.
 */
function oneQuestionPerAnswer(questions: readonly Question[]): Question[] {
	const answers = new Set<string>();
	return questions.filter((question) => {
		if (answers.has(question.answer)) {
			return false;
		}
		answers.add(question.answer);
		return true;
	});
}

/**
 * Orders the questions so that two about one card never follow each other:
 * each turn takes, among the cards other than the last one asked, one with
 * the most questions left.
 */
function interleave(
	questions: readonly Question[],
	random: Random,
): Question[] {
	const left = new Map<string, Question[]>();
	for (const question of shuffle(questions, random)) {
		left.set(question.id, [...(left.get(question.id) ?? []), question]);
	}
	const ordered: Question[] = [];
	let last: string | undefined;
	while (left.size > 0) {
		const others = [...left].filter(([id]) => id !== last);
		const pool = others.length > 0 ? others : [...left];
		const most = Math.max(...pool.map(([, queued]) => queued.length));
		const next = pick(
			pool.filter(([, queued]) => queued.length === most),
			random,
		);
		if (!next) {
			break;
		}
		const [id, [question, ...rest]] = next;
		if (question) {
			ordered.push(question);
		}
		last = id;
		if (rest.length > 0) {
			left.set(id, rest);
		} else {
			left.delete(id);
		}
	}
	return ordered;
}

/**
 * Every card that can make a question asks one; then, one card at a time,
 * the cards ask the other kinds they allow until the quiz holds `length`
 * questions or the cards have nothing left. A card never asks two questions
 * with the same answer. Distractors come from the other cards of the lesson,
 * so a lesson of fewer than three cards makes none.
 */
export function buildQuiz(
	cards: readonly StudyCard[],
	random: Random = Math.random,
	length: number = QUIZ_LENGTH,
): Question[] {
	const perCard = shuffle(cards, random).map((card) =>
		oneQuestionPerAnswer(
			shuffle(
				questionsFor(
					card,
					cards.filter((other) => other.id !== card.id),
					random,
				),
				random,
			),
		),
	);
	const rounds = Math.max(0, ...perCard.map((questions) => questions.length));
	const chosen: Question[] = [];
	for (let round = 0; round < rounds; round++) {
		for (const questions of perCard) {
			const question = questions[round];
			if (question && (round === 0 || chosen.length < length)) {
				chosen.push(question);
			}
		}
	}
	return interleave(chosen, random);
}

/** Whether the cards make at least one question, whatever the draw. */
export function hasQuiz(cards: readonly StudyCard[]): boolean {
	return buildQuiz(cards).length > 0;
}

/**
 * The queue once a missed question goes back in, its choices reshuffled: at
 * the end, or as close to it as it can sit without a question about the same
 * card on either side.
 */
export function requeue(
	rest: readonly Question[],
	missed: Question,
	random: Random = Math.random,
): Question[] {
	const again = { ...missed, choices: shuffle(missed.choices, random) };
	for (let at = rest.length; at > 0; at--) {
		if (rest[at - 1]?.id !== missed.id && rest[at]?.id !== missed.id) {
			return [...rest.slice(0, at), again, ...rest.slice(at)];
		}
	}
	return [...rest, again];
}

/** `firstTry` of `total` questions answered right the first time. */
export function gradeQuiz(total: number, firstTry: number): Grade {
	if (total <= 0) {
		return { percent: 0, passed: false };
	}
	return {
		percent: Math.floor((firstTry * 100) / total),
		passed: firstTry * 100 >= PASS_PERCENT * total,
	};
}
