import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { type FakeApi, installFakeApi } from "./fakeApi";
import { renderAt } from "./helpers";

function seedContent(api: FakeApi) {
	const content = api.content;
	const alphabet = content.seed("unit", {
		id: "u-alphabet",
		title: "L'alphabet",
		track: "arabe",
		status: "validated",
		validatedAt: "2026-10-02T09:00:00Z",
		validatedBy: { id: "u-bilal", username: "bilal", role: "admin" },
	});
	const letters = content.seed("lesson", {
		id: "l-lettres",
		parentId: alphabet,
		title: "Les 4 formes des lettres",
	});
	content.seed("card", {
		id: "c-shukran",
		parentId: letters,
		arabic: "شُكْرًا",
		transliteration: "shukran",
		meaningFr: "merci",
		audio: {
			id: "a-1",
			contentType: "audio/mpeg",
			sizeBytes: 1_200_000,
			durationMs: 3_000,
			createdAt: "2026-11-05T10:00:00Z",
		},
	});
	const pillars = content.seed("unit", {
		id: "u-piliers",
		title: "Les piliers",
		track: "islam",
	});
	content.seed("lesson", {
		id: "l-shahada",
		parentId: pillars,
		title: "La shahada",
	});
}

async function openContentPage() {
	const api = installFakeApi();
	seedContent(api);
	api.logIn("nour");
	const user = userEvent.setup();
	await renderAt("/admin/contenu");
	await screen.findByRole("heading", { name: "Contenu" });
	await screen.findByRole("table");
	return { api, user };
}

function titles() {
	return screen
		.getAllByRole("rowheader")
		.map((cell) => within(cell).getByRole("button").textContent ?? "");
}

function detail() {
	return screen.getByRole("region", { name: "Détail" });
}

