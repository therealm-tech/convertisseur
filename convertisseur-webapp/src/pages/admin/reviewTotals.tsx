import { useRouterState } from "@tanstack/react-router";
import {
	createContext,
	type ReactNode,
	useCallback,
	useContext,
	useEffect,
	useState,
} from "react";
import { logger } from "@/lib/logger";
import { fetchReviewTotals, type ReviewTotals } from "@/lib/review";

interface ReviewTotalsState {
	totals: ReviewTotals | undefined;
	/** Asks the API again, after a change of status, note or audio. */
	refresh: () => void;
}

const ReviewTotalsContext = createContext<ReviewTotalsState>({
	totals: undefined,
	refresh: () => undefined,
});

/** Keeps the review counts of the admin navigation, page after page. */
export function ReviewTotalsProvider({ children }: { children: ReactNode }) {
	const pathname = useRouterState({
		select: (state) => state.location.pathname,
	});
	const [totals, setTotals] = useState<ReviewTotals | undefined>();
	const [reloads, setReloads] = useState(0);
	const refresh = useCallback(() => setReloads((n) => n + 1), []);

	// Each navigation and each refresh fetches the counts again.
	useEffect(() => {
		let current = true;
		fetchReviewTotals().then(
			(next) => {
				if (current) {
					setTotals(next);
				}
			},
			(failure: unknown) =>
				logger.warn("admin.review_totals_failed", { error: failure }),
		);
		return () => {
			current = false;
		};
	}, [pathname, reloads]);

	return (
		<ReviewTotalsContext.Provider value={{ totals, refresh }}>
			{children}
		</ReviewTotalsContext.Provider>
	);
}

export function useReviewTotals(): ReviewTotalsState {
	return useContext(ReviewTotalsContext);
}
