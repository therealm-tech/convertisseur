import type { InputHTMLAttributes } from "react";

interface TextFieldProps {
	id: string;
	label: string;
	type: "text" | "email" | "password";
	autoComplete: InputHTMLAttributes<HTMLInputElement>["autoComplete"];
	value: string;
	onChange: (value: string) => void;
	error?: string | undefined;
	hint?: string | undefined;
}

export function TextField({
	id,
	label,
	type,
	autoComplete,
	value,
	onChange,
	error,
	hint,
}: TextFieldProps) {
	const hintId = `${id}-aide`;
	const errorId = `${id}-erreur`;
	const describedBy = [error ? errorId : undefined, hint ? hintId : undefined]
		.filter(Boolean)
		.join(" ");
	return (
		<div className="field">
			<label htmlFor={id} className="field-label">
				{label}
			</label>
			<input
				id={id}
				className="field-input"
				type={type}
				autoComplete={autoComplete}
				autoCapitalize="none"
				autoCorrect="off"
				spellCheck={false}
				value={value}
				onChange={(event) => onChange(event.target.value)}
				aria-invalid={error ? true : undefined}
				aria-describedby={describedBy || undefined}
			/>
			{error ? (
				<span id={errorId} className="field-error">
					{error}
				</span>
			) : null}
			{hint ? (
				<span id={hintId} className="field-hint">
					{hint}
				</span>
			) : null}
		</div>
	);
}
