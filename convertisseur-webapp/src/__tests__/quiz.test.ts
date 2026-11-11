import { buildQuiz, hasQuiz, type Random } from "@/lib/quiz";
import type { StudyCard } from "@/lib/tracks";

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

describe("quiz", () => {
	it("asks one question per card, with the answer among distinct choices", () => {
		for (let seed = 1; seed <= 50; seed++) {
			const questions = buildQuiz(words, seeded(seed));

			expect(questions.map((question) => question.id).sort()).toEqual(
				words.map((word) => word.id).sort(),
			);
			for (const question of questions) {
				expect(question.choices).toContain(question.answer);
				expect(new Set(question.choices).size).toBe(question.choices.length);
				expect(question.choices).toHaveLength(4);
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
