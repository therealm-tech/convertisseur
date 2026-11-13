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
			"0 sur 6",
		);
	});

	it("is offered from the first card of a lesson", async () => {
		loggedIn();
		await renderAt("/tracks/arabe/lessons/l-greetings");

		await userEvent.click(
			await screen.findByRole("link", { name: "S'entraîner" }),
		);

		expect(await screen.findByText("Exercice")).toBeInTheDocument();
		expect(screen.getByRole("progressbar")).toHaveAttribute(
			"aria-valuetext",
			"0 sur 6",
		);
	});

	it("is offered on the track's map, under lessons of three cards or more", async () => {
		loggedIn();
		const router = await renderAt("/tracks/arabe");

		await userEvent.click(
			await screen.findByRole("link", { name: "S'entraîner : Saluer" }),
		);

		expect(await screen.findByText("Exercice")).toBeInTheDocument();
		expect(router.state.location.pathname).toBe(
			"/tracks/arabe/lessons/l-greetings/exercise",
		);
	});

	it("is not offered on the map under three cards", async () => {
		loggedIn();
		await renderAt("/tracks/arabe");

		expect(
			await screen.findByRole("link", { name: /Remercier/ }),
		).toBeInTheDocument();
		expect(
			screen.queryByRole("link", { name: "S'entraîner : Remercier" }),
		).not.toBeInTheDocument();
	});

	it("is not offered under three cards", async () => {
		loggedIn();
		await renderAt("/tracks/arabe/lessons/l-short");
		expect(
			await screen.findByRole("heading", { name: "Remercier" }),
		).toBeInTheDocument();
		expect(
			screen.queryByRole("link", { name: "S'entraîner" }),
		).not.toBeInTheDocument();
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

	it("asks each card both ways, then grades a pass", async () => {
		loggedIn();
		await renderAt("/tracks/arabe/lessons/l-greetings/exercise");
		await screen.findByRole("heading", { level: 2 });
		const asked = new Set<string>();

		for (let i = 0; i < 6; i++) {
			const { prompt, answer: expected } = question();
			asked.add(prompt);
			await answer(true);
			expect(screen.getByText("Bonne réponse !")).toBeInTheDocument();
			const right = screen.getByRole("button", { name: /\(bonne réponse\)$/ });
			expect(right).toHaveTextContent(expected);
			expect(right).toBeDisabled();
			await userEvent.click(
				screen.getByRole("button", { name: i < 5 ? "Continuer" : "Terminer" }),
			);
		}

		expect(asked).toEqual(new Set(Object.keys(answers)));
		expect(
			screen.getByRole("heading", { name: "Exercice réussi" }),
		).toBeInTheDocument();
		expect(screen.getByText("100 %")).toBeInTheDocument();
		expect(screen.getByText("6 sur 6 du premier coup")).toBeInTheDocument();
		expect(
			screen.getByRole("link", { name: "Retour au parcours" }),
		).toHaveAttribute("href", "/tracks/arabe");
	});

	it("brings a missed question back, and fails under ninety percent", async () => {
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
			"0 sur 6",
		);

		const after: string[] = [];
		while (!screen.queryByRole("heading", { name: /^Exercice / })) {
			after.push(question().prompt);
			await answer(true);
			await userEvent.click(
				screen.getByRole("button", { name: /^(Continuer|Terminer)$/ }),
			);
		}

		expect(after).toHaveLength(6);
		expect(after).toContain(missed);
		expect(
			screen.getByRole("heading", { name: "Exercice à refaire" }),
		).toBeInTheDocument();
		expect(screen.getByText("83 %")).toBeInTheDocument();
		expect(screen.getByText("5 sur 6 du premier coup")).toBeInTheDocument();
		expect(screen.getByText("Il faut 90 % pour réussir.")).toBeInTheDocument();

		await userEvent.click(screen.getByRole("button", { name: "Recommencer" }));

		expect(screen.getByRole("progressbar")).toHaveAttribute(
			"aria-valuetext",
			"0 sur 6",
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

	it("asks to recognise a card's recording", async () => {
		const api = loggedIn();
		api.content.seed("lesson", {
			id: "l-heard",
			parentId: "u-darija",
			title: "Entendre",
			...validated,
		});
		for (const [index, [arabic, meaningFr]] of Object.entries(
			greetings,
		).entries()) {
			api.content.seed("card", {
				id: `h-${index}`,
				parentId: "l-heard",
				arabic,
				meaningFr,
				audio: {
					id: `a-${index}`,
					contentType: "audio/mpeg",
					sizeBytes: 1000,
					durationMs: 1200,
					createdAt: "2026-10-01T09:00:00Z",
				},
				...validated,
			});
		}

		await renderAt("/tracks/arabe/lessons/l-heard/exercise");
		await screen.findByRole("heading", { level: 2 });
		while (!screen.queryByRole("heading", { name: "Qu'entends-tu ?" })) {
			const [first] = question().choices;
			if (first) {
				await userEvent.click(first);
			}
			await userEvent.click(
				screen.getByRole("button", { name: /^(Continuer|Terminer)$/ }),
			);
		}

		expect(screen.getByRole("button", { name: "Écouter" })).toBeInTheDocument();
		expect(document.querySelector("audio")?.getAttribute("src")).toMatch(
			/^\/api\/v1\/audio\/a-[0-2]$/,
		);
		const group = screen.getByRole("group", { name: "Qu'entends-tu ?" });
		for (const choice of within(group).getAllByRole("button")) {
			expect(Object.keys(greetings)).toContain(choice.textContent);
		}
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
