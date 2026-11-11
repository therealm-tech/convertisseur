import { Link } from "@tanstack/react-router";
import { useEffect, useState } from "react";
import type { Track } from "@/lib/content";
import { countOf, trackLabels } from "@/lib/contentLabels";
import { logger } from "@/lib/logger";
import {
	type LessonSummary,
	listAllLessons,
	type ReviewTotals,
} from "@/lib/review";
import { FlagIcon, Progress, totalOf } from "@/pages/admin/reviewParts";

type Filter = "pending" | "silent";

type Listing =
	| { state: "loading" }
	| { state: "failed" }
	| { state: "ready"; lessons: LessonSummary[] };

interface UnitGroup {
	unit: LessonSummary["unit"];
	lessons: LessonSummary[];
}

interface TrackGroup {
	track: Track;
	units: UnitGroup[];
}

/** Lessons come in the order of the tracks, so each group is a run. */
function groupByTrack(lessons: LessonSummary[]): TrackGroup[] {
	const tracks: TrackGroup[] = [];
	for (const lesson of lessons) {
		let track = tracks[tracks.length - 1];
		if (track?.track !== lesson.unit.track) {
			track = { track: lesson.unit.track, units: [] };
			tracks.push(track);
		}
		const unit = track.units[track.units.length - 1];
		if (unit?.unit.id === lesson.unit.id) {
			unit.lessons.push(lesson);
		} else {
			track.units.push({ unit: lesson.unit, lessons: [lesson] });
		}
	}
	return tracks;
}

const emptyMessages: Record<Filter, string> = {
	pending: "Rien à relire.",
	silent: "Toutes les cartes ont leur audio.",
};

const leftNouns: Record<Filter, [string, string]> = {
	pending: ["carte à relire", "cartes à relire"],
	silent: ["carte sans audio", "cartes sans audio"],
};

/** The lessons with something to review or to record, by track and unit. */
export function ReviewQueuePage() {
	const [filter, setFilter] = useState<Filter>("pending");
	const [listing, setListing] = useState<Listing>({ state: "loading" });
	const [totals, setTotals] = useState<ReviewTotals | undefined>();

	useEffect(() => {
		let current = true;
		setListing({ state: "loading" });
		listAllLessons(
			filter === "pending" ? { pendingReview: true } : { missingAudio: true },
		).then(
			(result) => {
				if (current) {
					setListing({ state: "ready", lessons: result.lessons });
					setTotals(result.totals);
				}
			},
			(failure: unknown) => {
				logger.warn("admin.review_queue_failed", { error: failure });
				if (current) {
					setListing({ state: "failed" });
				}
			},
		);
		return () => {
			current = false;
		};
	}, [filter]);

	return (
		<div className="admin-content review-queue">
			<header className="admin-header">
				<h1 className="admin-title">Relecture</h1>
			</header>

			<fieldset className="status-filter review-filter">
				<legend className="visually-hidden">Leçons</legend>
				<FilterButton
					label="À relire"
					count={totals?.itemsToReview}
					pressed={filter === "pending"}
					onPress={() => setFilter("pending")}
				/>
				<FilterButton
					label="Sans audio"
					count={totals?.cardsWithoutAudio}
					pressed={filter === "silent"}
					onPress={() => setFilter("silent")}
				/>
			</fieldset>

			{listing.state === "loading" ? (
				<p role="status" className="admin-table-message">
					Chargement…
				</p>
			) : null}
			{listing.state === "failed" ? (
				<p role="alert" className="form-error">
					La liste n'a pas pu être chargée. Réessaie dans un moment.
				</p>
			) : null}
			{listing.state === "ready" && listing.lessons.length === 0 ? (
				<div className="review-empty">
					<p>{emptyMessages[filter]}</p>
					<Link to="/admin/content" className="button-outline">
						Voir le contenu
					</Link>
				</div>
			) : null}
			{listing.state === "ready" ? (
				<div className="review-tracks">
					{groupByTrack(listing.lessons).map((group) => (
						<section
							key={group.track}
							className="review-track"
							aria-label={trackLabels[group.track]}
						>
							<h2 className="review-track-title">{trackLabels[group.track]}</h2>
							{group.units.map((unit) => (
								<UnitSection key={unit.unit.id} group={unit} filter={filter} />
							))}
						</section>
					))}
				</div>
			) : null}
		</div>
	);
}

interface FilterButtonProps {
	label: string;
	count: number | undefined;
	pressed: boolean;
	onPress: () => void;
}

function FilterButton({ label, count, pressed, onPress }: FilterButtonProps) {
	return (
		<button type="button" aria-pressed={pressed} onClick={onPress}>
			{label}
			{count === undefined ? null : (
				<>
					{" "}
					<span className="review-filter-count">{count}</span>
				</>
			)}
		</button>
	);
}

function UnitSection({ group, filter }: { group: UnitGroup; filter: Filter }) {
	const { unit, lessons } = group;
	return (
		<section className="review-group" aria-label={unit.title}>
			<div className="review-group-head">
				<h3 className="review-group-title">{unit.title}</h3>
				{filter === "pending" && unit.status === "to_review" ? (
					<span className="status-pill status-pill-to_review">À valider</span>
				) : null}
			</div>
			<ul className="review-lessons">
				{lessons.map((lesson) => (
					<li key={lesson.id}>
						<LessonRow lesson={lesson} filter={filter} />
					</li>
				))}
			</ul>
		</section>
	);
}

/** A lesson with the one count its filter is about, and what else waits. */
function LessonRow({
	lesson,
	filter,
}: {
	lesson: LessonSummary;
	filter: Filter;
}) {
	const counts = lesson.cardCounts;
	const total = totalOf(counts);
	const left =
		filter === "pending"
			? counts.toReview + counts.flagged
			: counts.withoutAudio;
	const started = filter === "pending" && left > 0 && counts.validated > 0;
	const flagged = filter === "pending" && counts.flagged > 0;
	const [singular, plural] = leftNouns[filter];
	return (
		<Link
			to="/admin/review/lessons/$lessonId"
			params={{ lessonId: lesson.id }}
			className="review-lesson"
		>
			<span className="review-lesson-text">
				<span className="review-lesson-title">{lesson.title}</span>
				{started || flagged ? (
					<span className="review-lesson-meta">
						{started ? (
							<>
								<Progress done={counts.validated} total={total} />
								<span>{`${counts.validated}/${total} validées`}</span>
							</>
						) : null}
						{flagged ? (
							<span className="review-lesson-flagged">
								<FlagIcon />
								{countOf(counts.flagged, "signalée", "signalées")}
							</span>
						) : null}
					</span>
				) : null}
			</span>
			{left > 0 ? (
				<>
					<span className="review-count" aria-hidden="true">
						{left}
					</span>
					<span className="visually-hidden">
						{countOf(left, singular, plural)}
					</span>
				</>
			) : null}
			{left === 0 && filter === "pending" && lesson.status === "to_review" ? (
				<span className="status-pill status-pill-to_review">
					Leçon à valider
				</span>
			) : null}
			<ChevronIcon />
		</Link>
	);
}

function ChevronIcon() {
	return (
		<svg
			width="20"
			height="20"
			viewBox="0 0 24 24"
			fill="none"
			stroke="currentColor"
			strokeWidth="2.2"
			strokeLinecap="round"
			strokeLinejoin="round"
			aria-hidden="true"
		>
			<path d="m9 5 7 7-7 7" />
		</svg>
	);
}
