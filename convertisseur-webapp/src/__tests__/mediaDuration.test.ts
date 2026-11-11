import { vi } from "vitest";
import { readDuration } from "@/lib/mediaDuration";

/** An `Audio` that decodes its source as `outcome` says. */
function stubAudio(outcome: "loaded" | "failed" | "silent", duration: number) {
	class FakeAudio extends EventTarget {
		preload = "";
		duration = duration;
		removeAttribute = vi.fn();

		set src(_value: string) {
			if (outcome === "silent") {
				return;
			}
			queueMicrotask(() =>
				this.dispatchEvent(
					new Event(outcome === "loaded" ? "loadedmetadata" : "error"),
				),
			);
		}
	}
	vi.stubGlobal("Audio", FakeAudio);
}

beforeEach(() => {
	vi.stubGlobal("URL", {
		createObjectURL: vi.fn(() => "blob:fake"),
		revokeObjectURL: vi.fn(),
	});
});

afterEach(() => {
	vi.unstubAllGlobals();
	vi.useRealTimers();
});

describe("readDuration", () => {
	it("gives the decoded length in milliseconds", async () => {
		stubAudio("loaded", 3.2);
		expect(await readDuration(new Blob(["x"]))).toBe(3_200);
		expect(URL.revokeObjectURL).toHaveBeenCalledWith("blob:fake");
	});

	it("gives null for the Infinity Chrome reports for a recorded WebM", async () => {
		stubAudio("loaded", Number.POSITIVE_INFINITY);
		expect(await readDuration(new Blob(["x"]))).toBeNull();
	});

	it("gives null for a file the browser cannot decode", async () => {
		stubAudio("failed", Number.NaN);
		expect(await readDuration(new Blob(["x"]))).toBeNull();
	});

	it("gives up when no metadata comes", async () => {
		vi.useFakeTimers();
		stubAudio("silent", 0);
		const read = readDuration(new Blob(["x"]));
		await vi.advanceTimersByTimeAsync(3_000);
		expect(await read).toBeNull();
	});
});
