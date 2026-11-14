import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { type FakeApi, installFakeApi, TEMPORARY_PASSWORD } from "./fakeApi";
import { renderAt } from "./helpers";

const NEW_PASSWORD = "un nouveau mot de passe";

function resetByAnAdmin(api: FakeApi, username: string) {
	const account = api.account(username);
	if (!account) {
		throw new Error(`no account ${username}`);
	}
	account.password = TEMPORARY_PASSWORD;
	account.mustChangePassword = true;
}

async function fillChange(
	user: ReturnType<typeof userEvent.setup>,
	current: string,
	next: string,
	confirmation = next,
) {
	const currentLabel = /Mot de passe (temporaire|actuel)/;
	await user.type(screen.getByLabelText(currentLabel), current);
	await user.type(screen.getByLabelText("Nouveau mot de passe"), next);
	await user.type(
		screen.getByLabelText("Confirme le nouveau mot de passe"),
		confirmation,
	);
	await user.click(screen.getByRole("button", { name: "Enregistrer" }));
}

describe("forced password change", () => {
	it("sends a reset account to the new password screen after login", async () => {
		const api = installFakeApi();
		resetByAnAdmin(api, "aicha");
		const user = userEvent.setup();
		const router = await renderAt("/login");
		const form = await screen.findByRole("tabpanel", { name: "Connexion" });

		await user.type(screen.getByLabelText("Pseudo ou adresse e-mail"), "aicha");
		await user.type(screen.getByLabelText("Mot de passe"), TEMPORARY_PASSWORD);
		await user.click(form.querySelector("button") as HTMLButtonElement);

		expect(
			await screen.findByRole("heading", {
				name: "Choisis un nouveau mot de passe",
			}),
		).toBeInTheDocument();
		expect(router.state.location.pathname).toBe("/password");
		expect(screen.getByLabelText("Mot de passe temporaire")).toBeVisible();
	});

	it.each(["/", "/admin/users", "/login"])(
		"blocks %s until the password is changed",
		async (path) => {
			const api = installFakeApi();
			resetByAnAdmin(api, "nour");
			api.logIn("nour");

			const router = await renderAt(path);

			expect(
				await screen.findByRole("heading", {
					name: "Choisis un nouveau mot de passe",
				}),
			).toBeInTheDocument();
			expect(router.state.location.pathname).toBe("/password");
		},
	);

	it("checks the new password before calling the API", async () => {
		const api = installFakeApi();
		resetByAnAdmin(api, "aicha");
		api.logIn("aicha");
		const user = userEvent.setup();
		await renderAt("/password");
		await screen.findByRole("heading", {
			name: "Choisis un nouveau mot de passe",
		});

		await fillChange(user, TEMPORARY_PASSWORD, "court", "autre");

		expect(
			screen.getByLabelText("Nouveau mot de passe"),
		).toHaveAccessibleDescription(
			expect.stringContaining("au moins 12 caractères"),
		);
		expect(
			screen.getByLabelText("Confirme le nouveau mot de passe"),
		).toHaveAccessibleDescription(
			"Les deux mots de passe ne correspondent pas.",
		);
		expect(api.calls.filter((call) => call.method === "PUT")).toHaveLength(0);
	});

	it("lands on the home page once the password is changed", async () => {
		const api = installFakeApi();
		resetByAnAdmin(api, "aicha");
		api.logIn("aicha");
		const user = userEvent.setup();
		const router = await renderAt("/password");
		await screen.findByRole("heading", {
			name: "Choisis un nouveau mot de passe",
		});

		await fillChange(user, TEMPORARY_PASSWORD, NEW_PASSWORD);

		expect(
			await screen.findByRole("heading", { name: "Salam, aicha" }),
		).toBeInTheDocument();
		expect(router.state.location.pathname).toBe("/");
		expect(api.calls).toContainEqual({
			method: "PUT",
			path: "/api/v1/users/u-aicha/password",
			body: { currentPassword: TEMPORARY_PASSWORD, newPassword: NEW_PASSWORD },
		});
	});

	it("can log out instead", async () => {
		const api = installFakeApi();
		resetByAnAdmin(api, "aicha");
		api.logIn("aicha");
		await renderAt("/password");

		await userEvent.click(
			await screen.findByRole("button", { name: "Se déconnecter" }),
		);

		expect(
			await screen.findByRole("tab", { name: "Connexion" }),
		).toBeInTheDocument();
	});
});

describe("changing your own password", () => {
	it("is reachable from the home page", async () => {
		const api = installFakeApi();
		api.logIn("aicha");
		const router = await renderAt("/");

		await userEvent.click(
			await screen.findByRole("link", { name: "Changer mon mot de passe" }),
		);

		expect(
			await screen.findByRole("heading", { name: "Changer mon mot de passe" }),
		).toBeInTheDocument();
		expect(router.state.location.pathname).toBe("/password");
	});

	it("says when the current password is wrong, then changes it", async () => {
		const api = installFakeApi();
		api.logIn("aicha");
		const user = userEvent.setup();
		await renderAt("/password");
		await screen.findByRole("heading", { name: "Changer mon mot de passe" });

		await fillChange(user, "pas le bon mot de passe", NEW_PASSWORD);

		expect(
			await screen.findByText("Ce mot de passe est incorrect."),
		).toBeInTheDocument();
		expect(screen.getByLabelText("Mot de passe actuel")).toHaveAttribute(
			"aria-invalid",
			"true",
		);

		await user.clear(screen.getByLabelText("Mot de passe actuel"));
		await user.clear(screen.getByLabelText("Nouveau mot de passe"));
		await user.clear(screen.getByLabelText("Confirme le nouveau mot de passe"));
		await fillChange(user, "le bon mot de passe", NEW_PASSWORD);

		expect(await screen.findByRole("status")).toHaveTextContent(
			"Ton mot de passe a été changé. Tes autres sessions ont été fermées.",
		);
		expect(api.account("aicha")?.password).toBe(NEW_PASSWORD);
	});
});
