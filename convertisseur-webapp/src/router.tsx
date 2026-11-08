import {
	createRootRoute,
	createRoute,
	createRouter,
	Outlet,
	type RouterHistory,
} from "@tanstack/react-router";
import { LandingPage } from "@/pages/LandingPage";
import { NotFoundPage } from "@/pages/NotFoundPage";

const rootRoute = createRootRoute({
	component: Outlet,
	notFoundComponent: NotFoundPage,
});

const landingRoute = createRoute({
	getParentRoute: () => rootRoute,
	path: "/",
	component: LandingPage,
});

const routeTree = rootRoute.addChildren([landingRoute]);

export function createAppRouter(history?: RouterHistory) {
	return createRouter(history ? { routeTree, history } : { routeTree });
}

declare module "@tanstack/react-router" {
	interface Register {
		router: ReturnType<typeof createAppRouter>;
	}
}
