import { useState } from "react";
import { Dialog } from "@/components/Dialog";
import type { ListedUser } from "@/lib/api";

interface DeleteUserDialogProps {
	target: ListedUser;
	onConfirm: () => Promise<void>;
	onFailure: (event: string, failure: unknown) => void;
	onClose: () => void;
}

export function DeleteUserDialog({
	target,
	onConfirm,
	onFailure,
	onClose,
}: DeleteUserDialogProps) {
	const [busy, setBusy] = useState(false);
	const [failed, setFailed] = useState(false);

	async function confirm() {
		setBusy(true);
		setFailed(false);
		try {
			await onConfirm();
		} catch (failure) {
			onFailure("admin.delete_failed", failure);
			setFailed(true);
			setBusy(false);
		}
	}

	return (
		<Dialog title={`Supprimer ${target.username} ?`} onClose={onClose}>
			<p className="dialog-text">
				Le compte sera supprimé définitivement, et ses sessions fermées.
			</p>
			{failed ? (
				<p role="alert" className="form-error">
					La suppression a échoué. Réessaie dans un moment.
				</p>
			) : null}
			<div className="dialog-actions">
				<button type="button" className="button-secondary" onClick={onClose}>
					Annuler
				</button>
				<button
					type="button"
					className="button-danger-solid"
					disabled={busy}
					aria-busy={busy}
					onClick={confirm}
				>
					Supprimer le compte
				</button>
			</div>
		</Dialog>
	);
}
