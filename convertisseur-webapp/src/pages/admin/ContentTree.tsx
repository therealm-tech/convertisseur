import { Link } from "@tanstack/react-router";
import { countOf, trackLabels } from "@/lib/contentLabels";
import type { ContentSearch } from "@/lib/contentRoutes";
import {
	buildTree,
	type Filters,
	type TreeData,
	type UnitNode,
} from "@/lib/contentTree";
import type { LessonSummary } from "@/lib/review";
import { ChevronIcon, ContentCrumbs } from "@/pages/admin/contentNav";
import { StatusPill } from "@/pages/admin/contentParts";
import { FlagIcon, Progress, totalOf } from "@/pages/admin/reviewParts";

interface ContentTreeProps {
	data: TreeData;
	filters: Filters;
	search: ContentSearch;
}

function emptyMessage(filters: Filters): string {
	if (filters.status === "to_review" && !filters.note && !filters.audio) {
		return "Rien à relire.";
	}
	if (filters.audio && !filters.status && !filters.note) {
		return "Toutes les cartes ont leur audio.";
	}
	return "Aucun contenu ne correspond.";
}

/** The tracks, their units and their lessons, as the filters keep them. */
export function ContentTree({ data, filters, search }: ContentTreeProps) {
	const tracks = buildTree(data, filters);
	const unit = filters.unit
		? data.units.find((candidate) => candidate.id === filters.unit)
		: undefined;
	return (
		<>
			{unit ? (
				<ContentCrumbs
					search={search}
					trail={[
						{ label: trackLabels[unit.track], search: { track: unit.track } },
					]}
				/>
			) : null}
			{tracks.length === 0 ? (
				<p className="admin-table-message">{emptyMessage(filters)}</p>
			) : (
				<div className="review-tracks">
					{tracks.map((node) => (
						<section
							key={node.track}
							className="review-track"
							aria-label={trackLabels[node.track]}
						>
							<h2 className="review-track-title">{trackLabels[node.track]}</h2>
							{node.units.map((unitNode) => (
								<UnitSection
									key={unitNode.unit.id}
									node={unitNode}
									filters={filters}
									search={search}
								/>
							))}
						</section>
					))}
				</div>
			)}
		</>
	);
}

interface UnitSectionProps {
	node: UnitNode;
	filters: Filters;
	search: ContentSearch;
}

function UnitSection({ node, filters, search }: UnitSectionProps) {
	const { unit, lessons, lessonCount } = node;
	const unfiltered =
		filters.status === undefined && !filters.note && !filters.audio;
	return (
		<section className="review-group" aria-label={unit.title}>
			<div className="review-group-head">
				<h3 className="review-group-title">
					<Link
						to="/admin/content"
						search={{ ...search, unit: unit.id, page: undefined }}
						className="content-tree-unit"
					>
						{unit.title}
					</Link>
				</h3>
				{unit.status === "to_review" ? <StatusPill status="to_review" /> : null}
				{unfiltered ? (
					<span className="review-lesson-meta">
						{lessonCount === 0
							? "Aucune leçon"
							: countOf(lessonCount, "leçon", "leçons")}
					</span>
				) : null}
				<Link
					to="/admin/content/$section/$itemId"
					params={{ section: "units", itemId: unit.id }}
					state={{ contentList: search }}
					className="content-tree-edit"
				>
					Modifier
				</Link>
			</div>
			{lessons.length > 0 ? (
				<ul className="review-lessons">
					{lessons.map((lesson) => (
						<li key={lesson.id} className="content-tree-lesson">
							<LessonRow lesson={lesson} filters={filters} search={search} />
						</li>
					))}
				</ul>
			) : null}
		</section>
	);
}

interface LessonRowProps {
	lesson: LessonSummary;
	filters: Filters;
	search: ContentSearch;
}

/**
 * A lesson with its cards, the count its filter is about, and the way to
 * review it while it is filtered to review.
 */
function LessonRow({ lesson, filters, search }: LessonRowProps) {
	const counts = lesson.cardCounts;
	const total = totalOf(counts);
	const silent = filters.audio && filters.status !== "to_review";
	const left = silent ? counts.withoutAudio : counts.toReview + counts.flagged;
	const [singular, plural] = silent
		? ["carte sans audio", "cartes sans audio"]
		: ["carte à relire", "cartes à relire"];
	const started = !silent && left > 0 && counts.validated > 0;
	const flagged = !silent && counts.flagged > 0;
	return (
		<>
			<Link
				to="/admin/content"
				search={{ ...search, lesson: lesson.id, page: undefined }}
				className="review-lesson"
			>
				<span className="review-lesson-text">
					<span className="review-lesson-title">{lesson.title}</span>
					<span className="review-lesson-meta">
						<span>{countOf(total, "carte", "cartes")}</span>
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
						{lesson.status === "to_review" ? (
							<span className="status-pill status-pill-to_review">
								Leçon à relire
							</span>
						) : null}
					</span>
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
				<ChevronIcon />
			</Link>
			{filters.status === "to_review" ? (
				<Link
					to="/admin/review/lessons/$lessonId"
					params={{ lessonId: lesson.id }}
					className="button-outline content-tree-review"
					aria-label={`Relire ${lesson.title}`}
				>
					Relire
				</Link>
			) : null}
		</>
	);
}
