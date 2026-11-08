import { type FormEvent, useState } from "react";
import { TextField } from "@/components/TextField";
import type { Credentials } from "@/lib/api";
import {
	type CredentialErrors,
	describeFailure,
	PASSWORD_MIN_CHARS,
	USERNAME_MAX_CHARS,
	USERNAME_MIN_CHARS,
} from "@/lib/credentials";
import { logger } from "@/lib/logger";

interface CredentialsFormProps {
	kind: "login" | "registration";
	labelledBy: string;
	submitLabel: string;
	validate: (credentials: Credentials) => CredentialErrors;
	submit: (credentials: Credentials) => Promise<void>;
}

/** The username and password form behind both tabs of the auth screen. */
export function CredentialsForm({
	kind,
	labelledBy,
	submitLabel,
	validate,
	submit,
}: CredentialsFormProps) {
	const [credentials, setCredentials] = useState<Credentials>({
		username: "",
		password: "",
	});
	const [attempted, setAttempted] = useState(false);
	const [serverErrors, setServerErrors] = useState<CredentialErrors>({});
	const [formError, setFormError] = useState<string | undefined>();
	const [submitting, setSubmitting] = useState(false);

	const localErrors = attempted ? validate(credentials) : {};
	const errors = { ...serverErrors, ...localErrors };
	const registering = kind === "registration";

	function update(field: keyof Credentials, value: string) {
		setCredentials((current) => ({ ...current, [field]: value }));
		setServerErrors((current) => ({ ...current, [field]: undefined }));
		setFormError(undefined);
	}

	async function onSubmit(event: FormEvent<HTMLFormElement>) {
		event.preventDefault();
		setAttempted(true);
		if (Object.keys(validate(credentials)).length > 0) {
			return;
		}
		setSubmitting(true);
		try {
			await submit(credentials);
		} catch (error) {
			logger.info("auth.submit_failed", { kind, error });
			const failure = describeFailure(error);
			setServerErrors(failure.fields);
			setFormError(failure.form);
			setSubmitting(false);
		}
	}

	return (
		<form
			id={`panneau-${kind}`}
			role="tabpanel"
			aria-labelledby={labelledBy}
			className="auth-form"
			noValidate
			onSubmit={onSubmit}
		>
			<TextField
				id={`${kind}-pseudo`}
				label="Pseudo"
				type="text"
				autoComplete="username"
				value={credentials.username}
				onChange={(value) => update("username", value)}
				error={errors.username}
				hint={
					registering
						? `${USERNAME_MIN_CHARS} à ${USERNAME_MAX_CHARS} caractères : lettres, chiffres, . _ -`
						: undefined
				}
			/>
			<TextField
				id={`${kind}-mdp`}
				label="Mot de passe"
				type="password"
				autoComplete={registering ? "new-password" : "current-password"}
				value={credentials.password}
				onChange={(value) => update("password", value)}
				error={errors.password}
				hint={
					registering ? `${PASSWORD_MIN_CHARS} caractères minimum` : undefined
				}
			/>
			{formError ? (
				<p role="alert" className="form-error">
					{formError}
				</p>
			) : null}
			<button
				type="submit"
				className="button-primary auth-submit"
				disabled={submitting}
				aria-busy={submitting}
			>
				{submitLabel}
			</button>
		</form>
	);
}
