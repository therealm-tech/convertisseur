import { Arabic } from "@/components/Arabic";
import type { LetterForms } from "@/lib/letterForms";

const missingReasons: Record<
	Exclude<LetterForms["kind"], "joining">,
	string
> = {
	non_joining:
		"Ne s'attache jamais à la lettre suivante : pas de forme de début ni de milieu.",
	word_final: "Seulement en fin de mot : pas de forme de début ni de milieu.",
};

interface Shape {
	label: string;
	form: string | null;
}

/** The isolated, initial, medial and final shapes of a letter, in a row. */
export function LetterFormsRow({
	arabic,
	forms,
}: {
	arabic: string;
	forms: LetterForms;
}) {
	const joining = forms.kind === "joining";
	const shapes: Shape[] = [
		{ label: "Isolée", form: arabic },
		{ label: "Début", form: joining ? forms.initial : null },
		{ label: "Milieu", form: joining ? forms.medial : null },
		{ label: "Fin", form: forms.final },
	];
	return (
		<figure className="letter-forms" aria-label="Formes de la lettre">
			<dl className="letter-forms-row">
				{shapes.map(({ label, form }) => (
					<div
						key={label}
						className={form ? "letter-form" : "letter-form letter-form-none"}
					>
						<dt className="letter-form-label">{label}</dt>
						<dd className="letter-form-glyph">
							{form ? (
								<Arabic>{form}</Arabic>
							) : (
								<>
									<span aria-hidden="true">—</span>
									<span className="visually-hidden">Aucune</span>
								</>
							)}
						</dd>
					</div>
				))}
			</dl>
			{joining ? null : (
				<p className="letter-forms-note">{missingReasons[forms.kind]}</p>
			)}
		</figure>
	);
}
