import { useSyncExternalStore } from "react";

/** Wide enough for the admin area's desktop layouts. */
export const DESKTOP_QUERY = "(min-width: 1024px)";

/**
 * Whether the media query matches, kept up to date. False where the browser
 * cannot tell, so that the phone layout is the default.
 */
export function useMediaQuery(query: string): boolean {
	return useSyncExternalStore(
		(onChange) => {
			if (typeof window.matchMedia !== "function") {
				return () => undefined;
			}
			const list = window.matchMedia(query);
			list.addEventListener("change", onChange);
			return () => list.removeEventListener("change", onChange);
		},
		() =>
			typeof window.matchMedia === "function" &&
			window.matchMedia(query).matches,
	);
}
