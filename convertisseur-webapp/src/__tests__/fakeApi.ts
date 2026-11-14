import { vi } from "vitest";
import type { ListedUser, Role, User } from "@/lib/api";
import { FakeContent } from "./fakeContent";
import { json, problem } from "./responses";

export { problem };

interface Account {
	user: User;
	password: string;
	mustChangePassword: boolean;
	createdAt: string;
	lastActiveAt: string | null;
}

interface Call {
	method: string;
	path: string;
	body: unknown;
}

/** An in-memory stand-in for the sessions and users endpoints. */
export interface FakeApi {
	calls: Call[];
	/** Logs `username` in, as if the browser already held its cookie. */
	logIn(username: string): void;
	/** Ends the session on the server side, as an expiry would. */
	expireSession(): void;
	/** Answers the next request to `method path` with `response` instead. */
	failNext(method: string, path: string, response: Response): void;
	/** Makes every request fail as if the network were down. */
	goOffline(): void;
	/** The account named `username`, as the server holds it. */
	account(username: string): Account | undefined;
	/** Leaves `username` without an email, as the bootstrapped super admin. */
	clearEmail(username: string): void;
	/** The units, lessons and cards, as the server holds them. */
	content: FakeContent;
}

export const TEMPORARY_PASSWORD = "k7mqx-4tn9c-wr2hp-a8vjd";

const rank: Record<Role, number> = { user: 0, admin: 1, super_admin: 2 };

function seed(
	id: string,
	username: string,
	role: Role,
	lastActiveAt: string | null = null,
): Account {
	return {
		user: { id, username, email: `${username}@example.com`, role },
		password: "le bon mot de passe",
		mustChangePassword: false,
		createdAt: "2026-09-12T10:00:00Z",
		lastActiveAt,
	};
}

const memberPath =
	/^\/api\/v1\/users\/([^/]+)(\/role|\/password|\/password-resets|\/email)?$/;

const contentPath = /^\/api\/v1\/(content-items|units|lessons|cards)(\/|$)/;

