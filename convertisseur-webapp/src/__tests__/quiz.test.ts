import { z } from "zod";
import {
	buildQuiz,
	gradeQuiz,
	hasQuiz,
	PASS_PERCENT,
	QUIZ_LENGTH,
	type Question,
	type Random,
	requeue,
} from "@/lib/quiz";
import type { StudyCard } from "@/lib/tracks";
import exerciseCases from "./exerciseCases.json";

function card(
	id: string,
	arabic: string,
	meaningFr: string | null,
	extra: Partial<StudyCard> = {},
): StudyCard {
	return {
		id,
		arabic,
		transliteration: null,
		meaningFr,
		variety: "darija",
		forms: null,
		audio: null,
		seen: false,
		...extra,
	};
}

/** A deterministic draw, so that a failure replays. */
function seeded(seed: number): Random {
	let state = seed;
	return () => {
		state = (state * 1_103_515_245 + 12_345) % 2_147_483_648;
		return state / 2_147_483_648;
	};
}

const words = [
	card("salam", "السلام عليكم", "Bonjour (la paix sur vous)"),
	card("sbah", "صباح الخير", "Bonjour (le matin)"),
	card("msa", "مسا الخير", "Bonsoir"),
	card("bslama", "بسلامة", "Au revoir"),
	card("saha", "صحة", "Salut !"),
];

const letters = [
	card("ba", "ب", "« b »", {
		forms: { kind: "joining", initial: "بـ", medial: "ـبـ", final: "ـب" },
	}),
	card("ta", "ت", "« t »", {
		forms: { kind: "joining", initial: "تـ", medial: "ـتـ", final: "ـت" },
	}),
	card("dal", "د", "« d »", { forms: { kind: "non_joining", final: "ـد" } }),
];

function idsOf(questions: readonly Question[]): string[] {
	return questions.map((question) => question.id);
}

function neverTwiceInARow(questions: readonly Question[]) {
	for (const [index, question] of questions.entries()) {
		expect(question.id).not.toBe(questions[index + 1]?.id);
	}
}

const recorded = (id: string) => ({
	audio: { url: `/api/v1/audio/${id}`, durationMs: 900 },
});

const manyWords = [
	...words,
	card("chokran", "شكرا", "Merci"),
	card("afak", "عافاك", "S'il te plaît"),
	card("labas", "لاباس", "Ça va"),
];

