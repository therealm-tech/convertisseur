import type { User } from "@/lib/api";
import type { Audio, Kind, Status, Track, Variety } from "@/lib/content";
import type { LetterForms } from "@/lib/letterForms";
import { gradeQuiz, hasQuiz } from "@/lib/quiz";
import type { StudyCard } from "@/lib/tracks";
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
	forms: LetterForms | null;
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

const lessonCardsPath =
	/^\/api\/v1\/lessons\/([^/]+)\/(cards|card-validations)$/;

const trackPath = /^\/api\/v1\/tracks(?:\/([^/]+))?(?:\/lessons\/([^/]+))?$/;

const trackIds: Track[] = ["arabe", "islam"];

const cardViewPath = /^\/api\/v1\/card-views\/([^/]+)$/;

interface ExerciseResult {
	best: { firstTry: number; total: number; at: string };
	passedAt: string | null;
	lastAttemptAt: string;
}

interface LessonState {
	hasExercise: boolean;
	locked: boolean;
	seenCardCount: number;
	lastSeenAt: string | null;
	exercise: ExerciseResult | null;
}

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
	/** Per account, the cards it has seen and when it last saw each. */
	private views = new Map<string, Map<string, string>>();
	/** Per account, its exercise result in each lesson. */
	private results = new Map<string, Map<string, ExerciseResult>>();
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
			forms: null,
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
					forms: row.forms,
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

	/** Records that the account saw the cards, as a test's starting point. */
	see(userId: string, cardIds: readonly string[]) {
		const seen = this.viewsOf(userId);
		for (const cardId of cardIds) {
			seen.set(cardId, this.tick());
		}
	}

	/** Records an exercise attempt of the account, as a test's starting point. */
	attempt(userId: string, lessonId: string, firstTry = 12, total = 12) {
		const results = this.resultsOf(userId);
		const previous = results.get(lessonId);
		const at = this.tick();
		const better =
			!previous ||
			firstTry * previous.best.total > previous.best.firstTry * total;
		const result: ExerciseResult = {
			best: better || !previous ? { firstTry, total, at } : previous.best,
			passedAt:
				previous?.passedAt ?? (gradeQuiz(total, firstTry).passed ? at : null),
			lastAttemptAt: at,
		};
		results.set(lessonId, result);
		return result;
	}

	private viewsOf(userId: string): Map<string, string> {
		const views = this.views.get(userId) ?? new Map<string, string>();
		this.views.set(userId, views);
		return views;
	}

	private resultsOf(userId: string): Map<string, ExerciseResult> {
		const results =
			this.results.get(userId) ?? new Map<string, ExerciseResult>();
		this.results.set(userId, results);
		return results;
	}

	private studyCard(card: Row, seen: ReadonlyMap<string, string>) {
		return {
			id: card.id,
			arabic: card.arabic,
			transliteration: card.transliteration,
			meaningFr: card.meaningFr,
			variety: card.variety,
			forms: card.forms,
			audio: playback(card.audio),
			seen: seen.has(card.id),
		} satisfies StudyCard;
	}

	/** The visible lessons of the track, in order, across its units. */
	private path(track: Track): Row[] {
		return this.rows
			.filter(
				(row) =>
					row.kind === "unit" && row.track === track && this.visible(row),
			)
			.sort((a, b) => a.position - b.position)
			.flatMap((unit) => this.children(unit.id, "lesson"));
	}

	private states(track: Track, userId: string): Map<string, LessonState> {
		const seen = this.viewsOf(userId);
		const results = this.resultsOf(userId);
		const states = new Map<string, LessonState>();
		let wayOpen = true;
		for (const lesson of this.path(track)) {
			const cards = this.children(lesson.id, "card");
			const hasExercise = hasQuiz(
				cards.map((card) => this.studyCard(card, seen)),
			);
			const allCards = this.rows.filter(
				(row) => row.kind === "card" && row.parentId === lesson.id,
			);
			const seenTimes = allCards.flatMap((card) => {
				const at = seen.get(card.id);
				return at ? [at] : [];
			});
			const exercise = results.get(lesson.id) ?? null;
			const started = seenTimes.length > 0 || exercise !== null;
			const locked: boolean = !(wayOpen || started);
			wayOpen = !locked && (exercise?.passedAt != null || !hasExercise);
			states.set(lesson.id, {
				hasExercise,
				locked,
				seenCardCount: cards.filter((card) => seen.has(card.id)).length,
				lastSeenAt: seenTimes.sort().at(-1) ?? null,
				exercise,
			});
		}
		return states;
	}

	/** `PUT /api/v1/card-views/{cardId}` and `POST /api/v1/exercise-attempts`. */
	record(method: string, url: URL, body: unknown, user: User): Response {
		const view = url.pathname.match(cardViewPath);
		if (method === "PUT" && view) {
			const card = this.row(decodeURIComponent(view[1] ?? ""));
			const lesson = card?.parentId ? this.row(card.parentId) : undefined;
			if (card?.kind !== "card" || !lesson || !this.visible(card)) {
				return problem(404, "not-found");
			}
			if (this.states(card.track, user.id).get(lesson.id)?.locked) {
				return problem(403, "lesson-locked");
			}
			this.see(user.id, [card.id]);
			return new Response(null, { status: 204 });
		}
		if (method === "POST" && url.pathname === "/api/v1/exercise-attempts") {
			const { lessonId, firstTry, total } = body as {
				lessonId: string;
				firstTry: number;
				total: number;
			};
			const lesson = this.row(lessonId);
			if (lesson?.kind !== "lesson" || !this.visible(lesson)) {
				return problem(422, "validation", {}, [
					{ field: "lessonId", code: "lesson_not_found" },
				]);
			}
			if (this.states(lesson.track, user.id).get(lesson.id)?.locked) {
				return problem(403, "lesson-locked");
			}
			const exercise = this.attempt(user.id, lesson.id, firstTry, total);
			const path = this.path(lesson.track);
			const next = path[path.findIndex((row) => row.id === lesson.id) + 1];
			const open =
				next && !this.states(lesson.track, user.id).get(next.id)?.locked;
			return json(200, {
				exercise,
				nextLesson: open ? { id: next.id, title: next.title } : null,
			});
		}
		return problem(404, "not-found");
	}

	private outline(track: Track, userId: string) {
		const states = this.states(track, userId);
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
					lessons: this.children(unit.id, "lesson").map((lesson) => {
						const state = states.get(lesson.id);
						return {
							id: lesson.id,
							title: lesson.title,
							cardCount: this.children(lesson.id, "card").length,
							hasExercise: state?.hasExercise ?? false,
							locked: state?.locked ?? false,
							progress: {
								seenCardCount: state?.seenCardCount ?? 0,
								lastSeenAt: state?.lastSeenAt ?? null,
								exercise: state?.exercise ?? null,
							},
						};
					}),
				})),
		};
	}

	/** The learners' endpoints under `/api/v1/tracks`. */
	study(url: URL, user: User): Response {
		const match = url.pathname.match(trackPath);
		if (!match) {
			return problem(404, "not-found");
		}
		const [, track, lessonId] = match;
		if (track === undefined) {
			return json(200, {
				items: trackIds.map((id) => this.outline(id, user.id)),
			});
		}
		if (!trackIds.includes(track as Track)) {
			return problem(404, "not-found");
		}
		if (lessonId === undefined) {
			return json(200, this.outline(track as Track, user.id));
		}
		const lesson = this.row(decodeURIComponent(lessonId));
		const unit = lesson?.parentId ? this.row(lesson.parentId) : undefined;
		if (!lesson || !unit || lesson.kind !== "lesson" || unit.track !== track) {
			return problem(404, "not-found");
		}
		if (!this.visible(lesson)) {
			return problem(404, "not-found");
		}
		const state = this.states(track as Track, user.id).get(lesson.id);
		if (state?.locked) {
			return problem(403, "lesson-locked");
		}
		const seen = this.viewsOf(user.id);
		return json(200, {
			id: lesson.id,
			track,
			unit: { id: unit.id, title: unit.title },
			title: lesson.title,
			audio: playback(lesson.audio),
			cards: this.children(lesson.id, "card").map((card) =>
				this.studyCard(card, seen),
			),
			exercise: state?.exercise ?? null,
		});
	}

	/** A lesson as the review queue lists it. */
	private summary(lesson: Row) {
		const unit = this.row(lesson.parentId ?? "") as Row;
		const cards = this.rows.filter(
			(row) => row.kind === "card" && row.parentId === lesson.id,
		);
		const flagged = (row: Row) =>
			row.status === "to_review" && row.reviewNote !== null;
		return {
			id: lesson.id,
			position: lesson.position,
			title: lesson.title,
			status: lesson.status,
			reviewNote: lesson.reviewNote,
			hasAudio: lesson.audio !== null,
			unit: {
				id: unit.id,
				track: unit.track,
				position: unit.position,
				title: unit.title,
				status: unit.status,
				reviewNote: unit.reviewNote,
			},
			cardCounts: {
				validated: cards.filter((card) => card.status === "validated").length,
				flagged: cards.filter(flagged).length,
				toReview: cards.filter(
					(card) => card.status === "to_review" && !flagged(card),
				).length,
				withoutAudio: cards.filter((card) => card.audio === null).length,
			},
		};
	}

	/** `GET /api/v1/lessons`: the review queue. */
	private lessons(url: URL): Response {
		const params = url.searchParams;
		const page = Number(params.get("page") ?? 1);
		const perPage = Number(params.get("perPage") ?? 50);
		const cardsOf = (lesson: Row) =>
			this.rows.filter(
				(row) => row.kind === "card" && row.parentId === lesson.id,
			);
		const unitOf = (lesson: Row) => this.row(lesson.parentId ?? "") as Row;
		const matching = this.rows
			.filter((row) => row.kind === "lesson")
			.filter(
				(lesson) =>
					!params.get("unitId") || lesson.parentId === params.get("unitId"),
			)
			.filter(
				(lesson) =>
					params.get("pendingReview") !== "true" ||
					lesson.status === "to_review" ||
					unitOf(lesson).status === "to_review" ||
					cardsOf(lesson).some((card) => card.status === "to_review"),
			)
			.filter(
				(lesson) =>
					params.get("missingAudio") !== "true" ||
					cardsOf(lesson).some((card) => card.audio === null),
			)
			.sort(
				(a, b) =>
					unitOf(a).track.localeCompare(unitOf(b).track) ||
					unitOf(a).position - unitOf(b).position ||
					a.position - b.position,
			);
		return json(200, {
			items: matching
				.slice((page - 1) * perPage, page * perPage)
				.map((lesson) => this.summary(lesson)),
			page,
			perPage,
			total: matching.length,
			itemsToReview: this.rows.filter((row) => row.status === "to_review")
				.length,
			cardsWithoutAudio: this.rows.filter(
				(row) => row.kind === "card" && row.audio === null,
			).length,
		});
	}

	/** A lesson's cards, and their validation in bulk. */
	private lessonCards(
		method: string,
		lesson: Row | undefined,
		sub: string,
		body: unknown,
		actor: User,
	): Response {
		if (lesson?.kind !== "lesson") {
			return problem(404, "not-found");
		}
		const cards = this.rows
			.filter((row) => row.kind === "card" && row.parentId === lesson.id)
			.sort((a, b) => a.position - b.position);
		if (method === "GET" && sub === "cards") {
			return json(200, {
				items: cards.map((card) => this.body(card)),
				page: 1,
				perPage: 100,
				total: cards.length,
			});
		}
		if (method === "POST" && sub === "card-validations") {
			const { cardIds } = body as { cardIds: string[] };
			const named = cards.filter((card) => cardIds.includes(card.id));
			if (cardIds.length === 0 || new Set(cardIds).size !== named.length) {
				return problem(422, "validation", {}, [
					{ field: "cardIds", code: "card_not_in_lesson" },
				]);
			}
			for (const card of named) {
				if (card.status === "to_review" && card.reviewNote === null) {
					card.status = "validated";
					card.validatedBy = actor;
					card.validatedAt = this.tick();
				}
			}
			return json(200, {
				cards: named.map((card) => ({
					id: card.id,
					status: card.status,
					validatedAt: card.validatedAt,
					validatedBy: author(card.validatedBy),
					reviewNote: card.reviewNote,
				})),
			});
		}
		return problem(404, "not-found");
	}

	handle(method: string, url: URL, body: unknown, actor: User): Response {
		if (url.pathname === "/api/v1/content-items" && method === "GET") {
			return this.list(url);
		}
		if (url.pathname === "/api/v1/lessons" && method === "GET") {
			return this.lessons(url);
		}
		const lessonCards = url.pathname.match(lessonCardsPath);
		if (lessonCards) {
			const [, lessonId = "", sub = ""] = lessonCards;
			return this.lessonCards(
				method,
				this.row(decodeURIComponent(lessonId)),
				sub,
				body,
				actor,
			);
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
