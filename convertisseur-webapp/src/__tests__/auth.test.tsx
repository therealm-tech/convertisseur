import { screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { type FakeApi, installFakeApi, problem } from "./fakeApi";
import { renderAt } from "./helpers";

async function openAuthScreen(tab: "Connexion" | "Inscription" = "Connexion") {
	const api = installFakeApi();
	await renderAt("/login");
	const user = userEvent.setup();
	await user.click(await screen.findByRole("tab", { name: tab }));
	return { api, user, form: screen.getByRole("tabpanel", { name: tab }) };
}

async function fill(
	user: ReturnType<typeof userEvent.setup>,
	form: HTMLElement,
	username: string,
	password: string,
) {
	const scope = within(form);
	if (username) {
		await user.type(scope.getByLabelText("Pseudo"), username);
	}
	if (password) {
		await user.type(scope.getByLabelText("Mot de passe"), password);
	}
}

function posted(api: FakeApi, path: string) {
	return api.calls.filter(
		(call) => call.method === "POST" && call.path === path,
	);
}

describe("auth screen", () => {
	it("shows the logo, the tabs and the login form first", async () => {
		installFakeApi();
		await renderAt("/login");

		expect(
			await screen.findByRole("img", { name: "Logo de Convertisseur" }),
		).toBeVisible();
		expect(
			screen.getByRole("tab", { name: "Connexion", selected: true }),
		).toBeInTheDocument();
		expect(
			screen.getByRole("tab", { name: "Inscription", selected: false }),
		).toBeInTheDocument();
		expect(
			screen.getByRole("button", { name: "Se connecter" }),
		).toBeInTheDocument();
		expect(screen.queryByLabelText(/mail/i)).not.toBeInTheDocument();
	});

	it("switches tabs with the arrow keys", async () => {
		installFakeApi();
		await renderAt("/login");
		const user = userEvent.setup();

		await user.click(await screen.findByRole("tab", { name: "Connexion" }));
		await user.keyboard("{ArrowRight}");

		expect(screen.getByRole("tab", { name: "Inscription" })).toHaveFocus();
		expect(
			screen.getByRole("button", { name: "Créer mon compte" }),
		).toBeInTheDocument();
	});

	describe("login", () => {
		it("asks for both fields before calling the API", async () => {
			const { api, user, form } = await openAuthScreen();

			await user.click(within(form).getByRole("button"));

			expect(within(form).getByLabelText("Pseudo")).toHaveAccessibleDescription(
				"Saisis ton pseudo.",
			);
			expect(
				within(form).getByLabelText("Mot de passe"),
			).toHaveAccessibleDescription("Saisis ton mot de passe.");
			expect(posted(api, "/api/v1/sessions")).toHaveLength(0);
		});

		it("lands on the home page once logged in", async () => {
			const { user, form } = await openAuthScreen();

			await fill(user, form, "Aicha", "le bon mot de passe");
			await user.click(within(form).getByRole("button"));

			expect(
				await screen.findByRole("heading", { name: "Salam, aicha" }),
			).toBeInTheDocument();
		});

		it("refuses wrong credentials without saying which is wrong", async () => {
			const { user, form } = await openAuthScreen();

			await fill(user, form, "aicha", "pas le bon mot de passe");
			await user.click(within(form).getByRole("button"));

			expect(await screen.findByRole("alert")).toHaveTextContent(
				"Pseudo ou mot de passe incorrect.",
			);
		});

		it("says how long to wait when rate limited", async () => {
			const { api, user, form } = await openAuthScreen();
			api.failNext(
				"POST",
				"/api/v1/sessions",
				problem(429, "rate-limited", { "Retry-After": "90" }),
			);

			await fill(user, form, "aicha", "le bon mot de passe");
			await user.click(within(form).getByRole("button"));

			expect(await screen.findByRole("alert")).toHaveTextContent(
				"Trop de tentatives. Réessaie dans 2 minutes.",
			);
		});

		it("says so when the server cannot be reached", async () => {
			const { api, user, form } = await openAuthScreen();
			api.goOffline();

			await fill(user, form, "aicha", "le bon mot de passe");
			await user.click(within(form).getByRole("button"));

			expect(await screen.findByRole("alert")).toHaveTextContent(
				"Impossible de joindre le serveur. Vérifie ta connexion.",
			);
		});
	});

	describe("registration", () => {
		it.each([
			["", "Choisis un pseudo."],
			["ab", "Ton pseudo doit faire entre 3 et 32 caractères."],
			["a".repeat(33), "Ton pseudo doit faire entre 3 et 32 caractères."],
			[
				"aïcha",
				"Ton pseudo ne peut contenir que des lettres sans accent, des chiffres, et . _ -",
			],
		])("refuses the username %j", async (username, message) => {
			const { api, user, form } = await openAuthScreen("Inscription");

			await fill(user, form, username, "un mot de passe assez long");
			await user.click(within(form).getByRole("button"));

			expect(within(form).getByLabelText("Pseudo")).toHaveAccessibleDescription(
				expect.stringContaining(message),
			);
			expect(posted(api, "/api/v1/users")).toHaveLength(0);
		});

		it("refuses a password under 12 characters", async () => {
			const { api, user, form } = await openAuthScreen("Inscription");

			await fill(user, form, "bilal", "onze caract");
			await user.click(within(form).getByRole("button"));

			expect(
				within(form).getByLabelText("Mot de passe"),
			).toHaveAccessibleDescription(
				expect.stringContaining(
					"Ton mot de passe doit faire au moins 12 caractères.",
				),
			);
			expect(posted(api, "/api/v1/users")).toHaveLength(0);
		});

		it("clears a message once the field is fixed", async () => {
			const { user, form } = await openAuthScreen("Inscription");
			const username = within(form).getByLabelText("Pseudo");

			await fill(user, form, "ab", "un mot de passe assez long");
			await user.click(within(form).getByRole("button"));
			await user.type(username, "c");

			expect(username).not.toHaveAttribute("aria-invalid");
		});

		it("creates the account, logs in and lands on the home page", async () => {
			const { api, user, form } = await openAuthScreen("Inscription");

			await fill(user, form, "bilal_92", "un mot de passe assez long");
			await user.click(within(form).getByRole("button"));

			expect(
				await screen.findByRole("heading", { name: "Salam, bilal_92" }),
			).toBeInTheDocument();
			expect(posted(api, "/api/v1/users")[0]?.body).toEqual({
				username: "bilal_92",
				password: "un mot de passe assez long",
			});
			expect(posted(api, "/api/v1/sessions")).toHaveLength(1);
		});

		it("says when the username is taken", async () => {
			const { user, form } = await openAuthScreen("Inscription");

			await fill(user, form, "AICHA", "un mot de passe assez long");
			await user.click(within(form).getByRole("button"));

			expect(
				await within(form).findByText("Ce pseudo est déjà pris."),
			).toBeInTheDocument();
			expect(within(form).getByLabelText("Pseudo")).toHaveAttribute(
				"aria-invalid",
				"true",
			);
		});

		it("shows the server's field rules in French", async () => {
			const { api, user, form } = await openAuthScreen("Inscription");
			api.failNext(
				"POST",
				"/api/v1/users",
				problem(422, "validation", {}, [
					{ field: "password", code: "password_length" },
				]),
			);

			await fill(user, form, "bilal", "un mot de passe assez long");
			await user.click(within(form).getByRole("button"));

			expect(
				await within(form).findByText(
					"Ton mot de passe doit faire au moins 12 caractères.",
				),
			).toBeInTheDocument();
		});
	});
});
