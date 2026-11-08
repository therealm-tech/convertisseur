import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { vi } from "vitest";
import { installFakeApi, TEMPORARY_PASSWORD } from "./fakeApi";
import { renderAt } from "./helpers";

async function openUsersPage(username: string) {
	const api = installFakeApi();
	api.logIn(username);
	const user = userEvent.setup();
	const router = await renderAt("/admin/utilisateurs");
	await screen.findByRole("heading", { name: "Utilisateurs et rôles" });
	await screen.findByRole("table");
	return { api, user, router };
}

function row(username: string) {
	return screen.getByRole("rowheader", { name: new RegExp(`^${username}`) })
		.parentElement as HTMLElement;
}

function usernames() {
	return screen
		.getAllByRole("rowheader")
		.map((cell) => cell.firstChild?.textContent);
}

describe("admin area guard", () => {
	it("sends a visitor without a session to the auth screen", async () => {
		installFakeApi();

		const router = await renderAt("/admin/utilisateurs");

		expect(router.state.location.pathname).toBe("/connexion");
	});

	it("keeps a user out and hides the link", async () => {
		const api = installFakeApi();
		api.logIn("aicha");

		const router = await renderAt("/admin/utilisateurs");

		expect(
			await screen.findByRole("heading", { name: "Salam, aicha" }),
		).toBeInTheDocument();
		expect(router.state.location.pathname).toBe("/");
		expect(
			screen.queryByRole("link", { name: "Administration" }),
		).not.toBeInTheDocument();
	});

	it.each(["nour", "chef"])("opens the admin area to %s", async (username) => {
		const api = installFakeApi();
		api.logIn(username);
		const router = await renderAt("/");

		await userEvent.click(
			await screen.findByRole("link", { name: "Administration" }),
		);

		expect(
			await screen.findByRole("heading", { name: "Utilisateurs et rôles" }),
		).toBeInTheDocument();
		expect(router.state.location.pathname).toBe("/admin/utilisateurs");
		expect(
			screen.getByRole("link", { name: "Utilisateurs et rôles" }),
		).toHaveAttribute("aria-current", "page");
	});

	it("sends /admin to the users page", async () => {
		const api = installFakeApi();
		api.logIn("nour");

		const router = await renderAt("/admin");

		expect(router.state.location.pathname).toBe("/admin/utilisateurs");
	});
});

