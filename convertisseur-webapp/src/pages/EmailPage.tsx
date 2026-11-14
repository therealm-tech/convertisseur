import {
	Link,
	useNavigate,
	useRouteContext,
	useRouter,
} from "@tanstack/react-router";
import { type FormEvent, useState } from "react";
import { Logo } from "@/components/Logo";
import { TextField } from "@/components/TextField";
import { setEmail } from "@/lib/api";
import { describeFailure, EMAIL_HINT, validateEmail } from "@/lib/credentials";
import { logger } from "@/lib/logger";
import { LOGOUT_FAILED, useLogout } from "@/lib/useLogout";

/**
 * Sets the logged-in account's email. An account without one, created
 * before it was required, reaches no other page until it sets one.
 */
export function EmailPage() {
	const { session } = useRouteContext({ from: "/email" });
	const router = useRouter();
	const navigate = useNavigate();
	const forced = session.user.email === null;
	const [email, setEmailValue] = useState(session.user.email ?? "");
	const [attempted, setAttempted] = useState(false);
	const [serverError, setServerError] = useState<string | undefined>();
	const [formError, setFormError] = useState<string | undefined>();
	const [submitting, setSubmitting] = useState(false);
	const [changed, setChanged] = useState(false);
	const onLogout = useLogout(() => setFormError(LOGOUT_FAILED));

	const error = (attempted ? validateEmail(email) : undefined) ?? serverError;

	function update(value: string) {
		setEmailValue(value);
		setServerError(undefined);
		setFormError(undefined);
		setChanged(false);
	}

	async function onSubmit(event: FormEvent<HTMLFormElement>) {
		event.preventDefault();
		setAttempted(true);
		if (validateEmail(email)) {
			return;
		}
		setSubmitting(true);
		try {
			await setEmail(session.user.id, email.trim());
			logger.info("account.email_changed", { forced });
			await router.invalidate();
			if (forced) {
				await navigate({ to: "/", replace: true });
				return;
			}
			setAttempted(false);
			setChanged(true);
		} catch (error) {
			logger.info("account.email_change_failed", { error });
			const failure = describeFailure(error);
			setServerError(failure.fields.email);
			setFormError(failure.form);
		}
		setSubmitting(false);
	}

	return (
		<main className="auth">
			<header className="auth-header">
				<Logo size={72} />
				<h1 className="landing-title">
					{forced ? "Ajoute ton adresse e-mail" : "Changer mon adresse e-mail"}
				</h1>
				{forced ? (
					<p className="landing-tagline">
						Elle est désormais demandée à chaque compte.
					</p>
				) : null}
			</header>
			<section className="auth-card">
				<form className="auth-form" noValidate onSubmit={onSubmit}>
					<TextField
						id="email"
						label="Adresse e-mail"
						type="email"
						autoComplete="email"
						value={email}
						onChange={update}
						error={error}
						hint={EMAIL_HINT}
					/>
					{formError ? (
						<p role="alert" className="form-error">
							{formError}
						</p>
					) : null}
					{changed ? (
						<p role="status" className="form-success">
							Ton adresse e-mail a été changée.
						</p>
					) : null}
					<button
						type="submit"
						className="button-primary auth-submit"
						disabled={submitting}
						aria-busy={submitting}
					>
						Enregistrer
					</button>
				</form>
			</section>
			{forced ? (
				<button
					type="button"
					className="button-secondary auth-aside"
					onClick={onLogout}
				>
					Se déconnecter
				</button>
			) : (
				<Link to="/" className="auth-aside">
					Retour à l'accueil
				</Link>
			)}
		</main>
	);
}
