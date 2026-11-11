import {
	createRootRoute,
	createRoute,
	createRouter,
	Outlet,
	type RouterHistory,
	redirect,
} from "@tanstack/react-router";
import { fetchCurrentSession } from "@/lib/api";
import { parseContentSearch, parseSection } from "@/lib/contentRoutes";
import { isStaff } from "@/lib/roles";
import {
	fetchLesson,
	fetchTrack,
	fetchTracks,
	parseTrackId,
} from "@/lib/tracks";
import { AuthPage } from "@/pages/AuthPage";
import { AdminLayout } from "@/pages/admin/AdminLayout";
import {
	ContentCreatePage,
	ContentEditPage,
} from "@/pages/admin/ContentEditPage";
import { ContentPage } from "@/pages/admin/ContentPage";
import { ReviewLessonPage } from "@/pages/admin/ReviewLessonPage";
import { ReviewQueuePage } from "@/pages/admin/ReviewQueuePage";
import { UsersPage } from "@/pages/admin/UsersPage";
import { ChangePasswordPage } from "@/pages/ChangePasswordPage";
import { ErrorPage } from "@/pages/ErrorPage";
import { HomePage } from "@/pages/HomePage";
import { LessonNotFound, LessonPage } from "@/pages/LessonPage";
import { NotFoundPage } from "@/pages/NotFoundPage";
import { TrackPage } from "@/pages/TrackPage";

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
			throw redirect({ to: "/login", replace: true });
		}
		if (context.session.mustChangePassword) {
			throw redirect({ to: "/password", replace: true });
		}
		return { user: context.session.user };
	},
	component: Outlet,
});

const homeRoute = createRoute({
	getParentRoute: () => appRoute,
	path: "/",
	loader: fetchTracks,
	component: HomePage,
});

const trackRoute = createRoute({
	getParentRoute: () => appRoute,
	path: "/tracks/$track",
	loader: ({ params }) => fetchTrack(parseTrackId(params.track)),
	component: TrackPage,
});

const lessonRoute = createRoute({
	getParentRoute: () => appRoute,
	path: "/tracks/$track/lessons/$lessonId",
	loader: ({ params }) =>
		fetchLesson(parseTrackId(params.track), params.lessonId),
	component: LessonPage,
	notFoundComponent: LessonNotFound,
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
		throw redirect({ to: "/admin/content", replace: true });
	},
});

const adminContentRoute = createRoute({
	getParentRoute: () => adminRoute,
	path: "/content",
	validateSearch: parseContentSearch,
	component: ContentPage,
});

const adminContentItemRoute = createRoute({
	getParentRoute: () => adminRoute,
	path: "/content/$section/$itemId",
	beforeLoad: ({ params }) => ({ kind: parseSection(params.section) }),
	component: ContentEditPage,
});

const adminContentNewRoute = createRoute({
	getParentRoute: () => adminRoute,
	path: "/content/$section/new",
	beforeLoad: ({ params }) => ({ kind: parseSection(params.section) }),
	component: ContentCreatePage,
});

const adminReviewRoute = createRoute({
	getParentRoute: () => adminRoute,
	path: "/review",
	component: ReviewQueuePage,
});

const adminReviewLessonRoute = createRoute({
	getParentRoute: () => adminRoute,
	path: "/review/lessons/$lessonId",
	component: ReviewLessonPage,
});

const adminUsersRoute = createRoute({
	getParentRoute: () => adminRoute,
	path: "/users",
	component: UsersPage,
});

const passwordRoute = createRoute({
	getParentRoute: () => rootRoute,
	path: "/password",
	beforeLoad: ({ context }) => {
		if (!context.session) {
			throw redirect({ to: "/login", replace: true });
		}
		return { session: context.session };
	},
	component: ChangePasswordPage,
});

const authRoute = createRoute({
	getParentRoute: () => rootRoute,
	path: "/login",
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
		trackRoute,
		lessonRoute,
		adminRoute.addChildren([
			adminIndexRoute,
			adminReviewRoute,
			adminReviewLessonRoute,
			adminContentRoute,
			adminContentItemRoute,
			adminContentNewRoute,
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
