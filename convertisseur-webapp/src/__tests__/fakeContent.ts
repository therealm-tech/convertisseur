import type { User } from "@/lib/api";
import type { Audio, Kind, Status, Track, Variety } from "@/lib/content";
import { json, problem } from "./responses";

interface Row {
	kind: Kind;
	id: string;
	parentId: string | null;
	track: Track;
	position: number;
	title: string;
	arabic: string;
	transliteration: string | null;
	meaningFr: string | null;
	variety: Variety;
	audio: Audio | null;
	status: Status;
	createdBy: User;
	updatedAt: string;
	validatedBy: User | null;
	validatedAt: string | null;
	importKey: string | null;
	reviewNote: string | null;
}

const collections: Record<string, Kind> = {
	units: "unit",
	lessons: "lesson",
	cards: "card",
};

const memberPath =
	/^\/api\/v1\/(units|lessons|cards)(?:\/([^/]+))?(\/status|\/audio|\/review-note)?$/;

const trackPath = /^\/api\/v1\/tracks(?:\/([^/]+))?(?:\/lessons\/([^/]+))?$/;

const trackIds: Track[] = ["arabe", "islam"];

const playback = (audio: Audio | null) =>
	audio
		? { url: `/api/v1/audio/${audio.id}`, durationMs: audio.durationMs }
		: null;

const author = (user: User | null) =>
	user ? { id: user.id, username: user.username } : null;

const nobody: User = { id: "u-seed", username: "seed", role: "admin" };

/** An in-memory stand-in for the content endpoints. */
export class FakeContent {
	rows: Row[] = [];
	private clock = Date.parse("2026-11-01T09:00:00Z");

	private tick(): string {
		this.clock += 60_000;
		return new Date(this.clock).toISOString();
	}

	/** Adds a row as if an admin had written it, and returns its id. */
	seed(
		kind: Kind,
		fields: Partial<Row> & { title?: string; arabic?: string },
	): string {
		const parent = this.rows.find((row) => row.id === fields.parentId);
		const id = fields.id ?? `${kind}-${this.rows.length + 1}`;
		this.rows.push({
			kind,
			id,
			parentId: null,
			track: parent?.track ?? "arabe",
			position:
				this.rows.filter(
					(row) =>
						row.kind === kind && row.parentId === (fields.parentId ?? null),
				).length + 1,
			title: "",
			arabic: "",
			transliteration: null,
			meaningFr: null,
			variety: "classique",
			audio: null,
			status: "to_review",
			createdBy: nobody,
			updatedAt: this.tick(),
			validatedBy: null,
			validatedAt: null,
			importKey: null,
			reviewNote: null,
			...fields,
		});
		return id;
	}

	row(id: string): Row | undefined {
		return this.rows.find((row) => row.id === id);
	}

	private body(row: Row) {
		const review = {
			status: row.status,
			createdAt: "2026-10-01T09:00:00Z",
			createdBy: author(row.createdBy),
			updatedAt: row.updatedAt,
			updatedBy: author(row.createdBy),
			validatedAt: row.validatedAt,
			validatedBy: author(row.validatedBy),
			importKey: row.importKey,
			reviewNote: row.reviewNote,
		};
		const parent = row.parentId ? this.row(row.parentId) : undefined;
		switch (row.kind) {
			case "unit":
				return {
					...review,
					id: row.id,
					track: row.track,
					position: row.position,
					title: row.title,
				};
			case "lesson":
				return {
					...review,
					id: row.id,
					unit: {
						id: row.parentId,
						position: parent?.position ?? 1,
						title: parent?.title ?? "",
					},
					track: row.track,
					position: row.position,
					title: row.title,
					audio: row.audio,
				};
			case "card":
				return {
					...review,
					id: row.id,
					lesson: { id: row.parentId, title: parent?.title ?? "" },
					track: row.track,
					position: row.position,
					arabic: row.arabic,
					transliteration: row.transliteration,
					meaningFr: row.meaningFr,
					variety: row.variety,
					audio: row.audio,
				};
		}
	}