describe("content page", () => {
	it("lists units, lessons and cards, the latest edit first", async () => {
		await openContentPage();

		expect(titles()).toEqual([
			"La shahada",
			"Les piliers",
			"شُكْرًا · shukran",
			"Les 4 formes des lettres",
			"L'alphabet",
		]);
		const card = screen.getByRole("button", { name: "شُكْرًا · shukran" });
		const arabic = within(card).getByText("شُكْرًا");
		expect(arabic).toHaveAttribute("lang", "ar");
		expect(arabic).toHaveAttribute("dir", "rtl");
		expect(screen.getByRole("button", { name: "Tout · 5" })).toHaveAttribute(
			"aria-pressed",
			"true",
		);
		expect(screen.getByRole("button", { name: "À relire · 4" })).toBeVisible();
		expect(screen.getByRole("button", { name: "Validé · 1" })).toBeVisible();
	});

	it("filters by status, track and text", async () => {
		const { user } = await openContentPage();

		await user.click(screen.getByRole("button", { name: "Validé · 1" }));
		await waitFor(() => expect(titles()).toEqual(["L'alphabet"]));

		await user.click(screen.getByRole("button", { name: /^Tout/ }));
		await user.selectOptions(
			screen.getByRole("combobox", { name: "Parcours" }),
			"islam",
		);
		await waitFor(() =>
			expect(titles()).toEqual(["La shahada", "Les piliers"]),
		);
		expect(screen.getByRole("button", { name: "À relire · 2" })).toBeVisible();

		await user.selectOptions(
			screen.getByRole("combobox", { name: "Parcours" }),
			"",
		);
		await user.type(
			screen.getByRole("searchbox", { name: "Rechercher du contenu" }),
			"shuk",
		);
		await waitFor(() => expect(titles()).toEqual(["شُكْرًا · shukran"]));
	});

	it("edits a card in the detail panel, which sends it back to review", async () => {
		const { api, user } = await openContentPage();
		api.content.seed("card", {
			id: "c-wachrak",
			parentId: "l-lettres",
			arabic: "واش راك؟",
			variety: "darija",
			status: "validated",
			validatedAt: "2026-11-06T09:00:00Z",
			validatedBy: { id: "u-bilal", username: "bilal", role: "admin" },
		});
		await user.click(screen.getByRole("button", { name: "Validé · 1" }));
		await user.click(screen.getByRole("button", { name: "Tout · 6" }));

		await user.click(await screen.findByRole("button", { name: "واش راك؟" }));

		const panel = detail();
		const recto = await within(panel).findByRole("textbox", {
			name: "Recto (arabe, avec voyelles)",
		});
		expect(recto).toHaveValue("واش راك؟");
		expect(recto).toHaveAttribute("dir", "rtl");
		expect(recto).toHaveAttribute("lang", "ar");
		expect(
			within(panel).getByText(
				"Validé par bilal. Toute modification le repasse « à relire ».",
			),
		).toBeVisible();
		expect(within(panel).getByText("Téléverser un fichier")).toBeVisible();

		await user.type(
			within(panel).getByRole("textbox", { name: "Translittération" }),
			"wach rak ?",
		);
		await user.click(
			within(panel).getByRole("button", { name: "Sauvegarder" }),
		);

		expect(await screen.findByRole("status")).toHaveTextContent(
			"Modifications enregistrées.",
		);
		expect(api.calls).toContainEqual({
			method: "PUT",
			path: "/api/v1/cards/c-wachrak",
			body: {
				lessonId: "l-lettres",
				arabic: "واش راك؟",
				transliteration: "wach rak ?",
				meaningFr: "",
				variety: "darija",
				position: 2,
			},
		});
		expect(
			await within(panel).findByText(
				"En attente de relecture. Invisible pour les apprenants.",
			),
		).toBeVisible();
		expect(
			within(panel).getByRole("button", { name: "Marquer comme validé" }),
		).toBeVisible();
	});

	it("validates an item, then sends it back to review", async () => {
		const { api, user } = await openContentPage();

		await user.click(
			screen.getByRole("button", { name: "Les 4 formes des lettres" }),
		);
		const panel = detail();
		await user.click(
			await within(panel).findByRole("button", {
				name: "Marquer comme validé",
			}),
		);

		expect(
			await within(panel).findByText(
				"Validé par nour. Toute modification le repasse « à relire ».",
			),
		).toBeVisible();
		expect(api.content.row("l-lettres")?.status).toBe("validated");
		await waitFor(() =>
			expect(screen.getByRole("button", { name: "Validé · 2" })).toBeVisible(),
		);

		await user.click(
			within(panel).getByRole("button", { name: "Repasser à relire" }),
		);

		expect(
			await within(panel).findByText(
				"En attente de relecture. Invisible pour les apprenants.",
			),
		).toBeVisible();
		expect(api.content.row("l-lettres")?.status).toBe("to_review");
	});

	it("shows the audio a card has", async () => {
		const { user } = await openContentPage();

		await user.click(screen.getByRole("button", { name: "شُكْرًا · shukran" }));

		const panel = detail();
		expect(await within(panel).findByText("MP3 · 1,2 Mo · 0:03")).toBeVisible();
		expect(within(panel).getByText("Ajouté le 5 nov. 2026")).toBeVisible();
	});

	it("creates a lesson in a unit", async () => {
		const { api, user } = await openContentPage();

		await user.click(screen.getByRole("button", { name: "Nouvelle leçon" }));
		const panel = detail();
		const unit = within(panel).getByRole("combobox", { name: "Unité" });
		await waitFor(() => expect(unit).toBeEnabled());
		await user.selectOptions(unit, "u-piliers");
		await user.type(
			within(panel).getByRole("textbox", { name: "Titre" }),
			"La prière",
		);
		await user.click(within(panel).getByRole("button", { name: "Créer" }));

		expect(await screen.findByRole("status")).toHaveTextContent("Leçon créée.");
		expect(api.calls).toContainEqual({
			method: "POST",
			path: "/api/v1/lessons",
			body: { unitId: "u-piliers", title: "La prière" },
		});
		await waitFor(() => expect(titles()[0]).toBe("La prière"));
		expect(
			await within(detail()).findByRole("button", {
				name: "Marquer comme validé",
			}),
		).toBeVisible();
	});

	it("explains a card refused for its recto", async () => {
		const { user } = await openContentPage();

		await user.click(screen.getByRole("button", { name: "Nouvelle carte" }));
		const panel = detail();
		const lesson = within(panel).getByRole("combobox", { name: "Leçon" });
		await waitFor(() => expect(lesson).toBeEnabled());
		await user.selectOptions(lesson, "l-shahada");
		await user.type(
			within(panel).getByRole("textbox", {
				name: "Recto (arabe, avec voyelles)",
			}),
			"shahada",
		);
		await user.click(within(panel).getByRole("button", { name: "Créer" }));

		expect(await within(panel).findByRole("alert")).toHaveTextContent(
			"Le recto doit être écrit en lettres arabes.",
		);
	});

	it("deletes an item after confirmation", async () => {
		const { api, user } = await openContentPage();

		await user.click(screen.getByRole("button", { name: "La shahada" }));
		await user.click(
			await within(detail()).findByRole("button", { name: "Supprimer" }),
		);
		const dialog = screen.getByRole("dialog", {
			name: "Supprimer cette leçon ?",
		});
		await user.click(within(dialog).getByRole("button", { name: "Supprimer" }));

		expect(await screen.findByRole("status")).toHaveTextContent(
			"Leçon supprimée.",
		);
		expect(api.content.row("l-shahada")).toBeUndefined();
		await waitFor(() => expect(titles()).not.toContain("La shahada"));
	});
});
