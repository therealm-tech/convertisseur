import { createMemoryHistory, RouterProvider } from "@tanstack/react-router";
import { render } from "@testing-library/react";
import { createAppRouter } from "@/router";

export async function renderAt(path: string) {
	const router = createAppRouter(
		createMemoryHistory({ initialEntries: [path] }),
	);
	await router.load();
	render(<RouterProvider router={router} />);
	return router;
}
