import { useEffect, useRef, useState } from "react";
import { logger } from "@/lib/logger";

interface PlayButtonProps {
	url: string;
	/** What the button plays, e.g. "Écouter la carte". */
	label: string;
}

/** Plays a recording, and pauses it on a second press or when unmounted. */
export function PlayButton({ url, label }: PlayButtonProps) {
	const player = useRef<HTMLAudioElement>(null);
	const [playing, setPlaying] = useState(false);
	const [failed, setFailed] = useState(false);

	useEffect(() => {
		const element = player.current;
		return () => element?.pause();
	}, []);

	function toggle() {
		const element = player.current;
		if (!element) {
			return;
		}
		if (playing) {
			element.pause();
			return;
		}
		setFailed(false);
		element.play().catch((error: unknown) => {
			logger.warn("audio.play_failed", { url, error });
			setPlaying(false);
			setFailed(true);
		});
	}

	return (
		<span className="play">
			<button
				type="button"
				className="play-button"
				aria-pressed={playing}
				onClick={toggle}
			>
				<svg
					width="22"
					height="22"
					viewBox="0 0 24 24"
					fill="currentColor"
					aria-hidden="true"
				>
					{playing ? (
						<path d="M7 5h3.5v14H7zM13.5 5H17v14h-3.5z" />
					) : (
						<path d="M8 4.8v14.4c0 .8.9 1.3 1.6.8l10.4-7.2c.6-.4.6-1.2 0-1.6L9.6 4c-.7-.5-1.6 0-1.6.8z" />
					)}
				</svg>
				{label}
			</button>
			{failed ? (
				<span role="alert" className="play-error">
					La lecture a échoué.
				</span>
			) : null}
			{/* biome-ignore lint/a11y/useMediaCaption: a pronunciation sample: the card's text is its transcript. */}
			<audio
				ref={player}
				src={url}
				preload="none"
				onPlay={() => setPlaying(true)}
				onPause={() => setPlaying(false)}
				onEnded={() => setPlaying(false)}
				onError={() => {
					logger.warn("audio.load_failed", { url });
					setPlaying(false);
					setFailed(true);
				}}
			/>
		</span>
	);
}
