import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { installFakeApi, problem } from "./fakeApi";
import { renderAt } from "./helpers";

const FORCED_TITLE = "Ajoute ton adresse e-mail";

describe("account without an email", () => {
	it.each(["/", "/admin/users", "/login"])(
		"blocks %s until it sets one",
		async (path) => {
			const api = installFakeApi();
			api.clearEmail("nour");
			api.logIn("nour");

			const router = await renderAt(path);

			expect(
				await screen.findByRole("heading", { name: FORCED_TITLE }),
			).toBeInTheDocument();
			expect(router.state.location.pathname).toBe("/email");
			expect(screen.getByLabelText("Adresse e-mail")).toHaveValue("");
		},
	);

	it("lands on the home page once the email is set", async () => {
		const api = installFakeApi();
		api.clearEmail("chef");
		api.logIn("chef");
		const user = userEvent.setup();
		const router = await renderAt("/");
		await screen.findByRole("heading", { name: FORCED_TITLE });

		await user.type(
			screen.getByLabelText("Adresse e-mail"),
			" chef@example.com",
		);
		await user.click(screen.getByRole("button", { name: "Enregistrer" }));

		expect(
			await screen.findByRole("heading", { name: "Salam, chef" }),
		).toBeInTheDocument();
		expect(router.state.location.pathname).toBe("/");
		expect(api.account("chef")?.user.email).toBe("chef@example.com");
	});

	it("checks the email before calling the API", async () => {
		const api = installFakeApi();
		api.clearEmail("aicha");
		api.logIn("aicha");
		const user = userEvent.setup();
		await renderAt("/email");
		await screen.findByRole("heading", { name: FORCED_TITLE });

		await user.click(screen.getByRole("button", { name: "Enregistrer" }));

		expect(screen.getByLabelText("Adresse e-mail")).toHaveAccessibleDescription(
			expect.stringContaining("Saisis ton adresse e-mail."),
		);
		expect(api.calls.filter((call) => call.method === "PUT")).toHaveLength(0);
	});

	it("offers to log out instead", async () => {
		const api = installFakeApi();
		api.clearEmail("aicha");
		api.logIn("aicha");
		const user = userEvent.setup();
		const router = await renderAt("/email");

		await user.click(
			await screen.findByRole("button", { name: "Se déconnecter" }),
		);

		expect(
			await screen.findByRole("button", { name: "Se connecter" }),
		).toBeInTheDocument();
		expect(router.state.location.pathname).toBe("/login");
	});
});

describe("email change", () => {
	it("is reached from the home page, filled with the current email", async () => {
		const api = installFakeApi();
		api.logIn("aicha");
		const user = userEvent.setup();
		await renderAt("/");

		await user.click(
			await screen.findByRole("link", { name: "Changer mon adresse e-mail" }),
		);

		expect(
			await screen.findByRole("heading", {
				name: "Changer mon adresse e-mail",
			}),
		).toBeInTheDocument();
		expect(screen.getByLabelText("Adresse e-mail")).toHaveValue(
			"aicha@example.com",
		);
		expect(
			screen.getByRole("link", { name: "Retour à l'accueil" }),
		).toBeInTheDocument();
	});

	it("saves the new email and says so", async () => {
		const api = installFakeApi();
		api.logIn("aicha");
		const user = userEvent.setup();
		await renderAt("/email");
		const field = await screen.findByLabelText("Adresse e-mail");

		await user.clear(field);
		await user.type(field, "aicha.b@example.org");
		await user.click(screen.getByRole("button", { name: "Enregistrer" }));

		expect(await screen.findByRole("status")).toHaveTextContent(
			"Ton adresse e-mail a été changée.",
		);
		expect(api.account("aicha")?.user.email).toBe("aicha.b@example.org");
	});

	it("says when another account has the email", async () => {
		const api = installFakeApi();
		api.logIn("aicha");
		const user = userEvent.setup();
		await renderAt("/email");
		const field = await screen.findByLabelText("Adresse e-mail");

		await user.clear(field);
		await user.type(field, "NOUR@example.com");
		await user.click(screen.getByRole("button", { name: "Enregistrer" }));

		expect(
			await screen.findByText("Cette adresse e-mail est déjà utilisée."),
		).toBeInTheDocument();
		expect(field).toHaveAttribute("aria-invalid", "true");
	});

	it("shows the server's rule in French", async () => {
		const api = installFakeApi();
		api.logIn("aicha");
		api.failNext(
			"PUT",
			"/api/v1/users/u-aicha/email",
			problem(422, "validation", {}, [
				{ field: "email", code: "email_invalid" },
			]),
		);
		const user = userEvent.setup();
		await renderAt("/email");
		await screen.findByLabelText("Adresse e-mail");

		await user.click(screen.getByRole("button", { name: "Enregistrer" }));

		expect(
			await screen.findByText("Cette adresse e-mail n'est pas valide."),
		).toBeInTheDocument();
	});
});
