import {
	type KeyboardEvent,
	type ReactNode,
	useEffect,
	useId,
	useRef,
} from "react";

interface DialogProps {
	title: string;
	children: ReactNode;
	onClose: () => void;
}

const focusable =
	'button:not([disabled]), [href], input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])';

/**
 * A modal dialog: it takes the focus, keeps Tab inside, closes on Escape,
 * and gives the focus back to what had it.
 */
export function Dialog({ title, children, onClose }: DialogProps) {
	const titleId = useId();
	const panel = useRef<HTMLDivElement>(null);

	useEffect(() => {
		const opener =
			document.activeElement instanceof HTMLElement
				? document.activeElement
				: null;
		panel.current?.querySelector<HTMLElement>(focusable)?.focus();
		return () => opener?.focus();
	}, []);

	function onKeyDown(event: KeyboardEvent<HTMLDivElement>) {
		if (event.key === "Escape") {
			event.stopPropagation();
			onClose();
			return;
		}
		if (event.key !== "Tab" || !panel.current) {
			return;
		}
		const items = [...panel.current.querySelectorAll<HTMLElement>(focusable)];
		const first = items[0];
		const last = items[items.length - 1];
		if (event.shiftKey && document.activeElement === first) {
			event.preventDefault();
			last?.focus();
		} else if (!event.shiftKey && document.activeElement === last) {
			event.preventDefault();
			first?.focus();
		}
	}

	return (
		<div className="dialog-backdrop">
			<div
				ref={panel}
				role="dialog"
				aria-modal="true"
				aria-labelledby={titleId}
				className="dialog"
				onKeyDown={onKeyDown}
			>
				<h2 id={titleId} className="dialog-title">
					{title}
				</h2>
				{children}
			</div>
		</div>
	);
}
