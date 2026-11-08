import { vi } from "vitest";
import type { User } from "@/lib/api";

interface Account {
	user: User;
	password: string;
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
}

function json(status: number, body: unknown, headers = {}): Response {
	return new Response(JSON.stringify(body), {
		status,
		headers: { "Content-Type": "application/json", ...headers },
	});
}

export function problem(
	status: number,
	type: string,
	headers: Record<string, string> = {},
	errors?: { field: string; code: string }[],
): Response {
	return json(
		status,
		{ type: `urn:convertisseur:problem:${type}`, status, errors },
		{ ...headers, "Content-Type": "application/problem+json" },
	);
}

export function installFakeApi(): FakeApi {
	const accounts: Account[] = [
		{
			user: { id: "u-aicha", username: "aicha", role: "user" },
			password: "le bon mot de passe",
		},
	];
	let session: User | null = null;
	let offline = false;
	const overrides = new Map<string, Response>();
	const calls: Call[] = [];

	const find = (username: string) =>
		accounts.find(
			(account) =>
				account.user.username.toLowerCase() === username.toLowerCase(),
		);

	vi.spyOn(globalThis, "fetch").mockImplementation(async (input, init) => {
		const path = String(input);
		const method = init?.method ?? "GET";
		const body: unknown =
			typeof init?.body === "string" ? JSON.parse(init.body) : undefined;
		calls.push({ method, path, body });
		if (offline) {
			throw new TypeError("Failed to fetch");
		}
		const override = overrides.get(`${method} ${path}`);
		if (override) {
			overrides.delete(`${method} ${path}`);
			return override;
		}
		const credentials = body as { username: string; password: string };

		switch (`${method} ${path}`) {
			case "GET /api/v1/sessions/current":
				return session
					? json(200, { user: session })
					: problem(401, "unauthenticated");
			case "DELETE /api/v1/sessions/current":
				session = null;
				return new Response(null, { status: 204 });
			case "POST /api/v1/sessions": {
				const account = find(credentials.username);
				if (!account || account.password !== credentials.password) {
					return problem(401, "invalid-credentials");
				}
				session = account.user;
				return json(201, { user: account.user });
			}
			case "POST /api/v1/users": {
				if (find(credentials.username)) {
					return problem(409, "username-taken");
				}
				const user: User = {
					id: `u-${credentials.username}`,
					username: credentials.username,
					role: "user",
				};
				accounts.push({ user, password: credentials.password });
				return json(201, user);
			}
			default:
				return problem(404, "not-found");
		}
	});

	return {
		calls,
		logIn(username) {
			session = find(username)?.user ?? null;
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
	};
}
