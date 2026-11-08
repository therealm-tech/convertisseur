import type { ReactNode } from "react";

/** Arabic text: always right-to-left, always set in Amiri. */
export function Arabic({ children }: { children: ReactNode }) {
	return (
		<span lang="ar" dir="rtl" className="arabic">
			{children}
		</span>
	);
}
