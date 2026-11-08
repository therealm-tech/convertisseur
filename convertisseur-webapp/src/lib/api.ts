import { z } from "zod";
import { logger } from "@/lib/logger";

const UserSchema = z.object({
	id: z.string(),
	username: z.string(),
	role: z.enum(["user", "admin", "super_admin"]),
});
export type User = z.infer<typeof UserSchema>;

const SessionSchema = z.object({ user: UserSchema });

const FieldProblemSchema = z.object({ field: z.string(), code: z.string() });
export type FieldProblem = z.infer<typeof FieldProblemSchema>;

const ProblemSchema = z.object({
	type: z.string(),
	errors: z.array(FieldProblemSchema).optional(),
});

export const problemTypes = {
	invalidCredentials: "urn:convertisseur:problem:invalid-credentials",
	rateLimited: "urn:convertisseur:problem:rate-limited",
	usernameTaken: "urn:convertisseur:problem:username-taken",
	validation: "urn:convertisseur:problem:validation",
} as const;

export interface Credentials {
	username: string;
	password: string;
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

async function send(
	method: "GET" | "POST" | "DELETE",
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

/** The logged-in user, or `null` when the browser holds no live session. */
export async function fetchCurrentUser(): Promise<User | null> {
	const response = await send("GET", "/api/v1/sessions/current");
	if (response.status === 401) {
		return null;
	}
	if (!response.ok) {
		throw await toError(response);
	}
	return SessionSchema.parse(await response.json()).user;
}

export async function register(credentials: Credentials): Promise<User> {
	const response = await send("POST", "/api/v1/users", credentials);
	if (!response.ok) {
		throw await toError(response);
	}
	return UserSchema.parse(await response.json());
}

export async function login(credentials: Credentials): Promise<User> {
	const response = await send("POST", "/api/v1/sessions", credentials);
	if (!response.ok) {
		throw await toError(response);
	}
	return SessionSchema.parse(await response.json()).user;
}

export async function logout(): Promise<void> {
	const response = await send("DELETE", "/api/v1/sessions/current");
	if (!response.ok) {
		throw await toError(response);
	}
}
