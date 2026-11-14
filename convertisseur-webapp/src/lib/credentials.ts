import { z } from "zod";
import {
	ApiError,
	type FieldProblem,
	type LoginCredentials,
	problemTypes,
	type Registration,
} from "@/lib/api";

export const USERNAME_MIN_CHARS = 3;
export const USERNAME_MAX_CHARS = 32;
export const PASSWORD_MIN_CHARS = 12;

export const EMAIL_HINT = "Pour te connecter. Aucun message n'y est envoyé.";

/** What either tab of the auth screen holds; each shows its own fields. */
export type CredentialsInput = LoginCredentials & Registration;

type CredentialField = keyof CredentialsInput;

export type CredentialErrors = Partial<Record<CredentialField, string>>;

const messages = {
	usernameMissing: "Choisis un pseudo.",
	usernameLength: `Ton pseudo doit faire entre ${USERNAME_MIN_CHARS} et ${USERNAME_MAX_CHARS} caractères.`,
	usernameCharacters:
		"Ton pseudo ne peut contenir que des lettres sans accent, des chiffres, et . _ -",
	emailMissing: "Saisis ton adresse e-mail.",
	emailInvalid: "Cette adresse e-mail n'est pas valide.",
	passwordLength: `Ton mot de passe doit faire au moins ${PASSWORD_MIN_CHARS} caractères.`,
	loginMissing: "Saisis ton pseudo ou ton adresse e-mail.",
	loginPasswordMissing: "Saisis ton mot de passe.",
	usernameTaken: "Ce pseudo est déjà pris.",
	emailTaken: "Cette adresse e-mail est déjà utilisée.",
	invalidCredentials: "Pseudo, adresse e-mail ou mot de passe incorrect.",
	unexpected: "Une erreur est survenue. Réessaie dans un moment.",
	unreachable: "Impossible de joindre le serveur. Vérifie ta connexion.",
} as const;

/** The server's validation codes, which the client-side rules mirror. */
const fieldProblemMessages: Record<string, string> = {
	username_length: messages.usernameLength,
	username_characters: messages.usernameCharacters,
	email_invalid: messages.emailInvalid,
	password_length: messages.passwordLength,
};

// Looser than the server's rule, which has the last word.
const EmailSchema = z
	.string()
	.trim()
	.min(1, messages.emailMissing)
	.regex(/^[^\s@]+@[^\s@]+$/, messages.emailInvalid);

// Passwords count Unicode code points, as the server does.
const RegistrationSchema = z.object({
	username: z
		.string()
		.min(1, messages.usernameMissing)
		.min(USERNAME_MIN_CHARS, messages.usernameLength)
		.max(USERNAME_MAX_CHARS, messages.usernameLength)
		.regex(/^[A-Za-z0-9._-]*$/, messages.usernameCharacters),
	email: EmailSchema,
	password: z
		.string()
		.refine(
			(password) => [...password].length >= PASSWORD_MIN_CHARS,
			messages.passwordLength,
		),
});

const LoginSchema = z.object({
	login: z.string().trim().min(1, messages.loginMissing),
	password: z.string().min(1, messages.loginPasswordMissing),
});

function firstErrorPerField(
	schema: typeof RegistrationSchema | typeof LoginSchema,
	credentials: CredentialsInput,
): CredentialErrors {
	const result = schema.safeParse(credentials);
	const errors: CredentialErrors = {};
	for (const issue of result.error?.issues ?? []) {
		const field = issue.path[0];
		if (isCredentialField(field) && !errors[field]) {
			errors[field] = issue.message;
		}
	}
	return errors;
}

function isCredentialField(field: unknown): field is CredentialField {
	return (
		field === "login" ||
		field === "username" ||
		field === "email" ||
		field === "password"
	);
}

export function validateRegistration(
	credentials: CredentialsInput,
): CredentialErrors {
	return firstErrorPerField(RegistrationSchema, credentials);
}

export function validateLogin(credentials: CredentialsInput): CredentialErrors {
	return firstErrorPerField(LoginSchema, credentials);
}

export function validateEmail(email: string): string | undefined {
	return EmailSchema.safeParse(email).error?.issues[0]?.message;
}

export function retryDelay(seconds: number | undefined): string {
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
		if (isCredentialField(field) && message) {
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
		case problemTypes.emailTaken:
			return { fields: { email: messages.emailTaken }, form: undefined };
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
