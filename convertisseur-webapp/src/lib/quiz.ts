import type { StudyCard } from "@/lib/tracks";

/**
 * `meaning`: the Arabic, pick its meaning. `arabic`: the meaning, pick the
 * Arabic. `form`: a letter's initial, medial or final form, pick the
 * isolated letter.
 */
export type QuestionKind = "meaning" | "arabic" | "form";

export interface Question {
	id: string;
	kind: QuestionKind;
	/** Whether the card is a letter of the alphabet, whose meaning is a sound. */
	ofLetter: boolean;
	prompt: string;
	choices: string[];
	answer: string;
}

export type Random = () => number;

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

function questionsFor(
	card: StudyCard,
	others: readonly StudyCard[],
	random: Random,
): Question[] {
	const candidates: Question[] = [];
	const ofLetter = card.forms !== null;
	if (card.meaningFr !== null) {
		const meanings = choicesFor(
			card.meaningFr,
			others
				.filter((other) => other.arabic !== card.arabic)
				.map((other) => other.meaningFr),
			random,
		);
		if (meanings) {
			candidates.push({
				id: card.id,
				kind: "meaning",
				ofLetter,
				prompt: card.arabic,
				choices: meanings,
				answer: card.meaningFr,
			});
		}
		const arabic = choicesFor(
			card.arabic,
			others
				.filter((other) => other.meaningFr !== card.meaningFr)
				.map((other) => other.arabic),
			random,
		);
		if (arabic) {
			candidates.push({
				id: card.id,
				kind: "arabic",
				ofLetter,
				prompt: card.meaningFr,
				choices: arabic,
				answer: card.arabic,
			});
		}
	}
	const form = pick(letterForms(card), random);
	if (form !== undefined) {
		const letters = choicesFor(
			card.arabic,
			others.filter((other) => other.forms).map((other) => other.arabic),
			random,
		);
		if (letters) {
			candidates.push({
				id: card.id,
				kind: "form",
				ofLetter,
				prompt: form,
				choices: letters,
				answer: card.arabic,
			});
		}
	}
	return candidates;
}

/**
 * One question per card that can make one, of a kind drawn among those it
 * allows, in a random order. Distractors come from the other cards of the
 * lesson, so a lesson of fewer than three cards makes none.
 */
export function buildQuiz(
	cards: readonly StudyCard[],
	random: Random = Math.random,
): Question[] {
	const questions = cards.flatMap((card) => {
		const others = cards.filter((other) => other.id !== card.id);
		const question = pick(questionsFor(card, others, random), random);
		return question ? [question] : [];
	});
	return shuffle(questions, random);
}

/** Whether the cards make at least one question, whatever the draw. */
export function hasQuiz(cards: readonly StudyCard[]): boolean {
	return buildQuiz(cards).length > 0;
}
