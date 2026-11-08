import { useEffect, useRef, useState } from "react";
import { Dialog } from "@/components/Dialog";
import { type ListedUser, resetPassword } from "@/lib/api";
import { logger } from "@/lib/logger";

interface ResetPasswordDialogProps {
	target: ListedUser;
	onFailure: (event: string, failure: unknown) => void;
	/** `done` is whether the password was reset. */
	onClose: (done: boolean) => void;
}

type Stage =
	| { step: "confirm"; busy: boolean; failed: boolean }
	| { step: "done"; temporaryPassword: string };

/**
 * Asks for confirmation, resets the password, then shows the temporary one:
 * the server answers it once, and this dialog is the only place it lives.
 */
export function ResetPasswordDialog({
	target,
	onFailure,
	onClose,
}: ResetPasswordDialogProps) {
	const [stage, setStage] = useState<Stage>({
		step: "confirm",
		busy: false,
		failed: false,
	});
	const [copied, setCopied] = useState<boolean | undefined>();
	const copyButton = useRef<HTMLButtonElement>(null);
	const done = stage.step === "done";

	useEffect(() => {
		if (done) {
			copyButton.current?.focus();
		}
	}, [done]);

	async function reset() {
		setStage({ step: "confirm", busy: true, failed: false });
		try {
			const temporaryPassword = await resetPassword(target.id);
			setStage({ step: "done", temporaryPassword });
		} catch (failure) {
			onFailure("admin.reset_failed", failure);
			setStage({ step: "confirm", busy: false, failed: true });
		}
	}

	async function copy(password: string) {
		try {
			await navigator.clipboard.writeText(password);
			setCopied(true);
		} catch (failure) {
			logger.info("admin.copy_failed", { error: failure });
			setCopied(false);
		}
	}

	if (stage.step === "done") {
		return (
			<Dialog
				title={`Nouveau mot de passe de ${target.username}`}
				onClose={() => onClose(true)}
			>
				<p className="dialog-text">
					Transmets ce mot de passe temporaire à {target.username}. Il devra en
					choisir un nouveau à sa prochaine connexion.
				</p>
				<p className="temporary-password">
					<span className="temporary-password-label">
						Mot de passe temporaire
					</span>
					<code>{stage.temporaryPassword}</code>
				</p>
				<p className="dialog-warning">
					Il ne sera plus jamais affiché : note-le ou copie-le avant de fermer.
				</p>
				{copied === undefined ? null : (
					<p role="status" className="dialog-text">
						{copied
							? "Copié dans le presse-papiers."
							: "La copie a échoué : recopie-le à la main."}
					</p>
				)}
				<div className="dialog-actions">
					<button
						ref={copyButton}
						type="button"
						className="button-secondary"
						onClick={() => copy(stage.temporaryPassword)}
					>
						Copier
					</button>
					<button
						type="button"
						className="button-primary dialog-primary"
						onClick={() => onClose(true)}
					>
						Fermer
					</button>
				</div>
			</Dialog>
		);
	}

	return (
		<Dialog
			title={`Réinitialiser le mot de passe de ${target.username} ?`}
			onClose={() => onClose(false)}
		>
			<p className="dialog-text">
				Un mot de passe temporaire remplacera l'actuel, et toutes ses sessions
				seront fermées.
			</p>
			{stage.failed ? (
				<p role="alert" className="form-error">
					La réinitialisation a échoué. Réessaie dans un moment.
				</p>
			) : null}
			<div className="dialog-actions">
				<button
					type="button"
					className="button-secondary"
					onClick={() => onClose(false)}
				>
					Annuler
				</button>
				<button
					type="button"
					className="button-primary dialog-primary"
					disabled={stage.busy}
					aria-busy={stage.busy}
					onClick={reset}
				>
					Réinitialiser
				</button>
			</div>
		</Dialog>
	);
}
