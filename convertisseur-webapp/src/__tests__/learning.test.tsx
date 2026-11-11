import { screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { vi } from "vitest";
import { type FakeApi, installFakeApi } from "./fakeApi";
import { renderAt } from "./helpers";

const validated = { status: "validated" as const };

const recording = {
	id: "a-1",
	contentType: "audio/mpeg",
	sizeBytes: 1000,
	durationMs: 1200,
	createdAt: "2026-10-01T09:00:00Z",
};

/**
 * Arabic: "L'alphabet" with two lessons, "Salutations" still to review.
 * Islam: "La prière", validated with no lesson yet.
 */
function seedTracks(api: FakeApi) {
	const content = api.content;
	content.seed("unit", { id: "u-alpha", title: "L'alphabet", ...validated });
	content.seed("lesson", {
		id: "l-letters",
		parentId: "u-alpha",
		title: "Les lettres",
		...validated,
	});
	content.seed("lesson", {
		id: "l-shapes",
		parentId: "u-alpha",
		title: "Les formes",
		...validated,
	});
	content.seed("lesson", {
		id: "l-draft",
		parentId: "u-alpha",
		title: "Brouillon",
	});
	content.seed("card", {
		id: "c-ba",
		parentId: "l-letters",
		arabic: "بَ",
		transliteration: "ba",
		meaningFr: "la lettre ba",
		audio: recording,
		...validated,
	});
	content.seed("card", {
		id: "c-ta",
		parentId: "l-letters",
		arabic: "تَ",
		transliteration: "ta",
		meaningFr: "la lettre ta",
		variety: "darija",
		...validated,
	});
	content.seed("card", { id: "c-tha", parentId: "l-letters", arabic: "ثَ" });
	content.seed("unit", { id: "u-greetings", title: "Salutations" });
	content.seed("lesson", {
		id: "l-darija",
		parentId: "u-greetings",
		title: "En darija",
		...validated,
	});
	content.seed("unit", {
		id: "u-prayer",
		track: "islam",
		title: "La prière",
		...validated,
	});
}

function loggedIn(username = "aicha") {
	const api = installFakeApi();
	api.logIn(username);
	return api;
}

beforeEach(() => {
	vi.spyOn(HTMLMediaElement.prototype, "play").mockImplementation(function (
		this: HTMLMediaElement,
	) {
		this.dispatchEvent(new Event("play"));
		return Promise.resolve();
	});
	vi.spyOn(HTMLMediaElement.prototype, "pause").mockImplementation(function (
		this: HTMLMediaElement,
	) {
		this.dispatchEvent(new Event("pause"));
	});
});

describe("home", () => {
	it("shows both tracks with what they hold", async () => {
		seedTracks(loggedIn());

		await renderAt("/");

		const arabic = await screen.findByRole("link", { name: /^Arabe/ });
		expect(arabic).toHaveAttribute("href", "/tracks/arabe");
		expect(arabic).toHaveTextContent("Unité 1 · L'alphabet");
		expect(arabic).toHaveTextContent("1 unité · 2 leçons");
		const islam = screen.getByRole("link", { name: /^Islam/ });
		expect(islam).toHaveTextContent("1 unité · 0 leçon");
		expect(
			screen.queryByText(/Aucune leçon n'est encore disponible/),
		).not.toBeInTheDocument();
	});

	it("says so while nothing is validated", async () => {
		const api = loggedIn();
		api.content.seed("unit", { id: "u-draft", title: "Brouillon" });

		await renderAt("/");

		expect(
			await screen.findByText(/Aucune leçon n'est encore disponible/),
		).toBeInTheDocument();
		expect(screen.getByRole("link", { name: /^Arabe/ })).toHaveTextContent(
			"Bientôt disponible",
		);
		expect(screen.getByRole("link", { name: /^Islam/ })).toHaveTextContent(
			"Bientôt disponible",
		);
	});

	it("keeps the account links, and the admin area for staff only", async () => {
		loggedIn("aicha");
		await renderAt("/");

		expect(
			await screen.findByRole("link", { name: "Changer mon mot de passe" }),
		).toBeInTheDocument();
		expect(
			screen.getByRole("button", { name: "Se déconnecter" }),
		).toBeInTheDocument();
		expect(
			screen.queryByRole("link", { name: "Administration" }),
		).not.toBeInTheDocument();
	});

	it("shows the admin area to staff", async () => {
		loggedIn("nour");

		await renderAt("/");

		expect(
			await screen.findByRole("link", { name: "Administration" }),
		).toHaveAttribute("href", "/admin/content");
	});

	it("opens a track's lesson map", async () => {
		seedTracks(loggedIn());
		const router = await renderAt("/");

		await userEvent.click(await screen.findByRole("link", { name: /^Arabe/ }));

		expect(
			await screen.findByRole("heading", { name: "Parcours Arabe" }),
		).toBeInTheDocument();
		expect(router.state.location.pathname).toBe("/tracks/arabe");
	});
});

describe("lesson map", () => {
	it("lists the validated units and lessons in order", async () => {
		seedTracks(loggedIn());

		await renderAt("/tracks/arabe");

		const units = await screen.findAllByRole("region");
		expect(units).toHaveLength(1);
		const alphabet = within(units[0] as HTMLElement);
		expect(alphabet.getByRole("heading", { name: "L'alphabet" }));
		expect(alphabet.getByText("Unité 1")).toBeInTheDocument();
		const lessons = alphabet.getAllByRole("link");
		expect(lessons.map((link) => link.textContent)).toEqual([
			"1Leçon 1Les lettres2 cartes",
			"2Leçon 2Les formes0 carte",
		]);
		expect(lessons[0]).toHaveAttribute(
			"href",
			"/tracks/arabe/lessons/l-letters",
		);
		expect(screen.queryByText("Brouillon")).not.toBeInTheDocument();
		expect(screen.queryByText("Salutations")).not.toBeInTheDocument();
		expect(screen.queryByText(/verrouillée|terminée/)).not.toBeInTheDocument();
	});

	it("says when a unit has no lesson yet", async () => {
		seedTracks(loggedIn());

		await renderAt("/tracks/islam");

		expect(
			await screen.findByRole("heading", { name: "La prière" }),
		).toBeInTheDocument();
		expect(
			screen.getByText("Les leçons de cette unité arrivent bientôt."),
		).toBeInTheDocument();
	});

	it("says when the track has nothing validated", async () => {
		loggedIn();

		await renderAt("/tracks/islam");

		expect(
			await screen.findByText("Pas encore de leçon ici"),
		).toBeInTheDocument();
		expect(
			screen.getAllByRole("link", { name: "Retour à l'accueil" }),
		).toHaveLength(2);
	});

	it("does not know other tracks", async () => {
		loggedIn();

		await renderAt("/tracks/histoire");

		expect(
			await screen.findByRole("heading", { name: "Page introuvable" }),
		).toBeInTheDocument();
	});
});

describe("lesson", () => {
	it("shows the cards one at a time, the Arabic in Amiri", async () => {
		seedTracks(loggedIn());

		await renderAt("/tracks/arabe/lessons/l-letters");

		expect(
			await screen.findByRole("heading", { name: "Les lettres" }),
		).toBeInTheDocument();
		const arabic = screen.getByText("بَ");
		expect(arabic).toHaveAttribute("lang", "ar");
		expect(arabic).toHaveAttribute("dir", "rtl");
		expect(screen.queryByText("تَ")).not.toBeInTheDocument();
		expect(screen.queryByText("ثَ")).not.toBeInTheDocument();
		expect(screen.getByRole("progressbar")).toHaveAttribute(
			"aria-valuetext",
			"Carte 1 sur 2",
		);
	});

	it("reveals the transliteration and the meaning on demand", async () => {
		seedTracks(loggedIn());
		await renderAt("/tracks/arabe/lessons/l-letters");
		const face = await screen.findByRole("button", {
			name: /Voir la traduction/,
		});
		expect(screen.queryByText("la lettre ba")).not.toBeInTheDocument();

		await userEvent.click(face);

		expect(face).toHaveAttribute("aria-expanded", "true");
		expect(screen.getByText("ba")).toBeInTheDocument();
		expect(screen.getByText("la lettre ba")).toBeInTheDocument();

		await userEvent.click(face);

		expect(screen.queryByText("la lettre ba")).not.toBeInTheDocument();
	});

	it("moves between cards, then ends the lesson", async () => {
		seedTracks(loggedIn());
		await renderAt("/tracks/arabe/lessons/l-letters");
		const previous = await screen.findByRole("button", { name: "Précédente" });
		expect(previous).toBeDisabled();
		await userEvent.click(screen.getByRole("button", { name: /Voir la/ }));

		await userEvent.click(screen.getByRole("button", { name: "Suivante" }));

		expect(screen.getByText("تَ")).toBeInTheDocument();
		expect(screen.getByText("Darija algérienne")).toBeInTheDocument();
		expect(
			screen.getByRole("button", { name: /Voir la traduction/ }),
		).toHaveAttribute("aria-expanded", "false");
		await userEvent.click(screen.getByRole("button", { name: "Précédente" }));
		expect(screen.getByText("بَ")).toBeInTheDocument();

		await userEvent.click(screen.getByRole("button", { name: "Suivante" }));
		await userEvent.click(screen.getByRole("button", { name: "Terminer" }));

		expect(
			screen.getByRole("heading", { name: "Leçon terminée" }),
		).toBeInTheDocument();
		expect(screen.getByText(/Tu as parcouru les 2 cartes/)).toBeInTheDocument();
		expect(
			screen.getByRole("link", { name: "Retour au parcours" }),
		).toHaveAttribute("href", "/tracks/arabe");

		await userEvent.click(
			screen.getByRole("button", { name: "Revoir les cartes" }),
		);

		expect(screen.getByText("بَ")).toBeInTheDocument();
	});

	it("follows the arrow keys", async () => {
		seedTracks(loggedIn());
		await renderAt("/tracks/arabe/lessons/l-letters");
		await screen.findByText("بَ");

		await userEvent.keyboard("{ArrowRight}");
		expect(screen.getByText("تَ")).toBeInTheDocument();

		await userEvent.keyboard("{ArrowLeft}");
		expect(screen.getByText("بَ")).toBeInTheDocument();
	});

	it("plays a card's recording when it has one", async () => {
		seedTracks(loggedIn());
		await renderAt("/tracks/arabe/lessons/l-letters");
		const play = await screen.findByRole("button", { name: "Écouter" });
		const audio = document.querySelector("audio");
		expect(audio).toHaveAttribute("src", "/api/v1/audio/a-1");

		await userEvent.click(play);

		expect(HTMLMediaElement.prototype.play).toHaveBeenCalledOnce();
		expect(play).toHaveAttribute("aria-pressed", "true");

		await userEvent.click(screen.getByRole("button", { name: "Suivante" }));

		expect(HTMLMediaElement.prototype.pause).toHaveBeenCalled();
		expect(
			screen.queryByRole("button", { name: "Écouter" }),
		).not.toBeInTheDocument();
		expect(document.querySelector("audio")).toBeNull();
	});

	it("plays the lesson's own recording", async () => {
		const api = loggedIn();
		seedTracks(api);
		const lesson = api.content.row("l-letters");
		if (lesson) {
			lesson.audio = { ...recording, id: "a-lesson" };
		}

		await renderAt("/tracks/arabe/lessons/l-letters");

		await userEvent.click(
			await screen.findByRole("button", { name: "Écouter la leçon" }),
		);
		expect(HTMLMediaElement.prototype.play).toHaveBeenCalledOnce();
		expect(
			document.querySelector('audio[src="/api/v1/audio/a-lesson"]'),
		).not.toBeNull();
	});

	it("says when the lesson has no card yet", async () => {
		seedTracks(loggedIn());

		await renderAt("/tracks/arabe/lessons/l-shapes");

		expect(await screen.findByText("Pas encore de carte")).toBeInTheDocument();
		expect(
			screen.queryByRole("button", { name: "Suivante" }),
		).not.toBeInTheDocument();
	});

	it("does not open a lesson learners do not see", async () => {
		seedTracks(loggedIn());

		await renderAt("/tracks/arabe/lessons/l-darija");

		expect(
			await screen.findByRole("heading", { name: "Leçon introuvable" }),
		).toBeInTheDocument();
	});
});
