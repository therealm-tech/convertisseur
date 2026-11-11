import type { TrackId } from "@/lib/tracks";

/** The emblem of a track: two letters for Arabic, a mihrab arch for Islam. */
export function TrackGlyph({ track }: { track: TrackId }) {
	return (
		<span className="track-glyph" aria-hidden="true">
			{track === "arabe" ? (
				<span lang="ar" dir="rtl" className="track-glyph-letters">
					أ ب
				</span>
			) : (
				<svg
					width="28"
					height="28"
					viewBox="0 0 24 24"
					fill="none"
					stroke="currentColor"
					strokeWidth="2"
					strokeLinecap="round"
					strokeLinejoin="round"
					aria-hidden="true"
				>
					<path d="M5 21V11a7 7 0 0 1 14 0v10" />
					<path d="M3 21h18" />
					<path d="M9.5 21v-5a2.5 2.5 0 0 1 5 0v5" />
				</svg>
			)}
		</span>
	);
}

/** The eight-pointed star a lesson stands on in a track's path. */
export function LessonStar({ number }: { number: number }) {
	return (
		<svg
			className="lesson-star"
			width="64"
			height="64"
			viewBox="0 0 100 100"
			aria-hidden="true"
		>
			<polygon points="50,2 64,16.1 83.9,16.1 83.9,36 98,50 83.9,64 83.9,83.9 64,83.9 50,98 36,83.9 16.1,83.9 16.1,64 2,50 16.1,36 16.1,16.1 36,16.1" />
			<text x="50" y="62" textAnchor="middle">
				{number}
			</text>
		</svg>
	);
}
