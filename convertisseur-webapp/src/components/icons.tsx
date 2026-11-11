import type { ReactNode } from "react";

function Stroke({ children }: { children: ReactNode }) {
	return (
		<svg
			width="22"
			height="22"
			viewBox="0 0 24 24"
			fill="none"
			stroke="currentColor"
			strokeWidth="2.4"
			strokeLinecap="round"
			strokeLinejoin="round"
			aria-hidden="true"
		>
			{children}
		</svg>
	);
}

export function BackIcon() {
	return (
		<Stroke>
			<path d="M15 5l-7 7 7 7" />
		</Stroke>
	);
}

export function CloseIcon() {
	return (
		<Stroke>
			<path d="M6 6l12 12M18 6L6 18" />
		</Stroke>
	);
}
