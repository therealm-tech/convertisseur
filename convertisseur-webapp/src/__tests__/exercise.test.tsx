import { screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { type FakeApi, installFakeApi } from "./fakeApi";
import { renderAt } from "./helpers";

const validated = { status: "validated" as const };

const greetings: Record<string, string> = {
	"صباح الخير": "Bonjour (le matin)",
	"مسا الخير": "Bonsoir",
	بسلامة: "Au revoir",
};

/** The prompt each card can be asked from, and its answer. */
const answers: Record<string, string> = Object.fromEntries(
	Object.entries(greetings).flatMap(([arabic, meaning]) => [
		[arabic, meaning],
		[meaning, arabic],
	]),
);

function seed(api: FakeApi) {
	const content = api.content;
	content.seed("unit", { id: "u-darija", title: "Darija", ...validated });
	content.seed("lesson", {
		id: "l-greetings",
		parentId: "u-darija",
		title: "Saluer",
		...validated,
	});
	for (const [index, [arabic, meaningFr]] of Object.entries(
		greetings,
	).entries()) {
		content.seed("card", {
			id: `c-${index}`,
			parentId: "l-greetings",
			arabic,
			meaningFr,
			...validated,
		});
	}
	content.seed("lesson", {
		id: "l-short",
		parentId: "u-darija",
		title: "Remercier",
		...validated,
	});
	content.seed("card", {
		id: "c-short-1",
		parentId: "l-short",
		arabic: "صحة",
		meaningFr: "Merci",
		...validated,
	});
	content.seed("card", {
		id: "c-short-2",
		parentId: "l-short",
		arabic: "يعطيك الصحة",
		meaningFr: "Merci (pour un effort)",
		...validated,
	});
	content.seed("lesson", {
		id: "l-draft",
		parentId: "u-darija",
		title: "Brouillon",
	});
}

function loggedIn() {
	const api = installFakeApi();
	api.logIn("aicha");
	seed(api);
	return api;
}

function question() {
	const heading = screen.getByRole("heading", { level: 2 });
	const prompt = heading.nextElementSibling?.textContent ?? "";
	const answer = answers[prompt];
	if (answer === undefined) {
		throw new Error(`no card asks "${prompt}"`);
	}
	return {
		prompt,
		answer,
		choices: within(
			screen.getByRole("group", { name: heading.textContent ?? "" }),
		).getAllByRole("button"),
	};
}

async function answer(right: boolean) {
	const { answer: expected, choices } = question();
	const choice = choices.find(
		(button) => (button.textContent === expected) === right,
	);
	if (!choice) {
		throw new Error("no such choice");
	}
	await userEvent.click(choice);
}

describe("exercise", () => {
	it("is offered at the end of a lesson of three cards or more", async () => {
		loggedIn();
		await renderAt("/tracks/arabe/lessons/l-greetings");
		for (let i = 0; i < 3; i++) {
			await userEvent.click(
				await screen.findByRole("button", { name: /Suivante|Terminer/ }),
			);
		}

		await userEvent.click(screen.getByRole("link", { name: "S'entraîner" }));

		expect(
			await screen.findByRole("heading", { name: "Saluer" }),
		).toBeInTheDocument();
		expect(screen.getByText("Exercice")).toBeInTheDocument();
		expect(screen.getByRole("progressbar")).toHaveAttribute(
			"aria-valuetext",
			"0 sur 3",
		);
	});

	it("is not offered under three cards", async () => {
		loggedIn();
		await renderAt("/tracks/arabe/lessons/l-short");
		await userEvent.click(
			await screen.findByRole("button", { name: "Suivante" }),
		);
		await userEvent.click(screen.getByRole("button", { name: "Terminer" }));

		expect(
			screen.getByRole("heading", { name: "Leçon terminée" }),
		).toBeInTheDocument();
		expect(
			screen.queryByRole("link", { name: "S'entraîner" }),
		).not.toBeInTheDocument();
	});

	it("asks every card once, then gives the score", async () => {
		loggedIn();
		await renderAt("/tracks/arabe/lessons/l-greetings/exercise");
		await screen.findByRole("heading", { level: 2 });
		const asked = new Set<string>();

		for (let i = 0; i < 3; i++) {
			const { prompt, answer: expected } = question();
			asked.add(greetings[prompt] ?? prompt);
			await answer(true);
			expect(screen.getByText("Bonne réponse !")).toBeInTheDocument();
			const right = screen.getByRole("button", { name: /\(bonne réponse\)$/ });
			expect(right).toHaveTextContent(expected);
			expect(right).toBeDisabled();
			await userEvent.click(
				screen.getByRole("button", { name: i < 2 ? "Continuer" : "Terminer" }),
			);
		}

		expect(asked).toEqual(new Set(Object.values(greetings)));
		expect(
			screen.getByRole("heading", { name: "Exercice terminé" }),
		).toBeInTheDocument();
		expect(screen.getByText("3 sur 3 du premier coup")).toBeInTheDocument();
		expect(
			screen.getByRole("link", { name: "Retour au parcours" }),
		).toHaveAttribute("href", "/tracks/arabe");
	});

	it("brings a missed question back at the end, and counts it missed", async () => {
		loggedIn();
		await renderAt("/tracks/arabe/lessons/l-greetings/exercise");
		await screen.findByRole("heading", { level: 2 });
		const missed = question().prompt;

		await answer(false);

		expect(
			screen.getByText("Pas tout à fait, elle reviendra."),
		).toBeInTheDocument();
		expect(
			screen.getByRole("button", { name: /\(bonne réponse\)$/ }),
		).toHaveTextContent(question().answer);
		expect(
			screen.getByRole("button", { name: /\(ta réponse\)$/ }),
		).toBeInTheDocument();
		await userEvent.click(screen.getByRole("button", { name: "Continuer" }));
		expect(screen.getByRole("progressbar")).toHaveAttribute(
			"aria-valuetext",
			"0 sur 3",
		);

		for (let i = 0; i < 2; i++) {
			await answer(true);
			await userEvent.click(screen.getByRole("button", { name: "Continuer" }));
		}
		expect(question().prompt).toBe(missed);
		await answer(true);
		await userEvent.click(screen.getByRole("button", { name: "Terminer" }));

		expect(screen.getByText("2 sur 3 du premier coup")).toBeInTheDocument();

		await userEvent.click(screen.getByRole("button", { name: "Recommencer" }));

		expect(screen.getByRole("progressbar")).toHaveAttribute(
			"aria-valuetext",
			"0 sur 3",
		);
		expect(question().choices).toHaveLength(3);
	});

	it("asks a letter for its sound, its shape or its forms", async () => {
		const api = loggedIn();
		api.content.seed("lesson", {
			id: "l-letters",
			parentId: "u-darija",
			title: "Les lettres",
			...validated,
		});
		for (const [id, arabic, sound] of [
			["ba", "ب", "« b »"],
			["ta", "ت", "« t »"],
			["tha", "ث", "« th »"],
		] as const) {
			api.content.seed("card", {
				id,
				parentId: "l-letters",
				arabic,
				meaningFr: sound,
				forms: {
					kind: "joining",
					initial: `${arabic}ـ`,
					medial: `ـ${arabic}ـ`,
					final: `ـ${arabic}`,
				},
				...validated,
			});
		}

		await renderAt("/tracks/arabe/lessons/l-letters/exercise");

		expect(
			(await screen.findByRole("heading", { level: 2 })).textContent,
		).toMatch(/^(Quel son \?|Quelle lettre \?|Quelle est cette lettre \?)$/);
	});

	it("says when a lesson is too short for one", async () => {
		loggedIn();

		await renderAt("/tracks/arabe/lessons/l-short/exercise");

		expect(
			await screen.findByText("Pas d'exercice pour cette leçon"),
		).toBeInTheDocument();
		expect(
			screen.getByRole("link", { name: "Retour à la leçon" }),
		).toHaveAttribute("href", "/tracks/arabe/lessons/l-short");
	});

	it("does not open for a lesson learners do not see", async () => {
		loggedIn();

		await renderAt("/tracks/arabe/lessons/l-draft/exercise");

		expect(
			await screen.findByRole("heading", { name: "Leçon introuvable" }),
		).toBeInTheDocument();
	});
});
