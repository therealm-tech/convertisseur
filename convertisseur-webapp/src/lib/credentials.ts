import { z } from "zod";
import {
	ApiError,
	type Credentials,
	type FieldProblem,
	problemTypes,
} from "@/lib/api";

export const USERNAME_MIN_CHARS = 3;
export const USERNAME_MAX_CHARS = 32;
export const PASSWORD_MIN_CHARS = 12;

export type CredentialErrors = Partial<Record<keyof Credentials, string>>;

const messages = {
	usernameMissing: "Choisis un pseudo.",
	usernameLength: `Ton pseudo doit faire entre ${USERNAME_MIN_CHARS} et ${USERNAME_MAX_CHARS} caractères.`,
	usernameCharacters:
		"Ton pseudo ne peut contenir que des lettres sans accent, des chiffres, et . _ -",
	passwordLength: `Ton mot de passe doit faire au moins ${PASSWORD_MIN_CHARS} caractères.`,
	loginUsernameMissing: "Saisis ton pseudo.",
	loginPasswordMissing: "Saisis ton mot de passe.",
	usernameTaken: "Ce pseudo est déjà pris.",
	invalidCredentials: "Pseudo ou mot de passe incorrect.",
	unexpected: "Une erreur est survenue. Réessaie dans un moment.",
	unreachable: "Impossible de joindre le serveur. Vérifie ta connexion.",
} as const;

/** The server's validation codes, which the client-side rules mirror. */
const fieldProblemMessages: Record<string, string> = {
	username_length: messages.usernameLength,
	username_characters: messages.usernameCharacters,
	password_length: messages.passwordLength,
};

// Passwords count Unicode code points, as the server does.
const RegistrationSchema = z.object({
	username: z
		.string()
		.min(1, messages.usernameMissing)
		.min(USERNAME_MIN_CHARS, messages.usernameLength)
		.max(USERNAME_MAX_CHARS, messages.usernameLength)
		.regex(/^[A-Za-z0-9._-]*$/, messages.usernameCharacters),
	password: z
		.string()
		.refine(
			(password) => [...password].length >= PASSWORD_MIN_CHARS,
			messages.passwordLength,
		),
});

const LoginSchema = z.object({
	username: z.string().trim().min(1, messages.loginUsernameMissing),
	password: z.string().min(1, messages.loginPasswordMissing),
});

function firstErrorPerField(
	schema: typeof RegistrationSchema | typeof LoginSchema,
	credentials: Credentials,
): CredentialErrors {
	const result = schema.safeParse(credentials);
	const errors: CredentialErrors = {};
	for (const issue of result.error?.issues ?? []) {
		const field = issue.path[0];
		if ((field === "username" || field === "password") && !errors[field]) {
			errors[field] = issue.message;
		}
	}
	return errors;
}

export function validateRegistration(
	credentials: Credentials,
): CredentialErrors {
	return firstErrorPerField(RegistrationSchema, credentials);
}

export function validateLogin(credentials: Credentials): CredentialErrors {
	return firstErrorPerField(LoginSchema, credentials);
}

function retryDelay(seconds: number | undefined): string {
	if (seconds === undefined) {
		return "un moment";
	}
	if (seconds < 60) {
		return `${seconds} seconde${seconds > 1 ? "s" : ""}`;
	}
	const minutes = Math.ceil(seconds / 60);
	return `${minutes} minute${minutes > 1 ? "s" : ""}`;
}

/** What a failed login or registration means for the person filling it in. */
export interface SubmitFailure {
	fields: CredentialErrors;
	form: string | undefined;
}

function fieldsFrom(problems: FieldProblem[]): CredentialErrors {
	const fields: CredentialErrors = {};
	for (const { field, code } of problems) {
		const message = fieldProblemMessages[code];
		if ((field === "username" || field === "password") && message) {
			fields[field] ??= message;
		}
	}
	return fields;
}

export function describeFailure(error: unknown): SubmitFailure {
	if (!(error instanceof ApiError)) {
		return { fields: {}, form: messages.unreachable };
	}
	switch (error.type) {
		case problemTypes.usernameTaken:
			return { fields: { username: messages.usernameTaken }, form: undefined };
		case problemTypes.invalidCredentials:
			return { fields: {}, form: messages.invalidCredentials };
		case problemTypes.rateLimited:
			return {
				fields: {},
				form: `Trop de tentatives. Réessaie dans ${retryDelay(error.retryAfterSeconds)}.`,
			};
		case problemTypes.validation: {
			const fields = fieldsFrom(error.fieldProblems);
			return Object.keys(fields).length > 0
				? { fields, form: undefined }
				: { fields: {}, form: messages.unexpected };
		}
		default:
			return { fields: {}, form: messages.unexpected };
	}
}
