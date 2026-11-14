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

async function openContentPage(path = "/admin/content") {
	const api = installFakeApi();
	seedContent(api);
	api.logIn("nour");
	const user = userEvent.setup();
	const router = await renderAt(path);
	await screen.findByRole("heading", { name: "Contenu" });
	return { api, user, router };
}

/** A lesson's cards, opened from its URL. */
async function openLesson(lessonId = "l-lettres") {
	const api = installFakeApi();
	seedContent(api);
	api.logIn("nour");
	const user = userEvent.setup();
	const router = await renderAt(`/admin/content?lesson=${lessonId}`);
	await screen.findByRole("link", { name: "Modifier la leçon" });
	return { api, user, router };
}

/** The units of each track the tree shows, and their lessons. */
async function tree() {
	await screen.findAllByRole("region", { name: /^(Arabe|Islam)$/ });
	return screen.queryAllByRole("region").flatMap((region) =>
		region.getAttribute("aria-label") === "Arabe" ||
		region.getAttribute("aria-label") === "Islam"
			? []
			: [
					[
						region.getAttribute("aria-label"),
						...within(region)
							.queryAllByRole("listitem")
							.map(
								(item) =>
									item.querySelector(".review-lesson-title")?.textContent ?? "",
							),
					],
				],
	);
}

function titles() {
	return screen
		.getAllByRole("rowheader")
		.map((cell) => within(cell).getAllByRole("link")[0]?.textContent ?? "");
}

function detail() {
	return screen.getByRole("main");
}

describe("content page", () => {
	it("draws the tracks, their units and lessons, with every count", async () => {
		await openContentPage();

		expect(await tree()).toEqual([
			["L'alphabet", "Les 4 formes des lettres"],
			["Les piliers", "La shahada"],
		]);
		const alphabet = screen.getByRole("region", { name: "L'alphabet" });
		expect(within(alphabet).getByText("1 leçon")).toBeVisible();
		expect(
			within(alphabet).getByRole("link", { name: /Les 4 formes des lettres/ }),
		).toHaveTextContent("1 carte");
		expect(
			within(alphabet).getByRole("link", { name: "Modifier" }),
		).toHaveAttribute("href", "/admin/content/units/u-alphabet");
		expect(screen.getByRole("button", { name: "Tout · 5" })).toHaveAttribute(
			"aria-pressed",
			"true",
		);
		expect(screen.getByRole("button", { name: "À relire · 4" })).toBeVisible();
		expect(screen.getByRole("button", { name: "Validé · 1" })).toBeVisible();
		expect(
			screen.getByRole("button", { name: "Sans audio · 0" }),
		).toBeInTheDocument();
	});

	it("filters the tree by status and track", async () => {
		const { user } = await openContentPage();
		await tree();

		await user.click(screen.getByRole("button", { name: "Validé · 1" }));
		expect(
			await screen.findByText("Aucun contenu ne correspond."),
		).toBeVisible();

		await user.click(screen.getByRole("button", { name: /^À relire/ }));
		expect(await tree()).toEqual([
			["L'alphabet", "Les 4 formes des lettres"],
			["Les piliers", "La shahada"],
		]);
		expect(
			screen.getByRole("link", { name: "Relire La shahada" }),
		).toHaveAttribute("href", "/admin/review/lessons/l-shahada");

		await user.selectOptions(
			screen.getByRole("combobox", { name: "Parcours" }),
			"islam",
		);
		await waitFor(async () =>
			expect(await tree()).toEqual([["Les piliers", "La shahada"]]),
		);
		expect(screen.getByRole("button", { name: "À relire · 2" })).toBeVisible();
	});

	it("narrows the tree to a unit, with a way back up", async () => {
		const { user, router } = await openContentPage(
			"/admin/content?status=to-review",
		);
		await tree();

		await user.click(screen.getByRole("link", { name: "Les piliers" }));

		await waitFor(async () =>
			expect(await tree()).toEqual([["Les piliers", "La shahada"]]),
		);
		expect(router.state.location.search).toEqual({
			status: "to-review",
			unit: "u-piliers",
		});
		const crumbs = screen.getByRole("navigation", { name: "Fil d'Ariane" });
		await user.click(within(crumbs).getByRole("link", { name: "Contenu" }));
		await waitFor(() =>
			expect(router.state.location.search).toEqual({ status: "to-review" }),
		);
	});

	it("searches units, lessons and cards, with where each sits", async () => {
		const { user, router } = await openContentPage();
		await tree();

		await user.type(
			screen.getByRole("searchbox", { name: "Rechercher du contenu" }),
			"lettres",
		);

		await waitFor(() =>
			expect(titles()).toEqual(["شُكْرًا · shukran", "Les 4 formes des lettres"]),
		);
		const card = screen.getByRole("link", { name: "شُكْرًا · shukran" });
		const arabic = within(card).getByText("شُكْرًا");
		expect(arabic).toHaveAttribute("lang", "ar");
		expect(arabic).toHaveAttribute("dir", "rtl");
		const row = card.closest("tr");
		if (!row) throw new Error("row missing");
		expect(row).toHaveTextContent(
			"Arabe › L'alphabet › Les 4 formes des lettres",
		);
		await user.click(
			within(row).getByRole("link", { name: "Les 4 formes des lettres" }),
		);

		await screen.findByRole("heading", { name: "Les 4 formes des lettres" });
		expect(router.state.location.search).toEqual({ lesson: "l-lettres" });
	});
});

