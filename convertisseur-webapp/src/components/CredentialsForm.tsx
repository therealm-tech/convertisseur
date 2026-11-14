import { type FormEvent, useState } from "react";
import { TextField } from "@/components/TextField";
import {
	type CredentialErrors,
	type CredentialsInput,
	describeFailure,
	EMAIL_HINT,
	PASSWORD_MIN_CHARS,
	USERNAME_MAX_CHARS,
	USERNAME_MIN_CHARS,
} from "@/lib/credentials";
import { logger } from "@/lib/logger";

interface CredentialsFormProps {
	kind: "login" | "registration";
	labelledBy: string;
	submitLabel: string;
	validate: (credentials: CredentialsInput) => CredentialErrors;
	submit: (credentials: CredentialsInput) => Promise<void>;
}

/** The form behind both tabs of the auth screen. */
export function CredentialsForm({
	kind,
	labelledBy,
	submitLabel,
	validate,
	submit,
}: CredentialsFormProps) {
	const [credentials, setCredentials] = useState<CredentialsInput>({
		login: "",
		username: "",
		email: "",
		password: "",
	});
	const [attempted, setAttempted] = useState(false);
	const [serverErrors, setServerErrors] = useState<CredentialErrors>({});
	const [formError, setFormError] = useState<string | undefined>();
	const [submitting, setSubmitting] = useState(false);

	const localErrors = attempted ? validate(credentials) : {};
	const errors = { ...serverErrors, ...localErrors };
	const registering = kind === "registration";

	function update(field: keyof CredentialsInput, value: string) {
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
			{registering ? (
				<>
					<TextField
						id={`${kind}-pseudo`}
						label="Pseudo"
						type="text"
						autoComplete="username"
						value={credentials.username}
						onChange={(value) => update("username", value)}
						error={errors.username}
						hint={`${USERNAME_MIN_CHARS} à ${USERNAME_MAX_CHARS} caractères : lettres, chiffres, . _ -`}
					/>
					<TextField
						id={`${kind}-email`}
						label="Adresse e-mail"
						type="email"
						autoComplete="email"
						value={credentials.email}
						onChange={(value) => update("email", value)}
						error={errors.email}
						hint={EMAIL_HINT}
					/>
				</>
			) : (
				<TextField
					id={`${kind}-identifiant`}
					label="Pseudo ou adresse e-mail"
					type="text"
					autoComplete="username"
					value={credentials.login}
					onChange={(value) => update("login", value)}
					error={errors.login}
				/>
			)}
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
