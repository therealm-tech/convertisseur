import {
	type ChangeEvent,
	type ReactNode,
	useEffect,
	useRef,
	useState,
} from "react";
import { ApiError, problemTypes } from "@/lib/api";
import { type Audio, audioUrl } from "@/lib/content";
import { describeAudio, formatDuration } from "@/lib/contentLabels";
import { formatDay } from "@/lib/dates";
import { logger } from "@/lib/logger";
import { readDuration } from "@/lib/mediaDuration";
import {
	extensionOf,
	MicrophoneError,
	type Recording,
	recordingObstacle,
	startRecording,
} from "@/lib/recorder";

interface AudioBlockProps {
	audio: Audio | null;
	/**
	 * Stores the file in place of the current recording, with the length
	 * measured in the browser when there is one.
	 */
	onUpload: (
		file: Blob,
		filename: string,
		durationMs: number | undefined,
	) => Promise<void>;
	onRemove: () => Promise<void>;
	/** "Audio" by default. */
	label?: string;
	/** The microphone button's label, "Enregistrer" by default. */
	recordLabel?: string;
	/** Leaves out the formats hint and the upload date, for a list. */
	compact?: boolean;
}

type Mode =
	| { mode: "idle" }
	| { mode: "choosing" }
	| { mode: "recording"; seconds: number }
	| { mode: "sending" };

const UNEXPECTED = "L'envoi a échoué. Réessaie dans un moment.";

const obstacleMessages = {
	insecure:
		"Le micro n'est disponible qu'en HTTPS. Ouvre l'administration en HTTPS, ou téléverse un fichier.",
	unsupported:
		"Ce navigateur ne sait pas enregistrer le son. Téléverse plutôt un fichier.",
} as const;

const microphoneMessages = {
	denied:
		"L'accès au micro a été refusé. Autorise-le pour ce site dans les réglages du navigateur, puis réessaie.",
	missing: "Aucun micro n'a été trouvé sur cet appareil.",
	failed: "Le micro n'a pas pu être ouvert. Réessaie, ou téléverse un fichier.",
} as const;

function describeUploadFailure(failure: unknown): string {
	if (failure instanceof ApiError) {
		if (failure.type === problemTypes.tooLarge || failure.status === 413) {
			return "Le fichier dépasse 10 Mo.";
		}
		if (failure.type === problemTypes.unsupportedAudio) {
			return "Ce fichier n'est pas un audio reconnu : MP3, M4A, OGG, WebM ou WAV.";
		}
	}
	return UNEXPECTED;
}

/**
 * The recording of a card or a lesson: play it, replace it with a file or
 * with the device's microphone, or remove it.
 */