describe("a lesson's cards", () => {
	it("lists them in order with their state, from the lesson's row", async () => {
		const { api, user } = await openContentPage();
		api.content.seed("card", {
			id: "c-afwan",
			parentId: "l-lettres",
			arabic: "عَفْوًا",
			transliteration: "ʿafwan",
			meaningFr: "de rien",
			status: "validated",
		});
		await tree();

		await user.click(
			screen.getByRole("link", { name: /Les 4 formes des lettres/ }),
		);

		expect(
			await screen.findByRole("heading", {
				name: "Les 4 formes des lettres",
				level: 1,
			}),
		).toBeVisible();
		await screen.findByRole("table");
		expect(titles()).toEqual(["شُكْرًا · shukran", "عَفْوًا · ʿafwan"]);
		const [shukran, afwan] = screen.getAllByRole("row").slice(1);
		expect(shukran).toHaveTextContent("merci");
		expect(shukran).toHaveTextContent("Oui");
		expect(shukran).toHaveTextContent("À relire");
		expect(afwan).toHaveTextContent("Non");
		expect(afwan).toHaveTextContent("Validée");
		expect(screen.getByText("2 cartes")).toBeVisible();
		const crumbs = screen.getByRole("navigation", { name: "Fil d'Ariane" });
		expect(
			within(crumbs).getByRole("link", { name: "L'alphabet" }),
		).toHaveAttribute("href", "/admin/content?unit=u-alphabet");
	});

	it("filters the cards, each filter counted under the others", async () => {
		const { api, user } = await openContentPage();
		api.content.seed("card", {
			parentId: "l-lettres",
			arabic: "عَفْوًا",
			transliteration: "ʿafwan",
			status: "validated",
		});
		await user.click(
			await screen.findByRole("link", { name: /Les 4 formes des lettres/ }),
		);
		await screen.findByRole("table");

		await user.click(screen.getByRole("button", { name: "Sans audio · 1" }));
		expect(titles()).toEqual(["عَفْوًا · ʿafwan"]);
		expect(screen.getByRole("button", { name: "À relire · 0" })).toBeVisible();

		await user.click(screen.getByRole("button", { name: "À relire · 0" }));
		expect(screen.getByText("Aucune carte ne correspond.")).toBeVisible();
	});

	it("leads to the review, the lesson's page and a new card in it", async () => {
		const { user, router } = await openLesson();

		expect(screen.getByRole("link", { name: "Relire" })).toHaveAttribute(
			"href",
			"/admin/review/lessons/l-lettres",
		);
		expect(
			screen.getByRole("link", { name: "Modifier la leçon" }),
		).toHaveAttribute("href", "/admin/content/lessons/l-lettres");

		await user.click(screen.getByRole("link", { name: "Nouvelle carte" }));

		const lesson = within(detail()).getByRole("combobox", { name: "Leçon" });
		await waitFor(() => expect(lesson).toHaveValue("l-lettres"));
		expect(router.state.location.search).toEqual({ parent: "l-lettres" });
	});

	it("moves to the next lesson the filters keep", async () => {
		const { user } = await openContentPage("/admin/content?status=to-review");
		await user.click(
			await screen.findByRole("link", { name: /^Les 4 formes des lettres/ }),
		);
		await screen.findByRole("table");

		const pager = screen.getByRole("navigation", { name: "Leçons" });
		await user.click(within(pager).getByRole("link", { name: "La shahada ›" }));

		expect(
			await screen.findByText("Cette leçon n'a pas encore de carte."),
		).toBeVisible();
		expect(
			screen.getByRole("heading", { name: "La shahada", level: 1 }),
		).toBeVisible();
	});

	it("says a lesson does not exist", async () => {
		const api = installFakeApi();
		seedContent(api);
		api.logIn("nour");
		await renderAt("/admin/content?lesson=l-disparue");

		expect(await screen.findByRole("alert")).toHaveTextContent(
			"Leçon introuvable.",
		);
	});
});

