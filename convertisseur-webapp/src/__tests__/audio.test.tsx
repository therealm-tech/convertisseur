import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { vi } from "vitest";
import { type FakeApi, installFakeApi } from "./fakeApi";
import { renderAt } from "./helpers";
import { problem } from "./responses";

/** A MediaRecorder that records "opus" as soon as it is stopped. */
class FakeRecorder extends EventTarget {
	static isTypeSupported(type: string): boolean {
		return type === "audio/webm;codecs=opus";
	}

	mimeType: string;
	state: "inactive" | "recording" = "inactive";

	constructor(_stream: MediaStream, options?: MediaRecorderOptions) {
		super();
		this.mimeType = options?.mimeType ?? "";
	}

	start() {
		this.state = "recording";
	}

	stop() {
		this.state = "inactive";
		this.dispatchEvent(
			Object.assign(new Event("dataavailable"), {
				data: new Blob(["opus"], { type: this.mimeType }),
			}),
		);
		this.dispatchEvent(new Event("stop"));
	}
}

const track = { stop: vi.fn() };

function giveMicrophone(getUserMedia: () => Promise<unknown>) {
	Object.defineProperty(navigator, "mediaDevices", {
		configurable: true,
		value: { getUserMedia: vi.fn(getUserMedia) },
	});
}

function setSecure(secure: boolean) {
	Object.defineProperty(window, "isSecureContext", {
		configurable: true,
		value: secure,
	});
}

beforeEach(() => {
	setSecure(true);
	vi.stubGlobal("MediaRecorder", FakeRecorder);
	giveMicrophone(async () => ({ getTracks: () => [track] }));
	vi.spyOn(HTMLMediaElement.prototype, "play").mockImplementation(function (
		this: HTMLMediaElement,
	) {
		this.dispatchEvent(new Event("play"));
		return Promise.resolve();
	});
	vi.spyOn(HTMLMediaElement.prototype, "pause").mockImplementation(function (
		this: HTMLMediaElement,
	) {
		this.dispatchEvent(new Event("pause"));
	});
});

afterEach(() => {
	vi.unstubAllGlobals();
	track.stop.mockClear();
});

function seed(api: FakeApi, withAudio: boolean) {
	const unit = api.content.seed("unit", { id: "u-1", title: "Salutations" });
	const lesson = api.content.seed("lesson", {
		id: "l-1",
		parentId: unit,
		title: "En darija",
	});
	api.content.seed("card", {
		id: "c-1",
		parentId: lesson,
		arabic: "واش راك؟",
		status: "validated",
		validatedAt: "2026-11-06T09:00:00Z",
		validatedBy: { id: "u-bilal", username: "bilal", role: "admin" },
		audio: withAudio
			? {
					id: "a-1",
					contentType: "audio/mpeg",
					sizeBytes: 48_000,
					durationMs: null,
					createdAt: "2026-11-05T10:00:00Z",
				}
			: null,
	});
}

async function openCard(withAudio: boolean) {
	const api = installFakeApi();
	seed(api, withAudio);
	api.logIn("nour");
	const user = userEvent.setup();
	await renderAt("/admin/contenu");
	await user.click(await screen.findByRole("button", { name: "واش راك؟" }));
	const panel = screen.getByRole("region", { name: "Détail" });
	await within(panel).findByRole("textbox", {
		name: "Recto (arabe, avec voyelles)",
	});
	return { api, user, panel };
}

function audioCalls(api: FakeApi) {
	return api.calls.filter((call) => call.path === "/api/v1/cards/c-1/audio");
}

/** The file the first audio call sent. */
function uploadedFile(api: FakeApi): File {
	const body = audioCalls(api)[0]?.body;
	const file = body instanceof FormData ? body.get("file") : null;
	if (!(file instanceof File)) {
		throw new Error("no file was uploaded");
	}
	return file;
}

