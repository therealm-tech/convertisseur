import { useNavigate } from "@tanstack/react-router";
import { type KeyboardEvent, useRef, useState } from "react";
import { CredentialsForm } from "@/components/CredentialsForm";
import { Logo } from "@/components/Logo";
import { type Credentials, login, register } from "@/lib/api";
import { validateLogin, validateRegistration } from "@/lib/credentials";

type Tab = "login" | "registration";

const tabs: { id: Tab; label: string }[] = [
	{ id: "login", label: "Connexion" },
	{ id: "registration", label: "Inscription" },
];

export function AuthPage() {
	const navigate = useNavigate();
	const [tab, setTab] = useState<Tab>("login");
	const tabRefs = useRef<Partial<Record<Tab, HTMLButtonElement | null>>>({});

	async function logIn(credentials: Credentials) {
		await login(credentials);
		await navigate({ to: "/" });
	}

	async function signUp(credentials: Credentials) {
		await register(credentials);
		await logIn(credentials);
	}

	function onTabKeyDown(event: KeyboardEvent<HTMLDivElement>) {
		if (event.key !== "ArrowLeft" && event.key !== "ArrowRight") {
			return;
		}
		const next = tab === "login" ? "registration" : "login";
		setTab(next);
		tabRefs.current[next]?.focus();
	}

	return (
		<main className="auth">
			<header className="auth-header">
				<Logo size={104} />
				<h1 className="landing-title">Convertisseur</h1>
				<p className="landing-tagline">
					L'arabe et les bases de l'Islam, un petit jeu à la fois.
				</p>
			</header>
			<section className="auth-card">
				<div
					role="tablist"
					aria-label="Compte"
					className="auth-tabs"
					onKeyDown={onTabKeyDown}
				>
					{tabs.map(({ id, label }) => (
						<button
							key={id}
							ref={(element) => {
								tabRefs.current[id] = element;
							}}
							id={`onglet-${id}`}
							type="button"
							role="tab"
							className="auth-tab"
							aria-selected={tab === id}
							aria-controls={`panneau-${id}`}
							tabIndex={tab === id ? 0 : -1}
							onClick={() => setTab(id)}
						>
							{label}
						</button>
					))}
				</div>
				{tab === "login" ? (
					<CredentialsForm
						key="login"
						kind="login"
						labelledBy="onglet-login"
						submitLabel="Se connecter"
						validate={validateLogin}
						submit={logIn}
					/>
				) : (
					<CredentialsForm
						key="registration"
						kind="registration"
						labelledBy="onglet-registration"
						submitLabel="Créer mon compte"
						validate={validateRegistration}
						submit={signUp}
					/>
				)}
			</section>
		</main>
	);
}
