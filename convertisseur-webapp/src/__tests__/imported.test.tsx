import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { installFakeApi } from "./fakeApi";
import { renderAt } from "./helpers";

async function openContentPage() {
	const api = installFakeApi();
	const content = api.content;
	const unit = content.seed("unit", {
		id: "u-salutations",
		title: "Salutations",
		importKey: "arabe/salutations",
	});
	const lesson = content.seed("lesson", {
		id: "l-bonjour",
		parentId: unit,
		title: "Bonjour",
		importKey: "arabe/salutations/bonjour",
	});
	content.seed("card", {
		id: "c-salam",
		parentId: lesson,
		arabic: "سَلَام",
		transliteration: "salam",
		meaningFr: "Paix",
		variety: "darija",
		importKey: "arabe/salutations/bonjour/salam",
		status: "validated",
		validatedAt: "2026-11-02T09:00:00Z",
		validatedBy: { id: "u-bilal", username: "bilal", role: "admin" },
		reviewNote: "Il manque le tanwin",
	});
	content.seed("card", {
		id: "c-ajout",
		parentId: lesson,
		arabic: "صْبَاح النُّور",
		variety: "darija",
	});
	api.logIn("nour");
	const user = userEvent.setup();
	await renderAt("/admin/contenu");
	await screen.findByRole("table");
	return { api, user };
}

function detail() {
	return screen.getByRole("main");
}

function titles() {
	return screen
		.getAllByRole("rowheader")
		.map((cell) => within(cell).getByRole("link").textContent ?? "");
}

describe("imported content", () => {
	it("shows an imported item read-only, its status and audio still open", async () => {
		const { user } = await openContentPage();

		await user.click(screen.getByRole("link", { name: "سَلَام · salam" }));

		const panel = detail();
		const recto = await within(panel).findByRole("textbox", {
			name: "Recto (arabe, avec voyelles)",
		});
		expect(within(panel).getByText("Géré dans le repo")).toBeVisible();
		expect(within(panel).getByText(/se corrigent dans le dépôt/)).toBeVisible();
		expect(recto).toHaveAttribute("readonly");
		for (const name of ["Translittération", "Sens"]) {
			expect(within(panel).getByRole("textbox", { name })).toHaveAttribute(
				"readonly",
			);
		}
		expect(
			within(panel).getByRole("spinbutton", { name: "Position" }),
		).toHaveAttribute("readonly");
		expect(
			within(panel).getByRole("combobox", { name: "Variété" }),
		).toBeDisabled();
		expect(
			within(panel).getByRole("combobox", { name: "Leçon" }),
		).toBeDisabled();
		expect(
			within(panel).queryByRole("button", { name: "Sauvegarder" }),
		).not.toBeInTheDocument();
		expect(
			within(panel).queryByRole("button", { name: "Supprimer" }),
		).not.toBeInTheDocument();
		expect(
			within(panel).getByRole("button", { name: "Repasser à relire" }),
		).toBeEnabled();
		expect(within(panel).getByText("Téléverser un fichier")).toBeVisible();
	});

	it("keeps an item written in the admin area editable", async () => {
		const { user } = await openContentPage();

		await user.click(screen.getByRole("link", { name: "صْبَاح النُّور" }));

		const panel = detail();
		const recto = await within(panel).findByRole("textbox", {
			name: "Recto (arabe, avec voyelles)",
		});
		expect(recto).not.toHaveAttribute("readonly");
		expect(
			within(panel).queryByText("Géré dans le repo"),
		).not.toBeInTheDocument();
		expect(
			within(panel).getByRole("combobox", { name: "Leçon" }),
		).toBeEnabled();
		expect(
			within(panel).getByRole("button", { name: "Sauvegarder" }),
		).toBeVisible();
		expect(
			within(panel).getByRole("button", { name: "Supprimer" }),
		).toBeVisible();
	});

	it("writes and clears a review note without changing the status", async () => {
		const { api, user } = await openContentPage();
		expect(screen.getAllByText("Remarque")).toHaveLength(1);

		await user.click(screen.getByRole("link", { name: "Bonjour" }));
		const panel = detail();
		const note = await within(panel).findByRole("textbox", {
			name: "Remarque de relecture",
		});
		const save = within(panel).getByRole("button", {
			name: "Enregistrer la remarque",
		});
		expect(note).toHaveValue("");
		expect(save).toBeDisabled();

		await user.type(note, "Vocalisation fausse sur le 2e mot");
		await user.click(save);

		expect(await screen.findByRole("status")).toHaveTextContent(
			"Remarque enregistrée.",
		);
		expect(api.calls).toContainEqual({
			method: "PUT",
			path: "/api/v1/lessons/l-bonjour/review-note",
			body: { text: "Vocalisation fausse sur le 2e mot" },
		});
		expect(api.content.row("l-bonjour")?.reviewNote).toBe(
			"Vocalisation fausse sur le 2e mot",
		);
		expect(api.content.row("l-bonjour")?.status).toBe("to_review");

		await user.click(screen.getByRole("link", { name: "Retour à la liste" }));
		await waitFor(() =>
			expect(screen.getAllByText("Remarque")).toHaveLength(2),
		);

		await user.click(screen.getByRole("link", { name: "Bonjour" }));
		const reopened = await within(detail()).findByRole("textbox", {
			name: "Remarque de relecture",
		});
		await waitFor(() =>
			expect(reopened).toHaveValue("Vocalisation fausse sur le 2e mot"),
		);
		await user.clear(reopened);
		await user.click(
			within(detail()).getByRole("button", {
				name: "Enregistrer la remarque",
			}),
		);
		expect(await screen.findByRole("status")).toHaveTextContent(
			"Remarque effacée.",
		);
		expect(api.content.row("l-bonjour")?.reviewNote).toBeNull();
	});

	it("caps a review note at 2000 characters, counted as typed", async () => {
		const { user } = await openContentPage();
		await user.click(screen.getByRole("link", { name: "Salutations" }));
		const panel = detail();
		const note = await within(panel).findByRole("textbox", {
			name: "Remarque de relecture",
		});
		expect(note).toHaveAttribute("maxlength", "2000");
		expect(within(panel).getByText("0 / 2000")).toBeVisible();
		await user.type(note, "Titre");
		expect(within(panel).getByText("5 / 2000")).toBeVisible();
	});

	it("filters the items with a review note, with their count", async () => {
		const { user } = await openContentPage();
		const filter = screen.getByRole("button", { name: "Avec remarque · 1" });
		expect(filter).toHaveAttribute("aria-pressed", "false");

		await user.click(filter);

		await waitFor(() => expect(titles()).toEqual(["سَلَام · salam"]));
		expect(
			screen.getByRole("button", { name: "Avec remarque · 1" }),
		).toHaveAttribute("aria-pressed", "true");
		expect(screen.getByRole("button", { name: "Validé · 1" })).toBeVisible();
		expect(screen.getByRole("button", { name: "À relire · 0" })).toBeVisible();

		await user.click(screen.getByRole("button", { name: "À relire · 0" }));
		await waitFor(() =>
			expect(screen.getByText("Aucun contenu ne correspond.")).toBeVisible(),
		);
		expect(
			screen.getByRole("button", { name: "Avec remarque · 0" }),
		).toBeVisible();
	});
});