export function AudioBlock({
	audio,
	onUpload,
	onRemove,
	label = "Audio",
	recordLabel = "Enregistrer",
	compact = false,
}: AudioBlockProps) {
	const [state, setState] = useState<Mode>({ mode: "idle" });
	const [error, setError] = useState<string | undefined>();
	const [playing, setPlaying] = useState(false);
	const player = useRef<HTMLAudioElement>(null);
	const recording = useRef<Recording | undefined>(undefined);
	const recordingMode = state.mode === "recording";

	useEffect(() => {
		if (!recordingMode) {
			return;
		}
		const timer = setInterval(() => {
			setState((current) =>
				current.mode === "recording"
					? { mode: "recording", seconds: current.seconds + 1 }
					: current,
			);
		}, 1000);
		return () => clearInterval(timer);
	}, [recordingMode]);

	// Leaving the panel mid-recording releases the microphone.
	useEffect(() => () => recording.current?.cancel(), []);

	async function send(file: Blob, filename: string, fallbackMs?: number) {
		setState({ mode: "sending" });
		setError(undefined);
		try {
			const durationMs = (await readDuration(file)) ?? fallbackMs;
			await onUpload(file, filename, durationMs);
			setState({ mode: "idle" });
		} catch (failure) {
			logger.warn("audio.upload_failed", { error: failure });
			setError(describeUploadFailure(failure));
			setState({ mode: "choosing" });
		}
	}

	function pickFile(event: ChangeEvent<HTMLInputElement>) {
		const file = event.target.files?.[0];
		event.target.value = "";
		if (file) {
			void send(file, file.name);
		}
	}

	async function record() {
		setError(undefined);
		const obstacle = recordingObstacle();
		if (obstacle) {
			setError(obstacleMessages[obstacle]);
			return;
		}
		try {
			recording.current = await startRecording();
			setPlaying(false);
			player.current?.pause();
			setState({ mode: "recording", seconds: 0 });
		} catch (failure) {
			setError(
				failure instanceof MicrophoneError
					? microphoneMessages[failure.reason]
					: microphoneMessages.failed,
			);
		}
	}

	async function stop() {
		const current = recording.current;
		recording.current = undefined;
		if (!current) {
			return;
		}
		const { blob, elapsedMs } = await current.stop();
		await send(blob, `enregistrement.${extensionOf(blob.type)}`, elapsedMs);
	}

	async function remove() {
		setState({ mode: "sending" });
		setError(undefined);
		try {
			await onRemove();
		} catch (failure) {
			logger.warn("audio.remove_failed", { error: failure });
			setError("La suppression a échoué. Réessaie dans un moment.");
		}
		setState({ mode: "idle" });
	}

	function togglePlay() {
		const element = player.current;
		if (!element) {
			return;
		}
		if (!playing) {
			element.play().catch((failure: unknown) => {
				logger.warn("audio.play_failed", { error: failure });
				setPlaying(false);
			});
		} else {
			element.pause();
		}
	}

	const showFile = audio !== null && state.mode === "idle";
	const choosing =
		state.mode === "choosing" || (audio === null && state.mode === "idle");

	return (
		<div className={compact ? "field audio-block-compact" : "field"}>
			<span className="field-label">{label}</span>

			{showFile ? (
				<div className="audio-file">
					<button
						type="button"
						className="audio-play"
						aria-label={playing ? "Mettre en pause" : "Écouter"}
						onClick={togglePlay}
					>
						{playing ? <PauseIcon /> : <PlayIcon />}
					</button>
					<div className="audio-file-text">
						<span className="audio-file-name">{describeAudio(audio)}</span>
						{compact ? null : (
							<span className="audio-file-meta">
								Ajouté le {formatDay(audio.createdAt)}
							</span>
						)}
					</div>
					<button
						type="button"
						className="button-small"
						onClick={() => {
							player.current?.pause();
							setError(undefined);
							setState({ mode: "choosing" });
						}}
					>
						Remplacer
					</button>
					<button
						type="button"
						className="audio-remove"
						aria-label="Supprimer l'audio"
						onClick={() => void remove()}
					>
						<TrashIcon />
					</button>
					{/* biome-ignore lint/a11y/useMediaCaption: a pronunciation sample: the card's text is its transcript. */}
					<audio
						ref={player}
						src={audioUrl(audio.id)}
						preload="none"
						onPlay={() => setPlaying(true)}
						onPause={() => setPlaying(false)}
						onEnded={() => setPlaying(false)}
					/>
				</div>
			) : null}

			{state.mode === "recording" ? (
				<div className="audio-recording">
					<span className="audio-recording-dot" aria-hidden="true" />
					<span role="status" className="audio-recording-status">
						Enregistrement en cours · {formatDuration(state.seconds * 1000)}
					</span>
					<button
						type="button"
						className="audio-stop"
						onClick={() => void stop()}
					>
						<StopIcon />
						Arrêter
					</button>
				</div>
			) : null}

			{state.mode === "sending" ? (
				<p role="status" className="field-hint">
					Envoi de l'audio…
				</p>
			) : null}

			{choosing ? (
				<div className="audio-choices">
					<div className="audio-choice-buttons">
						<label className="audio-upload">
							<UploadIcon />
							Téléverser un fichier
							<input
								type="file"
								accept="audio/*"
								className="visually-hidden"
								onChange={pickFile}
							/>
						</label>
						<button
							type="button"
							className="audio-record"
							onClick={() => void record()}
						>
							<MicIcon />
							{recordLabel}
						</button>
						{audio !== null ? (
							<button
								type="button"
								className="button-small audio-cancel"
								onClick={() => {
									setError(undefined);
									setState({ mode: "idle" });
								}}
							>
								Annuler
							</button>
						) : null}
					</div>
					{compact ? null : (
						<span className="field-hint">
							Un fichier MP3, M4A, OGG, WebM ou WAV de 10 Mo au plus, ou un
							enregistrement direct avec le micro de l'appareil, téléphone
							compris.
						</span>
					)}
				</div>
			) : null}

			{error ? (
				<p role="alert" className="form-error">
					{error}
				</p>
			) : null}
		</div>
	);
}

function Icon({ children, fill }: { children: ReactNode; fill?: true }) {
	return (
		<svg
			width="18"
			height="18"
			viewBox="0 0 24 24"
			fill={fill ? "currentColor" : "none"}
			stroke={fill ? "none" : "currentColor"}
			strokeWidth="2.2"
			strokeLinecap="round"
			strokeLinejoin="round"
			aria-hidden="true"
		>
			{children}
		</svg>
	);
}

function PlayIcon() {
	return (
		<Icon>
			<path d="M8 5.5v13l10.5-6.5z" />
		</Icon>
	);
}

function PauseIcon() {
	return (
		<Icon>
			<path d="M9 6v12M15 6v12" />
		</Icon>
	);
}

function StopIcon() {
	return (
		<Icon fill>
			<rect x="5" y="5" width="14" height="14" rx="2.5" />
		</Icon>
	);
}

function TrashIcon() {
	return (
		<Icon>
			<path d="M4.5 7h15M9.5 7V4.5h5V7M6.5 7l1 12.5h9l1-12.5" />
		</Icon>
	);
}

function UploadIcon() {
	return (
		<Icon>
			<path d="M12 15V4.5M7.5 9 12 4.5 16.5 9M5 15.5v3A1.5 1.5 0 0 0 6.5 20h11a1.5 1.5 0 0 0 1.5-1.5v-3" />
		</Icon>
	);
}

function MicIcon() {
	return (
		<Icon>
			<rect x="9" y="3.5" width="6" height="11" rx="3" />
			<path d="M5.5 11.5a6.5 6.5 0 0 0 13 0M12 18v2.5" />
		</Icon>
	);
}
