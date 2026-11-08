import "@testing-library/jest-dom/vitest";
import { cleanup } from "@testing-library/react";
import { afterEach, beforeEach, vi } from "vitest";

beforeEach(() => {
	// The router restores the scroll position on navigation; jsdom has no
	// layout and reports every scrollTo() as "not implemented".
	vi.spyOn(window, "scrollTo").mockImplementation(() => undefined);
});

afterEach(() => {
	cleanup();
	vi.restoreAllMocks();
});