describe("quiz", () => {
	it("asks every question the cards allow, with the answer among distinct choices", () => {
		for (let seed = 1; seed <= 50; seed++) {
			const questions = buildQuiz(words, seeded(seed));

			expect(questions).toHaveLength(10);
			for (const word of words) {
				expect(
					questions
						.filter((question) => question.id === word.id)
						.map((question) => question.kind)
						.sort(),
				).toEqual(["arabic", "meaning"]);
			}
			for (const question of questions) {
				expect(question.choices).toContain(question.answer);
				expect(new Set(question.choices).size).toBe(question.choices.length);
				expect(question.choices).toHaveLength(4);
			}
		}
	});

	it("stops at twelve questions, every card asked at least once", () => {
		for (let seed = 1; seed <= 50; seed++) {
			const questions = buildQuiz(manyWords, seeded(seed));

			expect(questions).toHaveLength(QUIZ_LENGTH);
			expect(new Set(idsOf(questions))).toEqual(
				new Set(manyWords.map((word) => word.id)),
			);
		}
	});

	it("asks every card once when there are more cards than twelve", () => {
		const cards = Array.from({ length: 15 }, (_, index) =>
			card(`w${index}`, `كلمة${index}`, `Mot ${index}`),
		);

		const questions = buildQuiz(cards, seeded(3));

		expect(questions).toHaveLength(15);
		expect(new Set(idsOf(questions)).size).toBe(15);
	});

	it("never asks about the same card twice in a row", () => {
		for (let seed = 1; seed <= 100; seed++) {
			neverTwiceInARow(buildQuiz(words.slice(0, 3), seeded(seed)));
			neverTwiceInARow(buildQuiz(manyWords, seeded(seed)));
			neverTwiceInARow(buildQuiz(letters, seeded(seed)));
		}
	});

	it("asks to recognise a recording among the Arabic of the other cards", () => {
		const cards = [
			card("salam", "السلام عليكم", null, recorded("salam")),
			...words.slice(1),
		];

		for (let seed = 1; seed <= 20; seed++) {
			const listen = buildQuiz(cards, seeded(seed)).filter(
				(question) => question.kind === "listen",
			);

			expect(listen).toHaveLength(1);
			expect(listen[0]?.prompt).toBe("/api/v1/audio/salam");
			expect(listen[0]?.answer).toBe("السلام عليكم");
			for (const choice of listen[0]?.choices ?? []) {
				expect(cards.map((each) => each.arabic)).toContain(choice);
			}
		}
	});

	it("plays a recorded word for its meaning, instead of showing its Arabic", () => {
		const cards = [
			card("salam", "السلام عليكم", "Bonjour", recorded("salam")),
			...words.slice(1),
		];

		for (let seed = 1; seed <= 20; seed++) {
			const salam = buildQuiz(cards, seeded(seed)).filter(
				(question) => question.id === "salam",
			);

			expect(salam.map((question) => question.kind)).not.toContain("meaning");
			const understand = salam.filter(
				(question) => question.kind === "understand",
			);
			expect(understand).toHaveLength(1);
			expect(understand[0]?.prompt).toBe("/api/v1/audio/salam");
			expect(understand[0]?.answer).toBe("Bonjour");
		}
	});

	it("shows a recorded letter written, its meaning being its sound", () => {
		const cards = letters.map((letter) => ({
			...letter,
			...recorded(letter.id),
		}));

		for (let seed = 1; seed <= 20; seed++) {
			const kinds = buildQuiz(cards, seeded(seed)).map(
				(question) => question.kind,
			);

			expect(kinds).toContain("meaning");
			expect(kinds).not.toContain("understand");
		}
	});

	it("never asks one card twice for the same answer", () => {
		const alphabet = [
			...letters,
			card("tha", "ث", "« th »", {
				forms: { kind: "joining", initial: "ثـ", medial: "ـثـ", final: "ـث" },
			}),
			card("jim", "ج", "« dj »", {
				forms: { kind: "joining", initial: "جـ", medial: "ـجـ", final: "ـج" },
			}),
			card("dhal", "ذ", "« dh »", {
				forms: { kind: "non_joining", final: "ـذ" },
			}),
			card("ra", "ر", "« r »", { forms: { kind: "non_joining", final: "ـر" } }),
		].map((letter) => ({ ...letter, ...recorded(letter.id) }));

		for (let seed = 1; seed <= 50; seed++) {
			const questions = buildQuiz(alphabet, seeded(seed));

			expect(questions).toHaveLength(QUIZ_LENGTH);
			for (const letter of alphabet) {
				const answers = questions
					.filter((question) => question.id === letter.id)
					.map((question) => question.answer);
				expect(new Set(answers).size).toBe(answers.length);
			}
		}
	});

	it("asks the meaning of the Arabic, or the Arabic of the meaning", () => {
		const kinds = new Set(
			buildQuiz(words, seeded(7)).map((question) => question.kind),
		);
		const salam = (seed: number) =>
			buildQuiz(words, seeded(seed)).find(
				(question) => question.id === "salam",
			);

		expect(kinds).toEqual(new Set(["meaning", "arabic"]));
		for (let seed = 1; seed <= 20; seed++) {
			const question = salam(seed);
			if (question?.kind === "meaning") {
				expect(question.prompt).toBe("السلام عليكم");
				expect(question.answer).toBe("Bonjour (la paix sur vous)");
			} else {
				expect(question?.prompt).toBe("Bonjour (la paix sur vous)");
				expect(question?.answer).toBe("السلام عليكم");
			}
		}
	});

	it("asks a letter from one of its forms, among the other letters", () => {
		const asked = Array.from({ length: 40 }, (_, seed) =>
			buildQuiz(letters, seeded(seed + 1)),
		)
			.flat()
			.filter((question) => question.kind === "form");

		expect(asked.length).toBeGreaterThan(0);
		for (const question of asked) {
			const letter = letters.find((each) => each.id === question.id);
			expect(question.answer).toBe(letter?.arabic);
			expect(question.prompt).toContain(letter?.arabic);
			expect(question.prompt).toContain("ـ");
			expect(question.ofLetter).toBe(true);
			expect(new Set(question.choices)).toEqual(new Set(["ب", "ت", "د"]));
		}
	});

	it("draws no distractor that reads like the answer", () => {
		const twins = [
			card("a", "سلام", "Bonjour"),
			card("b", "صباح الخير", "Bonjour"),
			card("c", "بسلامة", "Au revoir"),
			card("d", "مسا الخير", "Bonsoir"),
		];

		for (let seed = 1; seed <= 50; seed++) {
			for (const question of buildQuiz(twins, seeded(seed))) {
				if (question.id === "a" && question.kind === "arabic") {
					expect(question.choices).not.toContain("صباح الخير");
				}
				expect(new Set(question.choices).size).toBe(question.choices.length);
			}
		}
	});

	it("leaves out a card with nothing to ask, and never offers a blank choice", () => {
		const cards = [...words.slice(0, 3), card("bare", "واش", null)];

		for (let seed = 1; seed <= 20; seed++) {
			const questions = buildQuiz(cards, seeded(seed));

			expect(questions.map((question) => question.id)).not.toContain("bare");
			for (const question of questions) {
				expect(question.choices).not.toContain(null);
			}
		}
	});

	it("needs three cards for the three choices of a question", () => {
		expect(hasQuiz(words.slice(0, 2))).toBe(false);
		expect(buildQuiz(words.slice(0, 2))).toEqual([]);
		expect(hasQuiz(words.slice(0, 3))).toBe(true);
		for (const question of buildQuiz(words.slice(0, 3))) {
			expect(question.choices).toHaveLength(3);
		}
	});
});

