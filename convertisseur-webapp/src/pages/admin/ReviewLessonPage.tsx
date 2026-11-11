import { Link, useParams } from "@tanstack/react-router";
import { useCallback, useEffect, useState } from "react";
import { ApiError } from "@/lib/api";
import {
	type Card,
	deleteAudio,
	getItem,
	type Lesson,
	type Status,
	setReviewNote,
	setStatus,
	type Unit,
	uploadAudio,
} from "@/lib/content";
import { logger } from "@/lib/logger";
import {
	cardState,
	fetchLessonCards,
	type LessonSummary,
	lessonOrder,
	listAllLessons,
	validateCards,
} from "@/lib/review";
import { DESKTOP_QUERY, useMediaQuery } from "@/lib/useMediaQuery";
import { ReviewDesktop } from "@/pages/admin/ReviewDesktop";
import { ReviewPhone } from "@/pages/admin/ReviewPhone";
import { countCards } from "@/pages/admin/reviewParts";
import { useReviewTotals } from "@/pages/admin/reviewTotals";

/** Everything a lesson's review shows. */
export interface ReviewData {
	lesson: Lesson;
	unit: Unit;
	cards: Card[];
	/** The lessons of the unit, the current one counted from `cards`. */
	siblings: LessonSummary[];
	/** The lessons left to review before and after this one. */
	previousId: string | undefined;
	nextId: string | undefined;
}

type Upload = (
	file: Blob,
	filename: string,
	durationMs: number | undefined,
) => Promise<void>;

/** What a reviewer does to the lesson; each one resolves once it is saved. */
export interface ReviewActions {
	busy: boolean;
	validate: (card: Card) => Promise<boolean>;
	flag: (card: Card, note: string) => Promise<boolean>;
	sendBack: (card: Card) => Promise<boolean>;
	validateRest: () => Promise<boolean>;
	setStatusOf: (kind: "lesson" | "unit", status: Status) => void;
	uploadCardAudio: (card: Card) => Upload;
	removeCardAudio: (card: Card) => () => Promise<void>;
	uploadLessonAudio: Upload;
	removeLessonAudio: () => Promise<void>;
}

type Phase =
	| { phase: "loading" }
	| { phase: "missing" }
	| { phase: "failed" }
	| { phase: "ready"; data: ReviewData; queue: LessonSummary[] };

/** Recomputes what depends on the lesson, its unit and its cards. */
function derive(
	lesson: Lesson,
	unit: Unit,
	cards: Card[],
	siblings: LessonSummary[],
	queue: LessonSummary[],
): ReviewData {
	const counts = countCards(cards);
	const current = siblings.map((sibling) =>
		sibling.id === lesson.id
			? {
					...sibling,
					status: lesson.status,
					hasAudio: lesson.audio !== null,
					unit: { ...sibling.unit, status: unit.status },
					cardCounts: counts,
				}
			: { ...sibling, unit: { ...sibling.unit, status: unit.status } },
	);
	const here = current.find((sibling) => sibling.id === lesson.id);
	const others = queue.filter((other) => other.id !== lesson.id);
	const before = here
		? others.filter((other) => lessonOrder(other, here) < 0)
		: [];
	const after = here
		? others.filter((other) => lessonOrder(other, here) > 0)
		: [];
	return {
		lesson,
		unit,
		cards,
		siblings: current,
		previousId: before[before.length - 1]?.id,
		nextId: after[0]?.id,
	};
}

async function load(lessonId: string) {
	const lesson = await getItem("lesson", lessonId);
	const [unit, cards, siblings, queue] = await Promise.all([
		getItem("unit", lesson.unit.id),
		fetchLessonCards(lessonId),
		listAllLessons({ unitId: lesson.unit.id }),
		listAllLessons({ pendingReview: true }),
	]);
	return {
		lesson,
		unit,
		cards,
		siblings: siblings.lessons,
		queue: queue.lessons,
	};
}

