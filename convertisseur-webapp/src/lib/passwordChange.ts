import { ApiError, type PasswordChange, problemTypes } from "@/lib/api";
import { PASSWORD_MIN_CHARS, retryDelay } from "@/lib/credentials";

export interface PasswordChangeForm extends PasswordChange {
	confirmation: string;
}

export type PasswordChangeErrors = Partial<
	Record<keyof PasswordChangeForm, string>
>;

const messages = {
	currentMissing: "Saisis ton mot de passe actuel.",
	temporaryMissing: "Saisis le mot de passe temporaire.",
	currentIncorrect: "Ce mot de passe est incorrect.",
	newLength: `Ton nouveau mot de passe doit faire au moins ${PASSWORD_MIN_CHARS} caractères.`,
	newUnchanged: "Choisis un mot de passe différent de l'actuel.",
	confirmationMismatch: "Les deux mots de passe ne correspondent pas.",
	unexpected: "Une erreur est survenue. Réessaie dans un moment.",
	unreachable: "Impossible de joindre le serveur. Vérifie ta connexion.",
} as const;

// Passwords count Unicode code points, as the server does.
export function validatePasswordChange(
	form: PasswordChangeForm,
	forced: boolean,
): PasswordChangeErrors {
	const errors: PasswordChangeErrors = {};
	if (!form.currentPassword) {
		errors.currentPassword = forced
			? messages.temporaryMissing
			: messages.currentMissing;
	}
	if ([...form.newPassword].length < PASSWORD_MIN_CHARS) {
		errors.newPassword = messages.newLength;
	} else if (form.newPassword === form.currentPassword) {
		errors.newPassword = messages.newUnchanged;
	}
	if (form.confirmation !== form.newPassword) {
		errors.confirmation = messages.confirmationMismatch;
	}
	return errors;
}

const fieldProblemMessages: Record<string, string> = {
	password_incorrect: messages.currentIncorrect,
	password_length: messages.newLength,
	password_unchanged: messages.newUnchanged,
};

export interface PasswordChangeFailure {
	fields: PasswordChangeErrors;
	form: string | undefined;
}

export function describePasswordChangeFailure(
	error: unknown,
): PasswordChangeFailure {
	if (!(error instanceof ApiError)) {
		return { fields: {}, form: messages.unreachable };
	}
	if (error.type === problemTypes.rateLimited) {
		return {
			fields: {},
			form: `Trop de tentatives. Réessaie dans ${retryDelay(error.retryAfterSeconds)}.`,
		};
	}
	const fields: PasswordChangeErrors = {};
	if (error.type === problemTypes.validation) {
		for (const { field, code } of error.fieldProblems) {
			const message = fieldProblemMessages[code];
			if (message && (field === "currentPassword" || field === "newPassword")) {
				fields[field] ??= message;
			}
		}
	}
	return Object.keys(fields).length > 0
		? { fields, form: undefined }
		: { fields: {}, form: messages.unexpected };
}