describe("requeue", () => {
	const ask = (id: string, kind: Question["kind"] = "meaning"): Question => ({
		id,
		kind,
		ofLetter: false,
		prompt: id,
		choices: ["a", "b", "c", "d"],
		answer: "a",
	});

	it("puts a missed question back at the end, its choices reshuffled", () => {
		const queue = requeue([ask("x"), ask("y")], ask("z"), seeded(4));

		expect(idsOf(queue)).toEqual(["x", "y", "z"]);
		expect(new Set(queue[2]?.choices)).toEqual(new Set(["a", "b", "c", "d"]));
	});

	it("keeps it away from a question about the same card", () => {
		const queue = requeue(
			[ask("x"), ask("y"), ask("z", "arabic")],
			ask("z"),
			seeded(4),
		);

		expect(idsOf(queue)).toEqual(["x", "z", "y", "z"]);
	});

	it("asks it again at once when nothing else is left", () => {
		expect(idsOf(requeue([], ask("z")))).toEqual(["z"]);
	});
});

describe("gradeQuiz", () => {
	it("passes from ninety percent answered right the first time", () => {
		expect(PASS_PERCENT).toBe(90);
		expect(gradeQuiz(12, 12)).toEqual({ percent: 100, passed: true });
		expect(gradeQuiz(12, 11)).toEqual({ percent: 91, passed: true });
		expect(gradeQuiz(12, 10)).toEqual({ percent: 83, passed: false });
		expect(gradeQuiz(10, 9)).toEqual({ percent: 90, passed: true });
		expect(gradeQuiz(6, 0)).toEqual({ percent: 0, passed: false });
	});

	it("never shows a near miss as ninety percent", () => {
		expect(gradeQuiz(29, 26)).toEqual({ percent: 89, passed: false });
	});
});

// The server repeats hasQuiz to lock the lessons after an exercise not
// passed; it runs the same cases.
const ExerciseCaseSchema = z.object({
	name: z.string(),
	cards: z.array(
		z.object({
			arabic: z.string(),
			meaningFr: z.string().nullable(),
			forms: z.boolean().optional(),
			audio: z.boolean().optional(),
		}),
	),
	hasExercise: z.boolean(),
});

describe("hasQuiz, on the cases shared with the server", () => {
	const cases = z.array(ExerciseCaseSchema).parse(exerciseCases);

	it.each(cases.map((shared) => [shared.name, shared] as const))(
		"%s",
		(_, shared) => {
			const cards = shared.cards.map((spec, index) =>
				card(`c-${index}`, spec.arabic, spec.meaningFr, {
					forms: spec.forms
						? {
								kind: "joining",
								initial: `${spec.arabic}ـ`,
								medial: `ـ${spec.arabic}ـ`,
								final: `ـ${spec.arabic}`,
							}
						: null,
					audio: spec.audio
						? { url: `/api/v1/audio/a-${index}`, durationMs: null }
						: null,
				}),
			);

			for (let draw = 0; draw < 20; draw++) {
				expect(hasQuiz(cards)).toBe(shared.hasExercise);
			}
		},
	);
});