describe("audio block", () => {
	it("offers a file or the microphone when there is no audio", async () => {
		const { panel } = await openCard(false);

		const input = within(panel).getByLabelText("Téléverser un fichier");
		expect(input).toHaveAttribute("type", "file");
		expect(input).toHaveAttribute("accept", "audio/*");
		expect(
			within(panel).getByRole("button", { name: "Enregistrer" }),
		).toBeVisible();
		expect(
			within(panel).queryByRole("button", { name: "Annuler" }),
		).not.toBeInTheDocument();
	});

	it("uploads a chosen file, which sends the card back to review", async () => {
		const { api, user, panel } = await openCard(false);

		await user.upload(
			within(panel).getByLabelText("Téléverser un fichier"),
			new File([new Uint8Array(2_500)], "shukran.mp3", { type: "audio/mpeg" }),
		);

		expect(await within(panel).findByText("MP3 · 3 ko")).toBeVisible();
		expect(audioCalls(api)[0]?.method).toBe("PUT");
		expect(uploadedFile(api).name).toBe("shukran.mp3");
		expect(api.content.row("c-1")?.status).toBe("to_review");
		expect(
			within(panel).getByText(
				"En attente de relecture. Invisible pour les apprenants.",
			),
		).toBeVisible();
	});

	it("records from the microphone with a live timer, then uploads", async () => {
		const { api, user, panel } = await openCard(false);

		await user.click(
			within(panel).getByRole("button", { name: "Enregistrer" }),
		);

		expect(
			within(panel).getByText("Enregistrement en cours · 0:00"),
		).toBeVisible();
		expect(
			await within(panel).findByText(
				"Enregistrement en cours · 0:01",
				{},
				{ timeout: 2_000 },
			),
		).toBeVisible();
		await user.click(within(panel).getByRole("button", { name: "Arrêter" }));

		expect(await within(panel).findByText(/^WebM · 1 ko/)).toBeVisible();
		const file = uploadedFile(api);
		expect(file.name).toBe("enregistrement.webm");
		expect(file.type).toBe("audio/webm;codecs=opus");
		expect(track.stop).toHaveBeenCalled();
	});

	it("plays and pauses the audio, replaces it or deletes it", async () => {
		const { api, user, panel } = await openCard(true);

		expect(within(panel).getByText("MP3 · 48 ko")).toBeVisible();
		await user.click(within(panel).getByRole("button", { name: "Écouter" }));
		await user.click(
			within(panel).getByRole("button", { name: "Mettre en pause" }),
		);
		expect(
			within(panel).getByRole("button", { name: "Écouter" }),
		).toBeVisible();
		expect(panel.querySelector("audio")).toHaveAttribute(
			"src",
			"/api/v1/audio/a-1",
		);

		await user.click(within(panel).getByRole("button", { name: "Remplacer" }));
		expect(
			within(panel).getByRole("button", { name: "Enregistrer" }),
		).toBeVisible();
		await user.click(within(panel).getByRole("button", { name: "Annuler" }));
		expect(within(panel).getByText("MP3 · 48 ko")).toBeVisible();

		await user.click(
			within(panel).getByRole("button", { name: "Supprimer l'audio" }),
		);

		expect(
			await within(panel).findByRole("button", { name: "Enregistrer" }),
		).toBeVisible();
		expect(audioCalls(api)).toEqual([
			{ method: "DELETE", path: "/api/v1/cards/c-1/audio", body: undefined },
		]);
		expect(api.content.row("c-1")?.audio).toBeNull();
	});

	it("explains a refused microphone", async () => {
		const { user, panel } = await openCard(false);
		giveMicrophone(async () => {
			throw new DOMException("Permission denied", "NotAllowedError");
		});

		await user.click(
			within(panel).getByRole("button", { name: "Enregistrer" }),
		);

		expect(await within(panel).findByRole("alert")).toHaveTextContent(
			"L'accès au micro a été refusé. Autorise-le pour ce site dans les réglages du navigateur, puis réessaie.",
		);
		expect(
			within(panel).getByRole("button", { name: "Enregistrer" }),
		).toBeVisible();
	});

	it("explains a browser that cannot record", async () => {
		const { user, panel } = await openCard(false);
		vi.stubGlobal("MediaRecorder", undefined);

		await user.click(
			within(panel).getByRole("button", { name: "Enregistrer" }),
		);

		expect(within(panel).getByRole("alert")).toHaveTextContent(
			"Ce navigateur ne sait pas enregistrer le son. Téléverse plutôt un fichier.",
		);
	});

	it("explains that the microphone needs HTTPS", async () => {
		const { user, panel } = await openCard(false);
		setSecure(false);

		await user.click(
			within(panel).getByRole("button", { name: "Enregistrer" }),
		);

		expect(within(panel).getByRole("alert")).toHaveTextContent(
			"Le micro n'est disponible qu'en HTTPS.",
		);
	});

	it("explains a file the server refuses", async () => {
		const { api, user, panel } = await openCard(false);
		api.failNext(
			"PUT",
			"/api/v1/cards/c-1/audio",
			problem(415, "unsupported-audio"),
		);

		await user.upload(
			within(panel).getByLabelText("Téléverser un fichier"),
			new File(["%PDF"], "notes.pdf", { type: "audio/mpeg" }),
		);

		expect(await within(panel).findByRole("alert")).toHaveTextContent(
			"Ce fichier n'est pas un audio reconnu : MP3, M4A, OGG, WebM ou WAV.",
		);
		await waitFor(() =>
			expect(
				within(panel).getByRole("button", { name: "Enregistrer" }),
			).toBeVisible(),
		);
	});
});
