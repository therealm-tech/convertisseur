import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { LetterFormsRow } from "@/components/LetterFormsRow";
import type { LetterForms } from "@/lib/letterForms";
import { installFakeApi } from "./fakeApi";
import { renderAt } from "./helpers";

const ba: LetterForms = {
	kind: "joining",
	initial: "بـ",
	medial: "ـبـ",
	final: "ـب",
};
const dal: LetterForms = { kind: "non_joining", final: "ـد" };
const taMarbuta: LetterForms = { kind: "word_final", final: "ـة" };

/** Each shape's label and what its cell reads. */
function shapes() {
	const row = screen.getByRole("figure", { name: "Formes de la lettre" });
	return within(row)
		.getAllByRole("term")
		.map((term) => [
			term.textContent,
			term.nextElementSibling?.textContent ?? "",
		]);
}

describe("letter forms", () => {
	it("shows the four shapes of a joining letter, in Amiri", () => {
		render(<LetterFormsRow arabic="ب" forms={ba} />);

		expect(shapes()).toEqual([
			["Isolée", "ب"],
			["Début", "بـ"],
			["Milieu", "ـبـ"],
			["Fin", "ـب"],
		]);
		const medial = screen.getByText("ـبـ");
		expect(medial).toHaveAttribute("lang", "ar");
		expect(medial).toHaveAttribute("dir", "rtl");
		expect(screen.queryByText(/pas de forme/)).not.toBeInTheDocument();
	});

	it("says a letter that never joins the next one has no initial or medial shape", () => {
		render(<LetterFormsRow arabic="د" forms={dal} />);

		expect(shapes()).toEqual([
			["Isolée", "د"],
			["Début", "—Aucune"],
			["Milieu", "—Aucune"],
			["Fin", "ـد"],
		]);
		expect(
			screen.getByText(
				"Ne s'attache jamais à la lettre suivante : pas de forme de début ni de milieu.",
			),
		).toBeInTheDocument();
	});

	it("says a letter that only ends a word has no initial or medial shape", () => {
		render(<LetterFormsRow arabic="ة" forms={taMarbuta} />);

		expect(shapes()[1]).toEqual(["Début", "—Aucune"]);
		expect(
			screen.getByText(
				"Seulement en fin de mot : pas de forme de début ni de milieu.",
			),
		).toBeInTheDocument();
	});

	it("shows the shapes under the letter on the lesson screen", async () => {
		const api = installFakeApi();
		api.logIn("aicha");
		const validated = { status: "validated" as const };
		api.content.seed("unit", {
			id: "u-alpha",
			title: "L'alphabet",
			...validated,
		});
		api.content.seed("lesson", {
			id: "l-letters",
			parentId: "u-alpha",
			title: "Les lettres",
			...validated,
		});
		api.content.seed("card", {
			parentId: "l-letters",
			arabic: "ب",
			forms: ba,
			...validated,
		});
		api.content.seed("card", {
			parentId: "l-letters",
			arabic: "شُكْرًا",
			...validated,
		});

		await renderAt("/parcours/arabe/lecons/l-letters");

		await screen.findByRole("heading", { name: "Les lettres" });
		expect(shapes()[2]).toEqual(["Milieu", "ـبـ"]);
		await userEvent.click(screen.getByRole("button", { name: "Suivante" }));
		expect(
			screen.queryByRole("figure", { name: "Formes de la lettre" }),
		).not.toBeInTheDocument();
	});

	it("shows the shapes read-only in the admin area", async () => {
		const api = installFakeApi();
		api.content.seed("unit", { id: "u-alpha", title: "L'alphabet" });
		api.content.seed("lesson", {
			id: "l-letters",
			parentId: "u-alpha",
			title: "Les lettres",
		});
		api.content.seed("card", {
			parentId: "l-letters",
			arabic: "د",
			transliteration: "dāl",
			forms: dal,
			importKey: "arabe/alphabet/lettres-2/dal",
		});
		api.logIn("nour");
		const user = userEvent.setup();
		await renderAt("/admin/contenu");
		await screen.findByRole("table");

		await user.click(screen.getByRole("button", { name: "د · dāl" }));

		const detail = await screen.findByRole("region", { name: "Détail" });
		await within(detail).findByText("Formes");
		expect(shapes()).toEqual([
			["Isolée", "د"],
			["Début", "—Aucune"],
			["Milieu", "—Aucune"],
			["Fin", "ـد"],
		]);
		expect(within(detail).queryByDisplayValue("ـد")).not.toBeInTheDocument();
	});
});
