export function Logo({ size }: { size: number }) {
	return (
		<img
			className="logo"
			src="/logo.svg"
			alt="Logo de Convertisseur"
			width={size}
			height={size}
		/>
	);
}