describe("users page", () => {
	it("lists accounts with their role, sign-up and last activity", async () => {
		await openUsersPage("nour");

		expect(usernames()).toEqual(["chef", "bilal", "nour", "aicha", "karim.dz"]);
		const chef = row("chef");
		expect(within(chef).getByText("super admin · verrouillé")).toBeVisible();
		expect(within(chef).getByText("12 sept. 2026")).toBeVisible();
		expect(within(row("aicha")).getByText("Jamais")).toBeVisible();
		expect(within(row("nour")).getByText("toi")).toBeVisible();
	});

	it("lets an admin manage users only, without role selects", async () => {
		await openUsersPage("nour");

		expect(
			screen.queryByRole("combobox", { name: /^Rôle de/ }),
		).not.toBeInTheDocument();
		for (const username of ["aicha", "karim.dz"]) {
			expect(
				within(row(username)).getByRole("button", {
					name: `Supprimer ${username}`,
				}),
			).toBeVisible();
			expect(
				within(row(username)).getByRole("button", {
					name: `Réinitialiser le mot de passe de ${username}`,
				}),
			).toBeVisible();
		}
		for (const username of ["chef", "bilal", "nour"]) {
			expect(within(row(username)).queryByRole("button")).toBeNull();
			expect(within(row(username)).getByText("Verrouillé")).toBeVisible();
		}
	});

	it("lets the super admin manage admins and change roles", async () => {
		await openUsersPage("chef");

		for (const username of ["bilal", "nour", "aicha", "karim.dz"]) {
			expect(
				within(row(username)).getByRole("combobox", {
					name: `Rôle de ${username}`,
				}),
			).toBeVisible();
			expect(
				within(row(username)).getByRole("button", {
					name: `Supprimer ${username}`,
				}),
			).toBeVisible();
		}
		expect(within(row("chef")).queryByRole("combobox")).toBeNull();
		expect(within(row("chef")).queryByRole("button")).toBeNull();
	});

	it("changes a role", async () => {
		const { api, user } = await openUsersPage("chef");

		await user.selectOptions(
			screen.getByRole("combobox", { name: "Rôle de aicha" }),
			"admin",
		);

		expect(await screen.findByRole("status")).toHaveTextContent(
			"Rôle de aicha : admin.",
		);
		expect(api.calls).toContainEqual({
			method: "PUT",
			path: "/api/v1/users/u-aicha/role",
			body: { role: "admin" },
		});
		await waitFor(() =>
			expect(usernames()).toEqual([
				"chef",
				"aicha",
				"bilal",
				"nour",
				"karim.dz",
			]),
		);
	});

	it("searches usernames and filters roles", async () => {
		const { user } = await openUsersPage("nour");

		await user.type(
			screen.getByRole("searchbox", { name: "Rechercher un utilisateur" }),
			"KAR",
		);
		await waitFor(() => expect(usernames()).toEqual(["karim.dz"]));

		await user.clear(screen.getByRole("searchbox"));
		await user.selectOptions(
			screen.getByRole("combobox", { name: "Rôle" }),
			"admin",
		);
		await waitFor(() => expect(usernames()).toEqual(["bilal", "nour"]));
	});

	it("shows the temporary password once, with a copy button", async () => {
		const { api, user } = await openUsersPage("nour");

		await user.click(
			screen.getByRole("button", {
				name: "Réinitialiser le mot de passe de aicha",
			}),
		);
		const confirm = screen.getByRole("dialog", {
			name: "Réinitialiser le mot de passe de aicha ?",
		});
		await user.click(
			within(confirm).getByRole("button", { name: "Réinitialiser" }),
		);

		const dialog = await screen.findByRole("dialog", {
			name: "Nouveau mot de passe de aicha",
		});
		expect(within(dialog).getByText(TEMPORARY_PASSWORD)).toBeVisible();
		expect(
			within(dialog).getByText(/ne sera plus jamais affiché/),
		).toBeVisible();
		await user.click(within(dialog).getByRole("button", { name: "Copier" }));
		expect(await navigator.clipboard.readText()).toBe(TEMPORARY_PASSWORD);
		expect(within(dialog).getByRole("status")).toHaveTextContent(
			"Copié dans le presse-papiers.",
		);

		await user.click(within(dialog).getByRole("button", { name: "Fermer" }));

		expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
		expect(screen.queryByText(TEMPORARY_PASSWORD)).not.toBeInTheDocument();
		expect(api.account("aicha")?.mustChangePassword).toBe(true);
	});

	it("cancels a reset without calling the API", async () => {
		const { api, user } = await openUsersPage("nour");

		await user.click(
			screen.getByRole("button", {
				name: "Réinitialiser le mot de passe de aicha",
			}),
		);
		await user.keyboard("{Escape}");

		expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
		expect(
			api.calls.some((call) => call.path.endsWith("/password-resets")),
		).toBe(false);
	});

	it("deletes an account after an in-app confirmation", async () => {
		const confirmSpy = vi.spyOn(window, "confirm");
		const { api, user } = await openUsersPage("nour");

		await user.click(screen.getByRole("button", { name: "Supprimer aicha" }));
		const dialog = screen.getByRole("dialog", { name: "Supprimer aicha ?" });
		await user.click(within(dialog).getByRole("button", { name: "Annuler" }));
		expect(api.account("aicha")).toBeDefined();

		await user.click(screen.getByRole("button", { name: "Supprimer aicha" }));
		await user.click(
			within(screen.getByRole("dialog")).getByRole("button", {
				name: "Supprimer le compte",
			}),
		);

		expect(await screen.findByRole("status")).toHaveTextContent(
			"Le compte de aicha a été supprimé.",
		);
		await waitFor(() =>
			expect(usernames()).toEqual(["chef", "bilal", "nour", "karim.dz"]),
		);
		expect(api.account("aicha")).toBeUndefined();
		expect(confirmSpy).not.toHaveBeenCalled();
	});

	it("leaves when the session loses its admin role", async () => {
		const { api, user, router } = await openUsersPage("nour");
		const nour = api.account("nour");
		if (nour) {
			nour.user = { ...nour.user, role: "user" };
		}

		await user.selectOptions(
			screen.getByRole("combobox", { name: "Rôle" }),
			"user",
		);

		await waitFor(() => expect(router.state.location.pathname).toBe("/"));
	});
});