describe("content edit pages", () => {
	it("edits a card on its own page, which sends it back to review", async () => {
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
		await user.click(
			await screen.findByRole("link", { name: /Les 4 formes des lettres/ }),
		);
		await user.click(await screen.findByRole("link", { name: "واش راك؟" }));

		const panel = detail();
		const recto = await within(panel).findByRole("textbox", {
			name: "Recto (arabe, avec voyelles)",
		});
		expect(recto).toHaveValue("واش راك؟");
		expect(recto).toHaveAttribute("dir", "rtl");
		expect(recto).toHaveAttribute("lang", "ar");
		expect(
			within(panel).getByText(
				"Validé par bilal. Toute modification de contenu le repasse « à relire », pas un simple déplacement.",
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
		const { api, user } = await openLesson();

		await user.click(screen.getByRole("link", { name: "Modifier la leçon" }));
		const panel = detail();
		await user.click(
			await within(panel).findByRole("button", {
				name: "Marquer comme validé",
			}),
		);

		expect(
			await within(panel).findByText(
				"Validé par nour. Toute modification de contenu le repasse « à relire », pas un simple déplacement.",
			),
		).toBeVisible();
		expect(api.content.row("l-lettres")?.status).toBe("validated");

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

	it("keeps a validated item validated when it is moved", async () => {
		const { api, user } = await openLesson();
		const row = api.content.row("l-lettres");
		if (!row) throw new Error("seed missing");
		row.status = "validated";
		row.validatedAt = "2026-11-06T09:00:00Z";
		row.validatedBy = { id: "u-bilal", username: "bilal", role: "admin" };

		await user.click(screen.getByRole("link", { name: "Modifier la leçon" }));
		const panel = detail();
		const position = await within(panel).findByRole("spinbutton", {
			name: "Position",
		});
		await user.clear(position);
		await user.type(position, "2");
		await user.click(
			within(panel).getByRole("button", { name: "Sauvegarder" }),
		);

		expect(await screen.findByRole("status")).toHaveTextContent(
			"Modifications enregistrées.",
		);
		expect(api.content.row("l-lettres")?.position).toBe(2);
		expect(api.content.row("l-lettres")?.status).toBe("validated");
		expect(
			within(panel).getByRole("button", { name: "Repasser à relire" }),
		).toBeVisible();
	});

	it("shows the audio a card has", async () => {
		const { user } = await openLesson();

		await user.click(screen.getByRole("link", { name: "شُكْرًا · shukran" }));

		const panel = detail();
		expect(await within(panel).findByText("MP3 · 1,2 Mo · 0:03")).toBeVisible();
		expect(within(panel).getByText("Ajouté le 5 nov. 2026")).toBeVisible();
	});

	it("creates a lesson in a unit, then edits it", async () => {
		const { api, user, router } = await openContentPage();

		await user.click(screen.getByRole("link", { name: "Nouvelle leçon" }));
		expect(router.state.location.pathname).toBe("/admin/content/lessons/new");
		expect(
			screen.getByRole("heading", { name: "Nouvelle leçon" }),
		).toBeVisible();
		const panel = detail();
		const unit = within(panel).getByRole("combobox", { name: "Unité" });
		await waitFor(() => expect(unit).toBeEnabled());
		await user.selectOptions(unit, "u-piliers");
		await user.type(
			within(panel).getByRole("textbox", { name: "Titre" }),
			"La prière",
		);
		await user.click(within(panel).getByRole("button", { name: "Créer" }));

		expect(
			await screen.findByRole("heading", { name: "La prière" }),
		).toBeVisible();
		expect(api.calls).toContainEqual({
			method: "POST",
			path: "/api/v1/lessons",
			body: { unitId: "u-piliers", title: "La prière" },
		});
		expect(router.state.location.pathname).toMatch(
			/^\/admin\/content\/lessons\/(?!new$)[^/]+$/,
		);
		expect(
			screen.getByRole("button", { name: "Marquer comme validé" }),
		).toBeVisible();

		router.history.back();
		await waitFor(async () =>
			expect(await tree()).toContainEqual([
				"Les piliers",
				"La shahada",
				"La prière",
			]),
		);
	});

	it("explains a card refused for its recto", async () => {
		const { user } = await openContentPage();

		await user.click(screen.getByRole("link", { name: "Nouvelle carte" }));
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

	it("deletes a lesson after confirmation, back on the tree", async () => {
		const { api, user, router } = await openLesson("l-shahada");

		await user.click(screen.getByRole("link", { name: "Modifier la leçon" }));
		await user.click(
			await within(detail()).findByRole("button", { name: "Supprimer" }),
		);
		const dialog = screen.getByRole("dialog", {
			name: "Supprimer cette leçon ?",
		});
		await user.click(within(dialog).getByRole("button", { name: "Supprimer" }));

		await waitFor(() => expect(router.state.location.search).toEqual({}));
		expect(router.state.location.pathname).toBe("/admin/content");
		expect(api.content.row("l-shahada")).toBeUndefined();
		expect(await tree()).toEqual([
			["L'alphabet", "Les 4 formes des lettres"],
			["Les piliers"],
		]);
	});

	it("opens an item's page from anywhere on its row", async () => {
		const { user, router } = await openLesson();

		const row = screen
			.getByRole("link", { name: "شُكْرًا · shukran" })
			.closest("tr");
		if (!row) throw new Error("row missing");
		await user.click(within(row).getByText("merci"));

		expect(
			await screen.findByRole("heading", { name: "شُكْرًا · shukran" }),
		).toBeVisible();
		expect(router.state.location.pathname).toBe(
			"/admin/content/cards/c-shukran",
		);
	});

	it("goes back to the list with its filters", async () => {
		const { user, router } = await openContentPage();
		await tree();
		await user.click(screen.getByRole("button", { name: /^À relire/ }));
		await waitFor(() =>
			expect(router.state.location.search).toEqual({ status: "to-review" }),
		);
		const historyLength = router.history.length;

		await user.click(
			within(screen.getByRole("region", { name: "L'alphabet" })).getByRole(
				"link",
				{ name: "Modifier" },
			),
		);
		await screen.findByRole("textbox", { name: "Titre" });
		expect(router.state.location.pathname).toBe(
			"/admin/content/units/u-alphabet",
		);

		await user.click(screen.getByRole("link", { name: "Retour à la liste" }));

		await screen.findByRole("region", { name: "L'alphabet" });
		expect(router.state.location.search).toEqual({ status: "to-review" });
		expect(router.history.length).toBe(historyLength + 1);
		expect(screen.getByRole("button", { name: /^À relire/ })).toHaveAttribute(
			"aria-pressed",
			"true",
		);
	});

	it("reads the list's filters from its URL", async () => {
		await openContentPage("/admin/content?track=islam&q=piliers");

		await waitFor(() =>
			expect(titles()).toEqual(["La shahada", "Les piliers"]),
		);
		expect(screen.getByRole("combobox", { name: "Parcours" })).toHaveValue(
			"islam",
		);
		expect(
			screen.getByRole("searchbox", { name: "Rechercher du contenu" }),
		).toHaveValue("piliers");
	});

	it("opens an item from its URL, with a way to the list", async () => {
		const api = installFakeApi();
		seedContent(api);
		api.logIn("nour");
		await renderAt("/admin/content/lessons/l-shahada");

		expect(
			await screen.findByRole("heading", { name: "La shahada" }),
		).toBeVisible();
		expect(
			screen.getByRole("link", { name: "Retour à la liste" }),
		).toHaveAttribute("href", "/admin/content");
	});

	it("places an item in its track, unit and lesson", async () => {
		const api = installFakeApi();
		seedContent(api);
		api.logIn("nour");
		await renderAt("/admin/content/cards/c-shukran");

		const crumbs = await screen.findByRole("navigation", {
			name: "Fil d'Ariane",
		});
		expect(
			within(crumbs)
				.getAllByRole("link")
				.map((link) => [link.textContent, link.getAttribute("href")]),
		).toEqual([
			["Contenu", "/admin/content"],
			["Arabe", "/admin/content?track=arabe"],
			["Les 4 formes des lettres", "/admin/content?lesson=l-lettres"],
		]);
	});

	it("leads from a unit to its lessons and from a lesson to its cards", async () => {
		const { user } = await openContentPage("/admin/content?status=to-review");
		await tree();

		await user.click(
			within(screen.getByRole("region", { name: "L'alphabet" })).getByRole(
				"link",
				{ name: "Modifier" },
			),
		);
		const lessons = await screen.findByRole("link", {
			name: "Voir ses leçons",
		});
		expect(lessons).toHaveAttribute(
			"href",
			"/admin/content?status=to-review&unit=u-alphabet",
		);

		await user.click(lessons);
		await user.click(
			await screen.findByRole("link", { name: /^Les 4 formes des lettres/ }),
		);
		await user.click(
			await screen.findByRole("link", { name: "Modifier la leçon" }),
		);

		expect(
			await screen.findByRole("link", { name: "Voir les cartes" }),
		).toHaveAttribute(
			"href",
			"/admin/content?status=to-review&lesson=l-lettres",
		);
		const crumbs = screen.getByRole("navigation", { name: "Fil d'Ariane" });
		expect(
			within(crumbs).getByRole("link", { name: "L'alphabet" }),
		).toHaveAttribute(
			"href",
			"/admin/content?status=to-review&unit=u-alphabet",
		);
	});

	it("says an item does not exist", async () => {
		const api = installFakeApi();
		seedContent(api);
		api.logIn("nour");
		await renderAt("/admin/content/cards/c-disparue");

		expect(await screen.findByRole("alert")).toHaveTextContent(
			"Carte introuvable.",
		);
	});

	it("answers page not found for a kind it does not know", async () => {
		const api = installFakeApi();
		api.logIn("nour");
		await renderAt("/admin/content/chapters/u-alphabet");

		expect(
			await screen.findByRole("heading", { name: "Page introuvable" }),
		).toBeVisible();
	});
});
