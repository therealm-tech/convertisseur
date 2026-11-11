import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { vi } from "vitest";
import type { Audio } from "@/lib/content";
import { type FakeApi, installFakeApi } from "./fakeApi";
import { renderAt } from "./helpers";

const recording: Audio = {
	id: "a-1",
	contentType: "audio/mpeg",
	sizeBytes: 40_000,
	durationMs: 2_000,
	createdAt: "2026-11-05T10:00:00Z",
};

const validated = {
	status: "validated" as const,
	validatedAt: "2026-11-02T09:00:00Z",
	validatedBy: { id: "u-bilal", username: "bilal", role: "admin" as const },
};

/**
 * Salutations, to review: Saluer has a validated card with audio, a flagged
 * one and one to review, Politesse only validated cards with audio. The
 * alphabet is validated, but one of its cards has no audio.
 */
function seedContent(api: FakeApi) {
	const content = api.content;
	const greetings = content.seed("unit", {
		id: "u-salutations",
		title: "Salutations",
		track: "arabe",
		importKey: "arabe/salutations",
	});
	const greet = content.seed("lesson", {
		id: "l-saluer",
		parentId: greetings,
		title: "Saluer",
		importKey: "arabe/salutations/saluer",
	});
	content.seed("card", {
		id: "c-salam",
		parentId: greet,
		arabic: "السَّلَامُ عَلَيْكُمْ",
		transliteration: "salam 3likoum",
		meaningFr: "bonjour",
		variety: "darija",
		audio: recording,
		importKey: "arabe/salutations/saluer/salam",
		...validated,
	});
	content.seed("card", {
		id: "c-wach-rak",
		parentId: greet,
		arabic: "وَاش رَاك",
		transliteration: "wach rak",
		meaningFr: "ça va ? (à un homme)",
		variety: "darija",
		reviewNote: "Il manque le point d'interrogation",
		importKey: "arabe/salutations/saluer/wach-rak",
	});
	content.seed("card", {
		id: "c-labas",
		parentId: greet,
		arabic: "لَابَاس",
		transliteration: "labas",
		meaningFr: "ça va, bien",
		variety: "darija",
		importKey: "arabe/salutations/saluer/labas",
	});
	const politeness = content.seed("lesson", {
		id: "l-politesse",
		parentId: greetings,
		title: "Politesse",
		...validated,
	});
	content.seed("card", {
		id: "c-sahha",
		parentId: politeness,
		arabic: "صَحَّة",
		transliteration: "sa7a",
		meaningFr: "merci",
		audio: { ...recording, id: "a-2" },
		...validated,
	});
	const alphabet = content.seed("unit", {
		id: "u-alphabet",
		title: "L'alphabet",
		track: "arabe",
		...validated,
	});
	const letters = content.seed("lesson", {
		id: "l-lettres",
		parentId: alphabet,
		title: "Les lettres",
		...validated,
	});
	content.seed("card", {
		id: "c-alif",
		parentId: letters,
		arabic: "أَلِف",
		transliteration: "alif",
		...validated,
	});
}

function setDesktop(desktop: boolean) {
	vi.spyOn(window, "matchMedia").mockImplementation(
		(query: string) =>
			({
				matches: desktop,
				media: query,
				addEventListener: () => undefined,
				removeEventListener: () => undefined,
			}) as unknown as MediaQueryList,
	);
}

async function open(path: string, desktop = false) {
	// jsdom has no matchMedia: define one the spy can replace.
	window.matchMedia ??= ((query: string) => ({
		matches: false,
		media: query,
	})) as unknown as typeof window.matchMedia;
	setDesktop(desktop);
	const api = installFakeApi();
	seedContent(api);
	api.logIn("nour");
	const user = userEvent.setup();
	const router = await renderAt(path);
	return { api, user, router };
}

function calls(api: FakeApi, method: string, path: string) {
	return api.calls.filter(
		(call) => call.method === method && call.path === path,
	);
}

