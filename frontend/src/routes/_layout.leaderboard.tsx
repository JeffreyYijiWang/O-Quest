import { createFileRoute } from "@tanstack/react-router";
import { EllipsisVertical } from "lucide-react";
import { LeaderboardCard } from "@/components/leaderboard/card";
import { CardFromUser } from "@/components/leaderboard/card-from-user";
import { useApi } from "@/lib/app-context";
import { requireAuth } from "@/lib/auth";
import type { components } from "@/lib/schema.gen";

export const Route = createFileRoute("/_layout/leaderboard")({
	beforeLoad: async ({ context }) => {
		return await requireAuth(context);
	},
	component: Leaderboard,
});

type LeaderboardResponse = components["schemas"]["LeaderboardResponse"];

function Leaderboard() {
	const { user } = Route.useRouteContext();

	const { $api } = useApi();
	const { data, fetchNextPage, hasNextPage, isFetchingNextPage } =
		$api.useInfiniteQuery(
			"get",
			"/api/leaderboard",
			{
				params: {
					query: {
						limit: 20,
					},
				},
			},
			{
				getNextPageParam: (lastPage: LeaderboardResponse) =>
					lastPage.has_next ? lastPage.next_cursor : undefined,
				initialPageParam: undefined,
				pageParamName: "cursor",
			},
		);

	const allEntries = data?.pages.flatMap((page) => page.entries) ?? [];
	const position = user.leaderboard_position ?? Infinity;

	return (
		<div className="px-4 pt-6 max-w-xl mx-auto [view-transition-name:main-content]">
			<div className="space-y-3">
				{allEntries.map((entry) => (
					<LeaderboardCard
						key={entry.rank}
						entry={entry}
						name={user.andrew_id}
						totalChallenges={user.total_challenges.total}
						to="/profile"
					/>
				))}
			</div>

			{hasNextPage && (
				<div className="my-6 text-center">
					<button
						type="button"
						onClick={() => fetchNextPage()}
						disabled={isFetchingNextPage}
						className="inline-flex items-center px-4 py-2 border border-gray-300 rounded-md shadow-sm text-sm font-medium text-gray-700 bg-white hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 disabled:opacity-50 disabled:cursor-not-allowed transition-colors duration-150"
					>
						{isFetchingNextPage ? "Loading..." : "Load More"}
					</button>
				</div>
			)}

			{position > allEntries.length && (
				<div className="flex flex-col gap-6 mt-6">
					<EllipsisVertical className="mx-auto text-white" />
					<CardFromUser user={user} to="/profile" />
				</div>
			)}
		</div>
	);
}