	private list(url: URL): Response {
		const params = url.searchParams;
		const q = (params.get("q") ?? "").toLowerCase();
		const page = Number(params.get("page") ?? 1);
		const perPage = Number(params.get("perPage") ?? 50);
		const matching = this.rows
			.filter(
				(row) =>
					(!params.get("track") || row.track === params.get("track")) &&
					(!params.get("kind") || row.kind === params.get("kind")) &&
					`${row.title} ${row.arabic} ${row.transliteration ?? ""}`
						.toLowerCase()
						.includes(q),
			)
			.sort((a, b) => b.updatedAt.localeCompare(a.updatedAt));
		const status = params.get("status");
		const noted = params.get("hasReviewNote");
		const byNote = (row: Row) =>
			noted === null || (row.reviewNote !== null) === (noted === "true");
		const byStatus = (row: Row) => !status || row.status === status;
		const counts = {
			toReview: matching.filter(
				(row) => byNote(row) && row.status === "to_review",
			).length,
			validated: matching.filter(
				(row) => byNote(row) && row.status === "validated",
			).length,
		};
		const reviewNoteCount = matching.filter(
			(row) => byStatus(row) && row.reviewNote !== null,
		).length;
		const filtered = matching.filter((row) => byNote(row) && byStatus(row));
		const items = filtered
			.slice((page - 1) * perPage, page * perPage)
			.map((row) => ({
				kind: row.kind,
				id: row.id,
				track: row.track,
				title: row.kind === "card" ? row.arabic : row.title,
				transliteration: row.kind === "card" ? row.transliteration : null,
				parentTitle: row.parentId
					? (this.row(row.parentId)?.title ?? null)
					: null,
				status: row.status,
				updatedAt: row.updatedAt,
				importKey: row.importKey,
				reviewNote: row.reviewNote,
			}));
		return json(200, {
			items,
			page,
			perPage,
			total: filtered.length,
			statusCounts: counts,
			reviewNoteCount,
		});
	}

	private write(row: Row, body: Record<string, unknown>) {
		const content = () =>
			JSON.stringify([
				row.track,
				row.title,
				row.arabic,
				row.transliteration,
				row.meaningFr,
				row.variety,
			]);
		const placed = () => JSON.stringify([row.parentId, row.position]);
		const contentBefore = content();
		const placedBefore = placed();
		row.parentId = (body.unitId ?? body.lessonId ?? null) as string | null;
		row.track = (body.track ??
			(row.parentId ? this.row(row.parentId)?.track : undefined) ??
			row.track) as Track;
		row.title = (body.title as string | undefined)?.trim() ?? row.title;
		row.arabic = (body.arabic as string | undefined)?.trim() ?? row.arabic;
		row.transliteration = (body.transliteration as string) || null;
		row.meaningFr = (body.meaningFr as string) || null;
		row.variety = (body.variety as Variety | undefined) ?? row.variety;
		if (typeof body.position === "number") {
			row.position = body.position;
		}
		if (content() !== contentBefore) {
			row.status = "to_review";
			row.validatedBy = null;
			row.validatedAt = null;
		}
		if (content() !== contentBefore || placed() !== placedBefore) {
			row.updatedAt = this.tick();
		}
	}

	private audio(
		row: Row,
		method: string,
		body: unknown,
		actor: User,
	): Response {
		const before = row.audio;
		if (method === "DELETE") {
			row.audio = null;
		} else {
			const file = body instanceof FormData ? body.get("file") : null;
			if (!(file instanceof Blob)) {
				return problem(422, "validation", {}, [
					{ field: "file", code: "file_missing" },
				]);
			}
			const claimed = Number(
				body instanceof FormData && body.get("durationMs"),
			);
			row.audio = {
				id: `a-${this.rows.length}-${this.clock}`,
				contentType: file.type.split(";")[0] || "audio/mpeg",
				sizeBytes: file.size,
				durationMs: claimed > 0 ? claimed : null,
				createdAt: this.tick(),
			};
		}
		if (before !== row.audio) {
			row.status = "to_review";
			row.validatedBy = null;
			row.validatedAt = null;
			row.createdBy = row.createdBy ?? actor;
		}
		return row.audio && method === "PUT"
			? json(200, row.audio)
			: new Response(null, { status: 204 });
	}

	/** Whether learners see the row: it and every row above it are validated. */
	private visible(row: Row | undefined): boolean {
		if (row?.status !== "validated") {
			return false;
		}
		return row.parentId === null || this.visible(this.row(row.parentId));
	}