describe("review queue", () => {
	it("lists the lessons left to review by unit, with their counts", async () => {
		await open("/admin/relecture");

		await screen.findByRole("heading", { name: "Relecture", level: 1 });
		expect(
			await screen.findByText("4 éléments à relire · 3 cartes sans audio"),
		).toBeInTheDocument();
		const unit = screen.getByRole("region", { name: "Salutations" });
		expect(
			within(unit).getByRole("heading", { name: "Arabe · Salutations" }),
		).toBeInTheDocument();
		expect(within(unit).getByText("Unité à relire")).toBeInTheDocument();
		const links = within(unit).getAllByRole("link");
		expect(links.map((link) => link.textContent)).toEqual([
			"Saluer1/3 validées · 1 à relire · 1 signalée · 2 sans audio",
			"Politesse1/1 validées",
		]);
		expect(links[0]).toHaveAttribute(
			"href",
			"/admin/relecture/lecons/l-saluer",
		);
		expect(
			screen.queryByRole("region", { name: "L'alphabet" }),
		).not.toBeInTheDocument();
	});

	it("filters the lessons with cards without audio, even validated", async () => {
		const { user } = await open("/admin/relecture");
		await screen.findByRole("region", { name: "Salutations" });

		await user.click(screen.getByRole("button", { name: "Sans audio" }));

		const alphabet = await screen.findByRole("region", { name: "L'alphabet" });
		expect(
			within(alphabet).getByRole("link", { name: /Les lettres/ }),
		).toHaveTextContent("1/1 validées · 1 sans audio");
		expect(screen.getByRole("link", { name: /Saluer/ })).toBeInTheDocument();
		expect(
			screen.queryByRole("link", { name: /Politesse/ }),
		).not.toBeInTheDocument();
		expect(screen.getByRole("button", { name: "Sans audio" })).toHaveAttribute(
			"aria-pressed",
			"true",
		);
	});

	it("says when nothing is left to review", async () => {
		const { api } = await open("/admin/relecture");
		await screen.findByRole("region", { name: "Salutations" });
		for (const row of api.content.rows) {
			row.status = "validated";
			row.reviewNote = null;
		}

		await userEvent.click(screen.getByRole("link", { name: "Contenu" }));
		await userEvent.click(
			await screen.findByRole("link", { name: /^Relecture/ }),
		);

		expect(await screen.findByText("Rien à relire.")).toBeInTheDocument();
		expect(screen.getByRole("link", { name: "Relecture" })).toBeInTheDocument();
	});

	it("comes first in the admin navigation, with the count to review", async () => {
		await open("/admin/contenu");

		const nav = screen.getByRole("navigation", { name: "Administration" });
		const first = within(nav).getAllByRole("link")[0];
		await waitFor(() =>
			expect(first).toHaveAccessibleName("Relecture, 4 à relire"),
		);
		expect(first).toHaveAttribute("href", "/admin/relecture");
		expect(
			within(nav).getByRole("link", { name: "Utilisateurs et rôles" }),
		).toHaveAttribute("href", "/admin/utilisateurs");
	});
});

