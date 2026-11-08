import {
	createRootRoute,
	createRoute,
	createRouter,
	Outlet,
	type RouterHistory,
	redirect,
} from "@tanstack/react-router";
import { fetchCurrentUser } from "@/lib/api";
import { AuthPage } from "@/pages/AuthPage";
import { ErrorPage } from "@/pages/ErrorPage";
import { HomePage } from "@/pages/HomePage";
import { NotFoundPage } from "@/pages/NotFoundPage";

// Every navigation asks the API who is logged in: the session cookie is
// HttpOnly, so the server is the only one that knows.
const rootRoute = createRootRoute({
	beforeLoad: async () => ({ user: await fetchCurrentUser() }),
	component: Outlet,
	notFoundComponent: NotFoundPage,
	errorComponent: ErrorPage,
});

const homeRoute = createRoute({
	getParentRoute: () => rootRoute,
	path: "/",
	beforeLoad: ({ context }) => {
		if (!context.user) {
			throw redirect({ to: "/connexion", replace: true });
		}
		return { user: context.user };
	},
	component: HomePage,
});

const authRoute = createRoute({
	getParentRoute: () => rootRoute,
	path: "/connexion",
	beforeLoad: ({ context }) => {
		if (context.user) {
			throw redirect({ to: "/", replace: true });
		}
	},
	component: AuthPage,
});

const routeTree = rootRoute.addChildren([homeRoute, authRoute]);

export function createAppRouter(history?: RouterHistory) {
	return createRouter(history ? { routeTree, history } : { routeTree });
}

declare module "@tanstack/react-router" {
	interface Register {
		router: ReturnType<typeof createAppRouter>;
	}
}
