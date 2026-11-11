import { z } from "zod";

/**
 * The shapes of a letter within a word; its isolated shape is the card's
 * Arabic. `kind` says which shapes the letter has.
 */
export const LetterFormsSchema = z.discriminatedUnion("kind", [
	z.object({
		kind: z.literal("joining"),
		initial: z.string(),
		medial: z.string(),
		final: z.string(),
	}),
	z.object({ kind: z.literal("non_joining"), final: z.string() }),
	z.object({ kind: z.literal("word_final"), final: z.string() }),
]);
export type LetterForms = z.infer<typeof LetterFormsSchema>;