describe("reviewing on a phone", () => {
	it("takes the whole screen, and its close button leads back to the queue", async () => {
		const { user } = await open("/admin/relecture/lecons/l-saluer");
		await screen.findByRole("article", { name: "Carte 3" });

		expect(
			screen.queryByRole("navigation", { name: "Administration" }),
		).not.toBeInTheDocument();
		expect(screen.queryByText("Connecté en tant que")).not.toBeInTheDocument();

		await user.click(
			screen.getByRole("link", { name: "Revenir à la liste des leçons" }),
		);

		expect(
			await screen.findByRole("navigation", { name: "Administration" }),
		).toBeInTheDocument();
		expect(
			screen.getByRole("heading", { name: "Relecture", level: 1 }),
		).toBeInTheDocument();
	});

	it("opens on the first card to review, with its audio to record", async () => {
		await open("/admin/relecture/lecons/l-saluer");

		const card = await screen.findByRole("article", { name: "Carte 3" });
		expect(screen.getByText("3/3")).toHaveTextContent("Carte 3/3");
		expect(within(card).getByText("لَابَاس")).toHaveAttribute("lang", "ar");
		expect(within(card).getByText("لَابَاس")).toHaveAttribute("dir", "rtl");
		expect(within(card).getByText("Géré dans le repo")).toBeInTheDocument();
		expect(within(card).getByText("À relire")).toBeInTheDocument();
		expect(
			within(card).getByRole("button", {
				name: "Enregistrer la prononciation",
			}),
		).toBeVisible();
	});

	it("validates a card, then shows the end of the lesson", async () => {
		const { api, user } = await open("/admin/relecture/lecons/l-saluer");
		await screen.findByRole("article", { name: "Carte 3" });

		await user.click(screen.getByRole("button", { name: "Valider" }));

		expect(
			await screen.findByRole("heading", { name: "Leçon relue" }),
		).toHaveFocus();
		expect(calls(api, "PUT", "/api/v1/cards/c-labas/status")).toHaveLength(1);
		expect(api.content.row("c-labas")?.status).toBe("validated");
		expect(
			screen.getByText("2 validées · 1 signalée · 0 à relire"),
		).toBeInTheDocument();
		const flagged = screen.getByRole("region", { name: "Signalées" });
		expect(
			within(flagged).getByText("Il manque le point d'interrogation"),
		).toBeInTheDocument();
	});

	it("goes back, and passes over a validated card", async () => {
		const { api, user } = await open("/admin/relecture/lecons/l-saluer");
		await screen.findByRole("article", { name: "Carte 3" });

		await user.click(screen.getByRole("button", { name: "Carte précédente" }));
		await user.click(screen.getByRole("button", { name: "Carte précédente" }));
		await screen.findByRole("article", { name: "Carte 1" });
		expect(
			screen.getByRole("button", { name: "Carte précédente" }),
		).toBeDisabled();
		expect(screen.getByRole("button", { name: "Écouter" })).toBeInTheDocument();

		await user.click(screen.getByRole("button", { name: "Suivante" }));

		await screen.findByRole("article", { name: "Carte 2" });
		expect(calls(api, "PUT", "/api/v1/cards/c-salam/status")).toHaveLength(0);
	});

	it("flags a card in a panel that holds the focus, then moves on", async () => {
		const { api, user } = await open("/admin/relecture/lecons/l-saluer");
		await screen.findByRole("article", { name: "Carte 3" });
		const flag = screen.getByRole("button", { name: "Signaler" });

		await user.click(flag);
		let dialog = screen.getByRole("dialog", { name: "Signaler une erreur" });
		const field = within(dialog).getByLabelText("Qu'est-ce qui ne va pas ?");
		expect(field).toHaveFocus();
		await user.click(within(dialog).getByRole("button", { name: "Annuler" }));
		expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
		expect(flag).toHaveFocus();

		await user.click(flag);
		dialog = screen.getByRole("dialog", { name: "Signaler une erreur" });
		const submit = within(dialog).getByRole("button", {
			name: "Signaler et continuer",
		});
		expect(submit).toBeDisabled();
		await user.type(
			within(dialog).getByLabelText("Qu'est-ce qui ne va pas ?"),
			"Il faut un alif long",
		);
		await user.click(submit);

		expect(
			await screen.findByRole("heading", { name: "Leçon relue" }),
		).toBeInTheDocument();
		expect(api.content.row("c-labas")?.reviewNote).toBe("Il faut un alif long");
		expect(api.content.row("c-labas")?.status).toBe("to_review");
		expect(
			screen.getByText("1 validée · 2 signalées · 0 à relire"),
		).toBeInTheDocument();
	});

	it("validates a flagged card and clears its note", async () => {
		const { api, user } = await open("/admin/relecture/lecons/l-saluer");
		await screen.findByRole("article", { name: "Carte 3" });
		await user.click(screen.getByRole("button", { name: "Carte précédente" }));
		const card = await screen.findByRole("article", { name: "Carte 2" });
		expect(within(card).getByText("Signalée")).toBeInTheDocument();
		expect(within(card).getByText("Ta remarque")).toBeInTheDocument();

		await user.click(screen.getByRole("button", { name: "Valider" }));

		await screen.findByRole("article", { name: "Carte 3" });
		expect(api.content.row("c-wach-rak")?.status).toBe("validated");
		expect(api.content.row("c-wach-rak")?.reviewNote).toBeNull();
	});

	it("validates the lesson and its unit from the end screen", async () => {
		const { api, user } = await open("/admin/relecture/lecons/l-saluer");
		await screen.findByRole("article", { name: "Carte 3" });
		await user.click(screen.getByRole("button", { name: "Valider" }));
		await screen.findByRole("heading", { name: "Leçon relue" });

		expect(
			screen.getByText(
				"Les apprenants ne voient une carte que si sa leçon et son unité sont validées aussi.",
			),
		).toBeInTheDocument();
		await user.click(screen.getByRole("button", { name: "Valider la leçon" }));
		const unit = screen.getByRole("button", { name: "Valider l'unité" });
		await waitFor(() => expect(unit).toBeEnabled());
		await user.click(unit);

		expect(
			await screen.findByText(
				"Les cartes validées de cette leçon sont visibles des apprenants.",
			),
		).toBeInTheDocument();
		expect(api.content.row("l-saluer")?.status).toBe("validated");
		expect(api.content.row("u-salutations")?.status).toBe("validated");
		expect(
			screen.getAllByRole("button", { name: "Repasser à relire" }),
		).toHaveLength(2);
		expect(
			screen.getByText("Audio de la leçon (facultatif)"),
		).toBeInTheDocument();
		expect(
			screen.getByRole("link", { name: "Toutes les leçons" }),
		).toHaveAttribute("href", "/admin/relecture");
	});

	it("says when a lesson has no card", async () => {
		const { api } = await open("/admin/relecture");
		api.content.seed("lesson", {
			id: "l-vide",
			parentId: "u-salutations",
			title: "Au revoir",
		});
		await screen.findByRole("region", { name: "Salutations" });
		await userEvent.click(screen.getByRole("link", { name: "Contenu" }));
		await userEvent.click(
			await screen.findByRole("link", { name: /^Relecture/ }),
		);
		await userEvent.click(
			await screen.findByRole("link", { name: /Au revoir/ }),
		);

		expect(
			await screen.findByText("Cette leçon n'a aucune carte."),
		).toBeInTheDocument();
		expect(
			screen.getByRole("button", { name: "Valider la leçon" }),
		).toBeInTheDocument();
	});
});