/** One lesson to review: card by card on a phone, as a list on a desktop. */
export function ReviewLessonPage() {
	const { lessonId } = useParams({
		from: "/_app/admin/relecture/lecons/$lessonId",
	});
	const desktop = useMediaQuery(DESKTOP_QUERY);
	const { refresh } = useReviewTotals();
	const [state, setState] = useState<Phase>({ phase: "loading" });
	const [busy, setBusy] = useState(false);
	const [error, setError] = useState<string | undefined>();

	useEffect(() => {
		let current = true;
		setState({ phase: "loading" });
		setError(undefined);
		load(lessonId).then(
			({ lesson, unit, cards, siblings, queue }) => {
				if (current) {
					setState({
						phase: "ready",
						data: derive(lesson, unit, cards, siblings, queue),
						queue,
					});
				}
			},
			(failure: unknown) => {
				logger.warn("admin.review_lesson_failed", { error: failure });
				if (current) {
					setState(
						failure instanceof ApiError && failure.status === 404
							? { phase: "missing" }
							: { phase: "failed" },
					);
				}
			},
		);
		return () => {
			current = false;
		};
	}, [lessonId]);

	const update = useCallback(
		(change: (data: ReviewData) => Partial<Omit<ReviewData, "siblings">>) => {
			setState((current) => {
				if (current.phase !== "ready") {
					return current;
				}
				const next = { ...current.data, ...change(current.data) };
				return {
					...current,
					data: derive(
						next.lesson,
						next.unit,
						next.cards,
						current.data.siblings,
						current.queue,
					),
				};
			});
		},
		[],
	);

	const replaceCard = useCallback(
		(id: string, patch: Partial<Card>) =>
			update(({ cards }) => ({
				cards: cards.map((card) =>
					card.id === id ? { ...card, ...patch } : card,
				),
			})),
		[update],
	);

	if (state.phase === "loading") {
		return (
			<p role="status" className="admin-content admin-table-message">
				Chargement…
			</p>
		);
	}
	if (state.phase !== "ready") {
		return (
			<div className="admin-content">
				<p role="alert" className="form-error">
					{state.phase === "missing"
						? "Cette leçon n'existe plus."
						: "La leçon n'a pas pu être chargée. Réessaie dans un moment."}
				</p>
				<Link to="/admin/relecture" className="button-outline review-back-link">
					Toutes les leçons
				</Link>
			</div>
		);
	}

	async function act(event: string, run: () => Promise<void>) {
		setBusy(true);
		setError(undefined);
		try {
			await run();
			refresh();
			return true;
		} catch (failure) {
			logger.warn(event, { error: failure });
			setError("L'enregistrement a échoué. Réessaie dans un moment.");
			return false;
		} finally {
			setBusy(false);
		}
	}

	const { data } = state;

	const actions: ReviewActions = {
		busy,
		validate: (card) =>
			act("admin.review_validate_failed", async () => {
				const change = await setStatus("card", card.id, "validated");
				const reviewNote = card.reviewNote
					? await setReviewNote("card", card.id, "")
					: null;
				replaceCard(card.id, { ...change, reviewNote });
			}),
		flag: (card, note) =>
			act("admin.review_flag_failed", async () => {
				const reviewNote = await setReviewNote("card", card.id, note);
				const change =
					card.status === "validated"
						? await setStatus("card", card.id, "to_review")
						: {};
				replaceCard(card.id, { ...change, reviewNote });
			}),
		sendBack: (card) =>
			act("admin.review_send_back_failed", async () => {
				replaceCard(card.id, await setStatus("card", card.id, "to_review"));
			}),
		validateRest: () =>
			act("admin.review_bulk_failed", async () => {
				const ids = data.cards
					.filter((card) => cardState(card) === "to_review")
					.map((card) => card.id);
				await validateCards(data.lesson.id, ids);
				const cards = await fetchLessonCards(data.lesson.id);
				update(() => ({ cards }));
			}),
		setStatusOf: (kind, status) => {
			void act("admin.review_status_failed", async () => {
				const id = kind === "lesson" ? data.lesson.id : data.unit.id;
				const change = await setStatus(kind, id, status);
				update(({ lesson, unit }) =>
					kind === "lesson"
						? { lesson: { ...lesson, ...change } }
						: { unit: { ...unit, ...change } },
				);
			});
		},
		uploadCardAudio: (card) => async (file, filename, durationMs) => {
			await uploadAudio("card", card.id, file, filename, durationMs);
			replaceCard(card.id, await getItem("card", card.id));
			refresh();
		},
		removeCardAudio: (card) => async () => {
			await deleteAudio("card", card.id);
			replaceCard(card.id, await getItem("card", card.id));
			refresh();
		},
		uploadLessonAudio: async (file, filename, durationMs) => {
			await uploadAudio("lesson", data.lesson.id, file, filename, durationMs);
			const lesson = await getItem("lesson", data.lesson.id);
			update(() => ({ lesson }));
			refresh();
		},
		removeLessonAudio: async () => {
			await deleteAudio("lesson", data.lesson.id);
			const lesson = await getItem("lesson", data.lesson.id);
			update(() => ({ lesson }));
			refresh();
		},
	};

	const View = desktop ? ReviewDesktop : ReviewPhone;
	return (
		<View key={data.lesson.id} data={data} actions={actions} error={error} />
	);
}
