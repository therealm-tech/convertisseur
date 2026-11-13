import { screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { type FakeApi, installFakeApi } from "./fakeApi";
import { renderAt } from "./helpers";

const validated = { status: "validated" as const };

/**
 * Arabic: "Darija" with "Saluer" and "Remercier", three cards each that make
 * an exercise, then "La famille" with "Les parents".
 */
function seed(api: FakeApi) {
	const content = api.content;
	const lessons: [string, string, string, [string, string][]][] = [
		[
			"u-darija",
			"l-greet",
			"Saluer",
			[
				["صباح الخير", "Bonjour"],
				["مسا الخير", "Bonsoir"],
				["بسلامة", "Au revoir"],
			],
		],
		[
			"u-darija",
			"l-thank",
			"Remercier",
			[
				["صحة", "Merci"],
				["سمحلي", "Pardon"],
				["من فضلك", "S'il te plaît"],
			],
		],
		[
			"u-family",
			"l-parents",
			"Les parents",
			[
				["يما", "Maman"],
				["بابا", "Papa"],
				["خويا", "Mon frère"],
			],
		],
	];
	content.seed("unit", { id: "u-darija", title: "Darija", ...validated });
	content.seed("unit", { id: "u-family", title: "La famille", ...validated });
	for (const [unit, id, title, cards] of lessons) {
		content.seed("lesson", { id, parentId: unit, title, ...validated });
		for (const [index, [arabic, meaningFr]] of cards.entries()) {
			content.seed("card", {
				id: `${id}-${index}`,
				parentId: id,
				arabic,
				meaningFr,
				...validated,
			});
		}
	}
}

function learner() {
	const api = installFakeApi();
	api.logIn("aicha");
	seed(api);
	return { api, id: api.account("aicha")?.user.id ?? "" };
}

describe("track map", () => {
	it("locks every lesson after an exercise not passed", async () => {
		learner();

		await renderAt("/tracks/arabe");

		expect(
			await screen.findByRole("link", { name: /^Leçon 1Saluer/ }),
		).toHaveAttribute("href", "/tracks/arabe/lessons/l-greet");
		expect(screen.queryByRole("link", { name: /Remercier/ })).toBeNull();
		expect(
			screen.getByText("Verrouillée · réussis l'exercice de « Saluer »"),
		).toBeInTheDocument();
		expect(screen.getByText("Verrouillée")).toBeInTheDocument();
		expect(
			screen.getByRole("link", { name: "S'entraîner : Saluer" }),
		).toHaveAttribute("href", "/tracks/arabe/lessons/l-greet/exercise");
		expect(screen.queryByRole("link", { name: /: Remercier$/ })).toBeNull();
	});

	it("shows what is done, in progress and passed", async () => {
		const { api, id } = learner();
		api.content.see(id, ["l-greet-0", "l-greet-1", "l-greet-2", "l-thank-0"]);
		api.content.attempt(id, "l-greet", 11, 12);

		await renderAt("/tracks/arabe");

		const darija = within(
			await screen.findByRole("region", { name: "Darija" }),
		);
		expect(
			darija.getByText("Unité 1 · 1 leçon terminée sur 2"),
		).toBeInTheDocument();
		expect(darija.getByText("Terminée · 3 cartes")).toBeInTheDocument();
		expect(
			darija.getByRole("link", { name: "Réussi · 11/12 : Saluer" }),
		).toBeInTheDocument();
		expect(darija.getByText("En cours · 1/3 cartes")).toBeInTheDocument();
		expect(
			darija.getByRole("link", { name: "S'entraîner : Remercier" }),
		).toBeInTheDocument();
		expect(
			screen.getByText("Verrouillée · réussis l'exercice de « Remercier »"),
		).toBeInTheDocument();
	});
});

describe("locked lesson", () => {
	it("says why it does not open", async () => {
		learner();

		await renderAt("/tracks/arabe/lessons/l-thank");

		expect(
			await screen.findByRole("heading", { name: "Leçon verrouillée" }),
		).toBeInTheDocument();
		expect(
			screen.getByRole("link", { name: "Retour au parcours" }),
		).toHaveAttribute("href", "/tracks/arabe");
	});

	it("keeps its exercise closed too", async () => {
		learner();

		await renderAt("/tracks/arabe/lessons/l-thank/exercise");

		expect(
			await screen.findByRole("heading", { name: "Leçon verrouillée" }),
		).toBeInTheDocument();
	});
});

describe("lesson progress", () => {
	it("records each card shown", async () => {
		const { api } = learner();
		await renderAt("/tracks/arabe/lessons/l-greet");
		await screen.findByText("1/3 cartes vues");

		await userEvent.click(screen.getByRole("button", { name: "Suivante" }));

		expect(await screen.findByText("2/3 cartes vues")).toBeInTheDocument();
		const views = api.calls
			.filter((call) => call.path.startsWith("/api/v1/card-views/"))
			.map((call) => `${call.method} ${call.path}`);
		expect(views).toEqual([
			"PUT /api/v1/card-views/l-greet-0",
			"PUT /api/v1/card-views/l-greet-1",
		]);
	});

	it("resumes at the first card not seen yet", async () => {
		const { api, id } = learner();
		api.content.see(id, ["l-greet-0"]);

		await renderAt("/tracks/arabe/lessons/l-greet");

		expect(
			await screen.findByText(/On reprend à la carte 2/),
		).toBeInTheDocument();
		expect(screen.getByText("مسا الخير")).toBeInTheDocument();
		await userEvent.click(
			screen.getByRole("button", { name: "Recommencer au début" }),
		);
		expect(screen.getByText("صباح الخير")).toBeInTheDocument();
		expect(screen.queryByText(/On reprend/)).toBeNull();
	});

	it("starts over a lesson whose cards are all seen, with its score", async () => {
		const { api, id } = learner();
		api.content.see(id, ["l-greet-0", "l-greet-1", "l-greet-2"]);
		api.content.attempt(id, "l-greet", 7, 12);

		await renderAt("/tracks/arabe/lessons/l-greet");

		expect(await screen.findByText("Terminée")).toBeInTheDocument();
		expect(screen.getByText("Meilleur score · 7/12")).toBeInTheDocument();
		expect(screen.getByText("صباح الخير")).toBeInTheDocument();
		expect(screen.queryByText(/On reprend/)).toBeNull();
		expect(screen.getByRole("link", { name: "S'entraîner" })).toHaveAttribute(
			"href",
			"/tracks/arabe/lessons/l-greet/exercise",
		);
	});
});

describe("home", () => {
	it("offers the first lesson to a new account", async () => {
		learner();

		await renderAt("/");

		const card = within(await screen.findByRole("region", { name: "Saluer" }));
		expect(
			card.getByText("Commencer", { selector: "span" }),
		).toBeInTheDocument();
		expect(card.getByText("Arabe · Unité 1 · Leçon 1")).toBeInTheDocument();
		expect(card.getByRole("link", { name: "Commencer" })).toHaveAttribute(
			"href",
			"/tracks/arabe/lessons/l-greet",
		);
	});

	it("resumes the last lesson touched", async () => {
		const { api, id } = learner();
		api.content.see(id, ["l-greet-0"]);

		await renderAt("/");

		const card = within(await screen.findByRole("region", { name: "Saluer" }));
		expect(card.getByText("Reprendre")).toBeInTheDocument();
		expect(card.getByText("1 carte vue sur 3")).toBeInTheDocument();
		expect(card.getByRole("link", { name: "Continuer" })).toHaveAttribute(
			"href",
			"/tracks/arabe/lessons/l-greet",
		);
		expect(screen.getByText("0 leçon terminée sur 3")).toBeInTheDocument();
	});

	it("sends to the exercise once every card is seen", async () => {
		const { api, id } = learner();
		api.content.see(id, ["l-greet-0", "l-greet-1", "l-greet-2"]);

		await renderAt("/");

		const card = within(await screen.findByRole("region", { name: "Saluer" }));
		expect(card.getByText("Réussis l'exercice")).toBeInTheDocument();
		expect(card.getByRole("link", { name: "S'entraîner" })).toHaveAttribute(
			"href",
			"/tracks/arabe/lessons/l-greet/exercise",
		);
		expect(screen.getByText("1 leçon terminée sur 3")).toBeInTheDocument();
	});

	it("moves on to the next lesson once the exercise is passed", async () => {
		const { api, id } = learner();
		api.content.see(id, ["l-greet-0", "l-greet-1", "l-greet-2"]);
		api.content.attempt(id, "l-greet");

		await renderAt("/");

		const card = within(
			await screen.findByRole("region", { name: "Remercier" }),
		);
		expect(card.getByText("Leçon suivante")).toBeInTheDocument();
		expect(card.getByRole("link", { name: "Commencer" })).toHaveAttribute(
			"href",
			"/tracks/arabe/lessons/l-thank",
		);
	});
});