	private children(parentId: string, kind: Kind): Row[] {
		return this.rows
			.filter(
				(row) =>
					row.kind === kind && row.parentId === parentId && this.visible(row),
			)
			.sort((a, b) => a.position - b.position);
	}

	private outline(track: Track) {
		return {
			id: track,
			units: this.rows
				.filter(
					(row) =>
						row.kind === "unit" && row.track === track && this.visible(row),
				)
				.sort((a, b) => a.position - b.position)
				.map((unit) => ({
					id: unit.id,
					title: unit.title,
					lessons: this.children(unit.id, "lesson").map((lesson) => ({
						id: lesson.id,
						title: lesson.title,
						cardCount: this.children(lesson.id, "card").length,
					})),
				})),
		};
	}

	/** The learners' endpoints under `/api/v1/tracks`. */
	study(url: URL): Response {
		const match = url.pathname.match(trackPath);
		if (!match) {
			return problem(404, "not-found");
		}
		const [, track, lessonId] = match;
		if (track === undefined) {
			return json(200, { items: trackIds.map((id) => this.outline(id)) });
		}
		if (!trackIds.includes(track as Track)) {
			return problem(404, "not-found");
		}
		if (lessonId === undefined) {
			return json(200, this.outline(track as Track));
		}
		const lesson = this.row(decodeURIComponent(lessonId));
		const unit = lesson?.parentId ? this.row(lesson.parentId) : undefined;
		if (!lesson || !unit || lesson.kind !== "lesson" || unit.track !== track) {
			return problem(404, "not-found");
		}
		if (!this.visible(lesson)) {
			return problem(404, "not-found");
		}
		return json(200, {
			id: lesson.id,
			track,
			unit: { id: unit.id, title: unit.title },
			title: lesson.title,
			audio: playback(lesson.audio),
			cards: this.children(lesson.id, "card").map((card) => ({
				id: card.id,
				arabic: card.arabic,
				transliteration: card.transliteration,
				meaningFr: card.meaningFr,
				variety: card.variety,
				audio: playback(card.audio),
			})),
		});
	}

	handle(method: string, url: URL, body: unknown, actor: User): Response {
		if (url.pathname === "/api/v1/content-items" && method === "GET") {
			return this.list(url);
		}
		const match = url.pathname.match(memberPath);
		if (!match) {
			return problem(404, "not-found");
		}
		const [, collection = "", id, sub] = match;
		const kind = collections[collection] ?? "unit";
		const fields = (body ?? {}) as Record<string, unknown>;
		if (!id && method === "POST") {
			if (kind === "card" && !/[؀-ۿ]/.test(String(fields.arabic))) {
				return problem(422, "validation", {}, [
					{ field: "arabic", code: "arabic_script" },
				]);
			}
			const newId = this.seed(kind, { createdBy: actor });
			const row = this.row(newId) as Row;
			this.write(row, fields);
			return json(201, this.body(row));
		}
		const row = id ? this.row(decodeURIComponent(id)) : undefined;
		if (row && sub === "/audio") {
			return this.audio(row, method, body, actor);
		}
		if (row && sub === "/review-note" && method === "PUT") {
			const text = String(fields.text ?? "").trim();
			if (text.length > 2000) {
				return problem(422, "validation", {}, [
					{ field: "text", code: "review_note_length" },
				]);
			}
			row.reviewNote = text || null;
			return json(200, { text: row.reviewNote });
		}
		if (row?.importKey && !sub && (method === "PUT" || method === "DELETE")) {
			return problem(409, "imported-content");
		}
		if (method === "DELETE") {
			if (row) {
				this.rows.splice(this.rows.indexOf(row), 1);
			}
			return new Response(null, { status: 204 });
		}
		if (!row || row.kind !== kind) {
			return problem(404, "not-found");
		}
		if (sub === "/status" && method === "PUT") {
			const next = fields.status as Status;
			if (row.status !== next) {
				row.status = next;
				row.validatedBy = next === "validated" ? actor : null;
				row.validatedAt = next === "validated" ? this.tick() : null;
			}
			return json(200, {
				status: row.status,
				validatedAt: row.validatedAt,
				validatedBy: author(row.validatedBy),
			});
		}
		if (method === "PUT") {
			this.write(row, fields);
			return json(200, this.body(row));
		}
		return json(200, this.body(row));
	}
}
