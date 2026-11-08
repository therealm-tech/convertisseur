import { screen } from "@testing-library/react";
import { renderAt } from "./helpers";

describe("landing page", () => {
	it("shows the logo and the app name", async () => {
		await renderAt("/");

		expect(
			await screen.findByRole("heading", { level: 1, name: "Convertisseur" }),
		).toBeInTheDocument();
		expect(
			screen.getByRole("img", { name: "Logo de Convertisseur" }),
		).toBeVisible();
	});

	it("sends an unknown path to a not-found page that links home", async () => {
		await renderAt("/nulle-part");

		expect(
			await screen.findByRole("heading", { name: "Page introuvable" }),
		).toBeInTheDocument();
		expect(
			screen.getByRole("link", { name: "Retour à l'accueil" }),
		).toHaveAttribute("href", "/");
	});
});
