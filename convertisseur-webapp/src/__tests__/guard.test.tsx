import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { installFakeApi } from "./fakeApi";
import { renderAt } from "./helpers";

describe("route guard", () => {
	it("sends a visitor without a session to the auth screen", async () => {
		installFakeApi();

		const router = await renderAt("/");

		expect(
			await screen.findByRole("tab", { name: "Connexion", selected: true }),
		).toBeInTheDocument();
		expect(router.state.location.pathname).toBe("/login");
	});

	it("greets a logged-in user on the home page", async () => {
		const api = installFakeApi();
		api.logIn("aicha");

		await renderAt("/");

		expect(
			await screen.findByRole("heading", { name: "Salam, aicha" }),
		).toBeInTheDocument();
	});

	it("sends a logged-in user away from the auth screen", async () => {
		const api = installFakeApi();
		api.logIn("aicha");

		const router = await renderAt("/login");

		expect(
			await screen.findByRole("heading", { name: "Salam, aicha" }),
		).toBeInTheDocument();
		expect(router.state.location.pathname).toBe("/");
	});

	it("logs out and returns to the auth screen", async () => {
		const api = installFakeApi();
		api.logIn("aicha");
		await renderAt("/");

		await userEvent.click(
			await screen.findByRole("button", { name: "Se déconnecter" }),
		);

		expect(
			await screen.findByRole("tab", { name: "Connexion" }),
		).toBeInTheDocument();
		expect(api.calls).toContainEqual({
			method: "DELETE",
			path: "/api/v1/sessions/current",
			body: undefined,
		});
	});

	it("follows a session that ends on the server", async () => {
		const api = installFakeApi();
		api.logIn("aicha");
		const router = await renderAt("/");
		await screen.findByRole("heading", { name: "Salam, aicha" });

		api.expireSession();
		await router.invalidate();

		expect(
			await screen.findByRole("tab", { name: "Connexion" }),
		).toBeInTheDocument();
	});

	it("says so when the server cannot be reached", async () => {
		const api = installFakeApi();
		api.goOffline();

		await renderAt("/");

		expect(
			await screen.findByText(
				"Impossible de joindre le serveur. Vérifie ta connexion.",
			),
		).toBeInTheDocument();
		expect(
			screen.getByRole("button", { name: "Réessayer" }),
		).toBeInTheDocument();
	});

	it("sends an unknown path to a not-found page that links home", async () => {
		installFakeApi();

		await renderAt("/nulle-part");

		expect(
			await screen.findByRole("heading", { name: "Page introuvable" }),
		).toBeInTheDocument();
		expect(
			screen.getByRole("link", { name: "Retour à l'accueil" }),
		).toHaveAttribute("href", "/");
	});
});
