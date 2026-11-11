import {
	Link,
	useNavigate,
	useRouteContext,
	useRouter,
} from "@tanstack/react-router";
import { type FormEvent, useState } from "react";
import { Logo } from "@/components/Logo";
import { TextField } from "@/components/TextField";
import { changePassword, logout } from "@/lib/api";
import { PASSWORD_MIN_CHARS } from "@/lib/credentials";
import { logger } from "@/lib/logger";
import {
	describePasswordChangeFailure,
	type PasswordChangeErrors,
	type PasswordChangeForm,
	validatePasswordChange,
} from "@/lib/passwordChange";

const emptyForm: PasswordChangeForm = {
	currentPassword: "",
	newPassword: "",
	confirmation: "",
};

/**
 * Changes the logged-in account's password. After a reset by an admin, it is
 * the only page the account can reach, and asks for the temporary password.
 */
export function ChangePasswordPage() {
	const { session } = useRouteContext({ from: "/password" });
	const router = useRouter();
	const navigate = useNavigate();
	const forced = session.mustChangePassword;
	const [form, setForm] = useState(emptyForm);
	const [attempted, setAttempted] = useState(false);
	const [serverErrors, setServerErrors] = useState<PasswordChangeErrors>({});
	const [formError, setFormError] = useState<string | undefined>();
	const [submitting, setSubmitting] = useState(false);
	const [changed, setChanged] = useState(false);

	const errors = {
		...serverErrors,
		...(attempted ? validatePasswordChange(form, forced) : {}),
	};

	function update(field: keyof PasswordChangeForm, value: string) {
		setForm((current) => ({ ...current, [field]: value }));
		setServerErrors((current) => ({ ...current, [field]: undefined }));
		setFormError(undefined);
		setChanged(false);
	}

	async function onSubmit(event: FormEvent<HTMLFormElement>) {
		event.preventDefault();
		setAttempted(true);
		if (Object.keys(validatePasswordChange(form, forced)).length > 0) {
			return;
		}
		setSubmitting(true);
		try {
			await changePassword(session.user.id, {
				currentPassword: form.currentPassword,
				newPassword: form.newPassword,
			});
			logger.info("account.password_changed", { forced });
			if (forced) {
				await router.invalidate();
				await navigate({ to: "/", replace: true });
				return;
			}
			setForm(emptyForm);
			setAttempted(false);
			setChanged(true);
		} catch (error) {
			logger.info("account.password_change_failed", { error });
			const failure = describePasswordChangeFailure(error);
			setServerErrors(failure.fields);
			setFormError(failure.form);
		}
		setSubmitting(false);
	}

	async function onLogout() {
		try {
			await logout();
			await navigate({ to: "/login" });
		} catch (error) {
			logger.warn("auth.logout_failed", { error });
			setFormError("La déconnexion a échoué. Réessaie dans un moment.");
		}
	}

	return (
		<main className="auth">
			<header className="auth-header">
				<Logo size={72} />
				<h1 className="landing-title">
					{forced
						? "Choisis un nouveau mot de passe"
						: "Changer mon mot de passe"}
				</h1>
				<p className="landing-tagline">
					{forced
						? "Ton mot de passe a été réinitialisé. Choisis-en un nouveau pour continuer."
						: "Tes autres sessions seront fermées."}
				</p>
			</header>
			<section className="auth-card">
				<form className="auth-form" noValidate onSubmit={onSubmit}>
					<TextField
						id="mdp-actuel"
						label={forced ? "Mot de passe temporaire" : "Mot de passe actuel"}
						type="password"
						autoComplete="current-password"
						value={form.currentPassword}
						onChange={(value) => update("currentPassword", value)}
						error={errors.currentPassword}
					/>
					<TextField
						id="mdp-nouveau"
						label="Nouveau mot de passe"
						type="password"
						autoComplete="new-password"
						value={form.newPassword}
						onChange={(value) => update("newPassword", value)}
						error={errors.newPassword}
						hint={`${PASSWORD_MIN_CHARS} caractères minimum`}
					/>
					<TextField
						id="mdp-confirmation"
						label="Confirme le nouveau mot de passe"
						type="password"
						autoComplete="new-password"
						value={form.confirmation}
						onChange={(value) => update("confirmation", value)}
						error={errors.confirmation}
					/>
					{formError ? (
						<p role="alert" className="form-error">
							{formError}
						</p>
					) : null}
					{changed ? (
						<p role="status" className="form-success">
							Ton mot de passe a été changé. Tes autres sessions ont été
							fermées.
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