export function installFakeApi(): FakeApi {
	const accounts: Account[] = [
		seed("u-chef", "chef", "super_admin", "2026-11-07T10:00:00Z"),
		seed("u-nour", "nour", "admin"),
		seed("u-bilal", "bilal", "admin"),
		seed("u-aicha", "aicha", "user"),
		seed("u-karim", "karim.dz", "user"),
	];
	let session: Account | null = null;
	const content = new FakeContent();
	let offline = false;
	const overrides = new Map<string, Response>();
	const calls: Call[] = [];

	const find = (username: string) =>
		accounts.find(
			(account) =>
				account.user.username.toLowerCase() === username.toLowerCase(),
		);
	const findByEmail = (email: string) =>
		accounts.find(
			(account) => account.user.email?.toLowerCase() === email.toLowerCase(),
		);
	const byId = (id: string) =>
		accounts.find((account) => account.user.id === id);
	const sessionBody = (account: Account) => ({
		user: account.user,
		mustChangePassword: account.mustChangePassword,
	});

	// The session of an endpoint that needs no pending password change and
	// at least the `minimum` role, or the problem the server would answer.
	function guard(minimum: Role): Response | Account {
		if (!session) {
			return problem(401, "unauthenticated");
		}
		if (session.mustChangePassword) {
			return problem(403, "password-change-required");
		}
		if (rank[session.user.role] < rank[minimum]) {
			return problem(403, "forbidden");
		}
		return session;
	}

	function list(url: URL): Response {
		const q = (url.searchParams.get("q") ?? "").toLowerCase();
		const role = url.searchParams.get("role");
		const page = Number(url.searchParams.get("page") ?? 1);
		const perPage = Number(url.searchParams.get("perPage") ?? 50);
		const matching = accounts
			.filter(
				({ user }) =>
					(user.username.toLowerCase().includes(q) ||
						(user.email ?? "").toLowerCase().includes(q)) &&
					(!role || user.role === role),
			)
			.sort(
				(a, b) =>
					rank[b.user.role] - rank[a.user.role] ||
					a.user.username.localeCompare(b.user.username),
			);
		const items: ListedUser[] = matching
			.slice((page - 1) * perPage, page * perPage)
			.map((account) => ({
				...account.user,
				createdAt: account.createdAt,
				lastActiveAt: account.lastActiveAt,
			}));
		return json(200, { items, page, perPage, total: matching.length });
	}

	function changePassword(target: Account | undefined, body: unknown) {
		if (!session) {
			return problem(401, "unauthenticated");
		}
		if (session !== target) {
			return problem(403, "forbidden");
		}
		const change = body as { currentPassword: string; newPassword: string };
		if (change.currentPassword !== session.password) {
			return problem(422, "validation", {}, [
				{ field: "currentPassword", code: "password_incorrect" },
			]);
		}
		session.password = change.newPassword;
		session.mustChangePassword = false;
		return new Response(null, { status: 204 });
	}

	function setEmail(target: Account | undefined, body: unknown) {
		const actor = guard("user");
		if (actor instanceof Response) {
			return actor;
		}
		if (actor !== target) {
			return problem(403, "forbidden");
		}
		const { email } = body as { email: string };
		const holder = findByEmail(email);
		if (holder && holder !== actor) {
			return problem(409, "email-taken");
		}
		actor.user = { ...actor.user, email };
		return json(200, { email });
	}

	function member(
		method: string,
		target: Account | undefined,
		sub: string,
		body: unknown,
	): Response {
		if (`${method} ${sub}` === "PUT /password") {
			return changePassword(target, body);
		}
		if (`${method} ${sub}` === "PUT /email") {
			return setEmail(target, body);
		}
		const actor = guard(sub === "/role" ? "super_admin" : "admin");
		if (actor instanceof Response) {
			return actor;
		}
		if (method === "DELETE" && sub === "") {
			if (target) {
				accounts.splice(accounts.indexOf(target), 1);
			}
			return new Response(null, { status: 204 });
		}
		if (!target) {
			return problem(404, "not-found");
		}
		if (method === "POST" && sub === "/password-resets") {
			target.password = TEMPORARY_PASSWORD;
			target.mustChangePassword = true;
			return json(201, { temporaryPassword: TEMPORARY_PASSWORD });
		}
		if (method === "PUT" && sub === "/role") {
			const { role } = body as { role: Role };
			target.user = { ...target.user, role };
			return json(200, { role });
		}
		return problem(404, "not-found");
	}

	vi.spyOn(globalThis, "fetch").mockImplementation(async (input, init) => {
		const path = String(input);
		const method = init?.method ?? "GET";
		const body: unknown =
			typeof init?.body === "string" ? JSON.parse(init.body) : init?.body;
		calls.push({ method, path, body });
		if (offline) {
			throw new TypeError("Failed to fetch");
		}
		const override = overrides.get(`${method} ${path}`);
		if (override) {
			overrides.delete(`${method} ${path}`);
			return override;
		}
		const url = new URL(path, "http://localhost");
		const memberMatch = url.pathname.match(memberPath);
		if (memberMatch) {
			const [, id = "", sub = ""] = memberMatch;
			return member(method, byId(decodeURIComponent(id)), sub, body);
		}
		if (url.pathname.startsWith("/api/v1/tracks") && method === "GET") {
			const actor = guard("user");
			return actor instanceof Response ? actor : content.study(url, actor.user);
		}
		if (
			url.pathname.startsWith("/api/v1/card-views/") ||
			url.pathname === "/api/v1/exercise-attempts"
		) {
			const actor = guard("user");
			return actor instanceof Response
				? actor
				: content.record(method, url, body, actor.user);
		}
		if (contentPath.test(url.pathname)) {
			const actor = guard("admin");
			if (actor instanceof Response) {
				return actor;
			}
			return content.handle(method, url, body, actor.user);
		}
		const credentials = body as {
			login: string;
			username: string;
			email: string;
			password: string;
		};

		switch (`${method} ${url.pathname}`) {
			case "GET /api/v1/sessions/current":
				return session
					? json(200, sessionBody(session))
					: problem(401, "unauthenticated");
			case "DELETE /api/v1/sessions/current":
				session = null;
				return new Response(null, { status: 204 });
			case "POST /api/v1/sessions": {
				const account =
					find(credentials.login) ?? findByEmail(credentials.login);
				if (!account || account.password !== credentials.password) {
					return problem(401, "invalid-credentials");
				}
				session = account;
				return json(201, sessionBody(account));
			}
			case "POST /api/v1/users": {
				if (find(credentials.username)) {
					return problem(409, "username-taken");
				}
				if (findByEmail(credentials.email)) {
					return problem(409, "email-taken");
				}
				const account = seed(
					`u-${credentials.username}`,
					credentials.username,
					"user",
				);
				account.user.email = credentials.email;
				account.password = credentials.password;
				accounts.push(account);
				return json(201, account.user);
			}
			case "GET /api/v1/users": {
				const actor = guard("admin");
				return actor instanceof Response ? actor : list(url);
			}
			default:
				return problem(404, "not-found");
		}
	});

	return {
		calls,
		logIn(username) {
			session = find(username) ?? null;
		},
		expireSession() {
			session = null;
		},
		failNext(method, path, response) {
			overrides.set(`${method} ${path}`, response);
		},
		goOffline() {
			offline = true;
		},
		account(username) {
			return find(username);
		},
		clearEmail(username) {
			const account = find(username);
			if (account) {
				account.user = { ...account.user, email: null };
			}
		},
		content,
	};
}
