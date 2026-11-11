/** Responses as the API shapes them, for the fake servers. */
export function json(status: number, body: unknown, headers = {}): Response {
	return new Response(JSON.stringify(body), {
		status,
		headers: { "Content-Type": "application/json", ...headers },
	});
}

export function problem(
	status: number,
	type: string,
	headers: Record<string, string> = {},
	errors?: { field: string; code: string }[],
): Response {
	return json(
		status,
		{ type: `urn:convertisseur:problem:${type}`, status, errors },
		{ ...headers, "Content-Type": "application/problem+json" },
	);
}
