import {
	createRootRoute,
	createRoute,
	createRouter,
	Outlet,
	type RouterHistory,
	redirect,
} from "@tanstack/react-router";
import { fetchCurrentSession } from "@/lib/api";
import { isStaff } from "@/lib/roles";
import { AuthPage } from "@/pages/AuthPage";
import { AdminLayout } from "@/pages/admin/AdminLayout";
import { ContentPage } from "@/pages/admin/ContentPage";
import { UsersPage } from "@/pages/admin/UsersPage";
import { ChangePasswordPage } from "@/pages/ChangePasswordPage";
import { ErrorPage } from "@/pages/ErrorPage";
import { HomePage } from "@/pages/HomePage";
import { NotFoundPage } from "@/pages/NotFoundPage";

// Every navigation asks the API who is logged in: the session cookie is
// HttpOnly, so the server is the only one that knows.
const rootRoute = createRootRoute({
	beforeLoad: async () => ({ session: await fetchCurrentSession() }),
	component: Outlet,
	notFoundComponent: NotFoundPage,
	errorComponent: ErrorPage,
});

// Every page but the auth screen and the password change needs a session
// whose password needs no change.
const appRoute = createRoute({
	getParentRoute: () => rootRoute,
	id: "_app",
	beforeLoad: ({ context }) => {
		if (!context.session) {
			throw redirect({ to: "/connexion", replace: true });
		}
		if (context.session.mustChangePassword) {
			throw redirect({ to: "/mot-de-passe", replace: true });
		}
		return { user: context.session.user };
	},
	component: Outlet,
});

const homeRoute = createRoute({
	getParentRoute: () => appRoute,
	path: "/",
	component: HomePage,
});

const adminRoute = createRoute({
	getParentRoute: () => appRoute,
	path: "/admin",
	beforeLoad: ({ context }) => {
		if (!isStaff(context.user.role)) {
			throw redirect({ to: "/", replace: true });
		}
	},
	component: AdminLayout,
});

const adminIndexRoute = createRoute({
	getParentRoute: () => adminRoute,
	path: "/",
	beforeLoad: () => {
		throw redirect({ to: "/admin/contenu", replace: true });
	},
});

const adminContentRoute = createRoute({
	getParentRoute: () => adminRoute,
	path: "/contenu",
	component: ContentPage,
});

const adminUsersRoute = createRoute({
	getParentRoute: () => adminRoute,
	path: "/utilisateurs",
	component: UsersPage,
});

const passwordRoute = createRoute({
	getParentRoute: () => rootRoute,
	path: "/mot-de-passe",
	beforeLoad: ({ context }) => {
		if (!context.session) {
			throw redirect({ to: "/connexion", replace: true });
		}
		return { session: context.session };
	},
	component: ChangePasswordPage,
});

const authRoute = createRoute({
	getParentRoute: () => rootRoute,
	path: "/connexion",
	beforeLoad: ({ context }) => {
		if (context.session) {
			throw redirect({ to: "/", replace: true });
		}
	},
	component: AuthPage,
});

const routeTree = rootRoute.addChildren([
	appRoute.addChildren([
		homeRoute,
		adminRoute.addChildren([
			adminIndexRoute,
			adminContentRoute,
			adminUsersRoute,
		]),
	]),
	passwordRoute,
	authRoute,
]);

export function createAppRouter(history?: RouterHistory) {
	return createRouter(history ? { routeTree, history } : { routeTree });
}

declare module "@tanstack/react-router" {
	interface Register {
		router: ReturnType<typeof createAppRouter>;
	}
}
