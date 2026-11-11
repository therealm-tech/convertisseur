import { z } from "zod";
import { logger } from "@/lib/logger";

const RoleSchema = z.enum(["user", "admin", "super_admin"]);
export type Role = z.infer<typeof RoleSchema>;

const UserSchema = z.object({
	id: z.string(),
	username: z.string(),
	role: RoleSchema,
});
export type User = z.infer<typeof UserSchema>;

const SessionSchema = z.object({
	user: UserSchema,
	mustChangePassword: z.boolean(),
});
export type Session = z.infer<typeof SessionSchema>;

const ListedUserSchema = UserSchema.extend({
	createdAt: z.iso.datetime({ offset: true }),
	lastActiveAt: z.iso.datetime({ offset: true }).nullable(),
});
export type ListedUser = z.infer<typeof ListedUserSchema>;

const UserPageSchema = z.object({
	items: z.array(ListedUserSchema),
	page: z.number(),
	perPage: z.number(),
	total: z.number(),
});
export type UserPage = z.infer<typeof UserPageSchema>;

const PasswordResetSchema = z.object({ temporaryPassword: z.string() });

const FieldProblemSchema = z.object({ field: z.string(), code: z.string() });
export type FieldProblem = z.infer<typeof FieldProblemSchema>;

const ProblemSchema = z.object({
	type: z.string(),
	errors: z.array(FieldProblemSchema).optional(),
});

export const problemTypes = {
	forbidden: "urn:convertisseur:problem:forbidden",
	importedContent: "urn:convertisseur:problem:imported-content",
	notEmpty: "urn:convertisseur:problem:not-empty",
	invalidCredentials: "urn:convertisseur:problem:invalid-credentials",
	passwordChangeRequired: "urn:convertisseur:problem:password-change-required",
	tooLarge: "urn:convertisseur:problem:too-large",
	unsupportedAudio: "urn:convertisseur:problem:unsupported-audio",
	rateLimited: "urn:convertisseur:problem:rate-limited",
	usernameTaken: "urn:convertisseur:problem:username-taken",
	validation: "urn:convertisseur:problem:validation",
} as const;

export interface Credentials {
	username: string;
	password: string;
}

export interface PasswordChange {
	currentPassword: string;
	newPassword: string;
}

export interface UserQuery {
	page: number;
	perPage: number;
	q: string;
	role: Role | undefined;
}

/** A response the API answered with a problem document, or did not expect. */
export class ApiError extends Error {
	readonly status: number;
	readonly type: string | undefined;
	readonly fieldProblems: FieldProblem[];
	readonly retryAfterSeconds: number | undefined;

	constructor(
		status: number,
		type: string | undefined,
		fieldProblems: FieldProblem[],
		retryAfterSeconds: number | undefined,
	) {
		super(`API answered ${status}${type ? ` (${type})` : ""}`);
		this.name = "ApiError";
		this.status = status;
		this.type = type;
		this.fieldProblems = fieldProblems;
		this.retryAfterSeconds = retryAfterSeconds;
	}
}

export async function send(
	method: "GET" | "POST" | "PUT" | "DELETE",
	path: string,
	body?: unknown,
): Promise<Response> {
	const init: RequestInit = {
		method,
		credentials: "same-origin",
		headers:
			body === undefined
				? { Accept: "application/json" }
				: { Accept: "application/json", "Content-Type": "application/json" },
	};
	if (body !== undefined) {
		init.body = JSON.stringify(body);
	}
	const response = await fetch(path, init);
	logger.debug("api.response", { method, path, status: response.status });
	return response;
}

/** Sends a `multipart/form-data` body; the browser sets its boundary. */
export async function sendForm(
	method: "POST" | "PUT",
	path: string,
	body: FormData,
): Promise<Response> {
	const response = await fetch(path, {
		method,
		credentials: "same-origin",
		headers: { Accept: "application/json" },
		body,
	});
	logger.debug("api.response", { method, path, status: response.status });
	return response;
}

async function toError(response: Response): Promise<ApiError> {
	const retryAfter = Number(response.headers.get("Retry-After"));
	const parsed = ProblemSchema.safeParse(
		await response.json().catch(() => undefined),
	);
	if (!parsed.success) {
		logger.warn("api.unexpected_error_body", { status: response.status });
	}
	return new ApiError(
		response.status,
		parsed.success ? parsed.data.type : undefined,
		parsed.success ? (parsed.data.errors ?? []) : [],
		Number.isFinite(retryAfter) && retryAfter > 0 ? retryAfter : undefined,
	);
}

export async function expectOk(response: Response): Promise<void> {
	if (!response.ok) {
		throw await toError(response);
	}
}

/** The live session, or `null` when the browser holds none. */
export async function fetchCurrentSession(): Promise<Session | null> {
	const response = await send("GET", "/api/v1/sessions/current");
	if (response.status === 401) {
		return null;
	}
	await expectOk(response);
	return SessionSchema.parse(await response.json());
}

export async function register(credentials: Credentials): Promise<User> {
	const response = await send("POST", "/api/v1/users", credentials);
	await expectOk(response);
	return UserSchema.parse(await response.json());
}

export async function login(credentials: Credentials): Promise<Session> {
	const response = await send("POST", "/api/v1/sessions", credentials);
	await expectOk(response);
	return SessionSchema.parse(await response.json());
}

export async function logout(): Promise<void> {
	await expectOk(await send("DELETE", "/api/v1/sessions/current"));
}

const usersPath = "/api/v1/users";

function userPath(id: string): string {
	return `${usersPath}/${encodeURIComponent(id)}`;
}

export async function listUsers(query: UserQuery): Promise<UserPage> {
	const params = new URLSearchParams({
		page: String(query.page),
		perPage: String(query.perPage),
	});
	if (query.q.trim()) {
		params.set("q", query.q.trim());
	}
	if (query.role) {
		params.set("role", query.role);
	}
	const response = await send("GET", `${usersPath}?${params}`);
	await expectOk(response);
	return UserPageSchema.parse(await response.json());
}

export async function setRole(
	id: string,
	role: Exclude<Role, "super_admin">,
): Promise<void> {
	await expectOk(await send("PUT", `${userPath(id)}/role`, { role }));
}

/** Resets the password; returns the temporary one, which no call repeats. */
export async function resetPassword(id: string): Promise<string> {
	const response = await send("POST", `${userPath(id)}/password-resets`);
	await expectOk(response);
	return PasswordResetSchema.parse(await response.json()).temporaryPassword;
}

export async function deleteUser(id: string): Promise<void> {
	await expectOk(await send("DELETE", userPath(id)));
}

export async function changePassword(
	userId: string,
	change: PasswordChange,
): Promise<void> {
	await expectOk(await send("PUT", `${userPath(userId)}/password`, change));
}