describe("reviewing on a desktop", () => {
	it("keeps the admin navigation", async () => {
		await open("/admin/relecture/lecons/l-saluer", true);
		await screen.findByRole("region", { name: "Cartes de la leçon" });

		expect(
			screen.getByRole("navigation", { name: "Administration" }),
		).toBeInTheDocument();
	});

	it("lists every card with its state, its audio and its note", async () => {
		await open("/admin/relecture/lecons/l-saluer", true);

		const cards = await screen.findByRole("region", {
			name: "Cartes de la leçon",
		});
		expect(within(cards).getAllByRole("article")).toHaveLength(3);
		const second = within(cards).getByRole("article", { name: "Carte 2" });
		expect(within(second).getByText("Signalée")).toBeInTheDocument();
		expect(
			within(second).getByText("Il manque le point d'interrogation"),
		).toBeInTheDocument();
		for (const name of ["Carte 2", "Carte 3"]) {
			expect(
				within(within(cards).getByRole("article", { name })).getByRole(
					"button",
					{ name: "Enregistrer la prononciation" },
				),
			).toBeVisible();
		}
		expect(
			within(within(cards).getByRole("article", { name: "Carte 1" })).getByRole(
				"button",
				{ name: "Écouter" },
			),
		).toBeInTheDocument();
		expect(
			screen.getByText("1 validée · 1 signalée · 1 à relire"),
		).toBeInTheDocument();
	});

	it("validates the cards left, the flagged one aside", async () => {
		const { api, user } = await open("/admin/relecture/lecons/l-saluer", true);
		await screen.findByRole("region", { name: "Cartes de la leçon" });

		await user.click(
			screen.getByRole("button", { name: "Valider la carte restante" }),
		);

		await screen.findByText("2 validées · 1 signalée · 0 à relire");
		expect(
			calls(api, "POST", "/api/v1/lessons/l-saluer/card-validations").map(
				(call) => call.body,
			),
		).toEqual([{ cardIds: ["c-labas"] }]);
		expect(api.content.row("c-wach-rak")?.status).toBe("to_review");
		expect(
			screen.queryByRole("button", { name: /Valider la carte restante/ }),
		).not.toBeInTheDocument();
	});

	it("flags a card inline and gives the focus back", async () => {
		const { api, user } = await open("/admin/relecture/lecons/l-saluer", true);
		const card = await screen.findByRole("article", { name: "Carte 3" });
		const flag = within(card).getByRole("button", {
			name: "Signaler la carte 3",
		});

		await user.click(flag);
		const field = within(card).getByLabelText("Qu'est-ce qui ne va pas ?");
		expect(field).toHaveFocus();
		await user.type(field, "Le sens est « bien »");
		await user.click(within(card).getByRole("button", { name: "Signaler" }));

		expect(
			await within(card).findByText("Le sens est « bien »"),
		).toBeInTheDocument();
		expect(flag).toHaveFocus();
		expect(within(card).getByText("Signalée")).toBeInTheDocument();
		expect(api.content.row("c-labas")?.reviewNote).toBe("Le sens est « bien »");
	});

	it("shows the unit's lessons and leads to the next lesson to review", async () => {
		await open("/admin/relecture/lecons/l-saluer", true);
		const side = await screen.findByRole("region", {
			name: "Leçons de l'unité",
		});

		expect(within(side).getByRole("link", { name: /Saluer/ })).toHaveAttribute(
			"aria-current",
			"page",
		);
		expect(
			within(side)
				.getAllByRole("link")
				.map((link) => link.textContent),
		).toEqual(["Saluer1/3", "Politesse1/1"]);
		expect(
			screen.getByRole("button", { name: "Leçon précédente" }),
		).toBeDisabled();
		expect(
			screen.getByRole("link", { name: "Leçon suivante" }),
		).toHaveAttribute("href", "/admin/relecture/lecons/l-politesse");
		expect(
			screen.getByRole("region", { name: "Visible des apprenants ?" }),
		).toHaveTextContent("Audio de la leçon (facultatif)");
	});
});

describe("content page", () => {
	it("leads from a lesson to its review", async () => {
		const { user } = await open("/admin/contenu");
		await screen.findByRole("table");

		await user.click(screen.getByRole("button", { name: "Saluer" }));
		await user.click(
			await screen.findByRole("link", { name: "Relire cette leçon" }),
		);

		expect(
			await screen.findByRole("article", { name: "Carte 3" }),
		).toBeInTheDocument();
	});
});
