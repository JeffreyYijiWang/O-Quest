import { createFileRoute } from "@tanstack/react-router";
import { Camera, Target, Trophy } from "lucide-react";
import StickyNoteTop from "@/assets/sticky-note-top.svg?react";
import { CardFromUser } from "@/components/leaderboard/card-from-user";
import { ModePill } from "@/components/mode-pill";
import CategoryProgressBar from "@/components/profile/category-progress-bar";
import { useApi } from "@/lib/app-context";
import { requireAuth } from "@/lib/auth";
import { categories } from "@/lib/data/categories";
import {
	type DormName,
	dormColors,
	dormGroupFromName,
	dormImagePaths,
} from "@/lib/data/dorms";

export const Route = createFileRoute("/_layout/profile")({
	beforeLoad: async ({ context }) => {
		return await requireAuth(context);
	},
	component: Profile,
});

function getDormMascotImage(dormName: string | null | undefined): string {
	if (!dormName) {
		return "/images/sample-profile-pic.svg";
	}

	return dormImagePaths[dormName] || "/images/sample-profile-pic.svg";
}

function getDormColor(dormName: string | null | undefined): string {
	if (!dormName) {
		return "bg-white";
	}

	const dormGroup = dormGroupFromName[dormName as DormName];
	if (!dormGroup || !dormColors[dormGroup]) {
		return "bg-white";
	}

	return dormColors[dormGroup].primary || "bg-white";
}

function Profile() {
	const { $api } = useApi();
	const { user } = Route.useRouteContext();

	const { data: prizeData } = $api.useQuery("get", "/api/rewards");

	const { data: journalData, isLoading: isLoadingJournal } = $api.useQuery(
		"get",
		"/api/journal",
	);

	const dormGroup = user.dorm
		? dormGroupFromName[user.dorm as DormName]
		: "No housing community";
	const dorm = user.dorm ?? "No dorm";

	const dormMascotImage = getDormMascotImage(user.dorm);
	const dormColor = getDormColor(user.dorm);

	return (
		<div className="p-4 [view-transition-name:main-content]">
			{/* Sticky note container */}
			<div className="rounded-2xl mb-6 mt-4 relative overflow-visible">
				{/* Dorm Header Section - Flush with top of card */}
				<div
					className={`${dormColor} w-full relative z-10 px-6 pt-6 pb-6 rounded-t-2xl`}
				>
					{/* Subtle pattern overlay */}
					<div className="absolute inset-0 bg-gradient-to-br from-white/10 to-transparent rounded-t-2xl"></div>

					{/* Mascot image centered in header */}
					<div className="flex justify-center relative z-10">
						<img
							src={dormMascotImage}
							alt={`${dorm} mascot`}
							className="w-24 h-32 rounded-xl object-contain"
						/>
					</div>
				</div>

				<div className="absolute top-0 left-0 right-0 z-20 w-full flex justify-center -mt-6">
					<StickyNoteTop className="h-auto max-w-md" />
				</div>

				<div className="flex flex-col gap-6 mb-6 p-6 bg-white rounded-b-2xl">
					<div className="w-full">
						<div className="mb-6 text-center">
							<h2 className="text-2xl font-bold text-gray-800 mb-1">
								{user.andrew_id}
							</h2>
							{/* <p className="text-lg text-gray-600 font-medium">
								{user.andrew_id}
							</p> */}
							<div className="flex justify-center">
								<ModePill isAdmin={user.groups.includes("O-Quest Admin")} />
							</div>
						</div>

						{/* Info Cards */}
						<div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
							{/* Housing Community Card */}
							<div className="bg-gradient-to-br from-purple-50 to-purple-100 rounded-2xl p-4 border border-purple-200">
								<div className="flex items-center mb-2">
									<div className="w-3 h-3 bg-purple-500 rounded-full mr-2"></div>
									<span className="font-semibold text-purple-800 text-sm uppercase tracking-wide">
										Housing Community
									</span>
								</div>
								<p className="text-purple-900 font-medium break-words">
									{dormGroup}
								</p>
							</div>

							{/* Carnegie Cup Contribution Card */}
							<div className="bg-gradient-to-br from-amber-50 to-amber-100 rounded-2xl p-4 border border-amber-200">
								<div className="flex items-center mb-2">
									<div className="w-3 h-3 bg-amber-500 rounded-full mr-2"></div>
									<span className="font-semibold text-amber-800 text-sm uppercase tracking-wide">
										Carnegie Cup Contribution
									</span>
								</div>
								<p className="text-amber-900 font-bold text-lg">
									{prizeData?.rewards.find(
										(reward) => reward.name === "Carnegie Cup Contribution",
									)?.transaction_info.total_purchased || 0}
								</p>
							</div>
						</div>

						{/* Additional Stats Row */}
						<div className="mt-4 grid grid-cols-1 sm:grid-cols-2 gap-4">
							{/* Leaderboard Position Preview */}
							<div className="bg-gradient-to-br from-orange-50 to-orange-100 rounded-2xl p-4 border border-orange-200">
								<div className="flex items-center mb-2">
									<div className="w-3 h-3 bg-orange-500 rounded-full mr-2"></div>
									<span className="font-semibold text-orange-800 text-sm uppercase tracking-wide">
										Rank
									</span>
								</div>
								<p className="text-orange-900 font-bold text-lg">
									#{user.leaderboard_position}
								</p>
							</div>

							<div className="bg-gradient-to-br from-green-50 to-green-100 rounded-2xl p-4 border border-green-200">
								<div className="flex items-center mb-2">
									<div className="w-3 h-3 bg-green-500 rounded-full mr-2"></div>
									<span className="font-semibold text-green-800 text-sm uppercase tracking-wide">
										Completion Rate
									</span>
								</div>
								<p className="text-green-900 font-bold text-lg">
									{Math.round(
										(user.challenges_completed.total /
											user.total_challenges.total) *
											100,
									)}
									%
								</p>
							</div>
						</div>
					</div>
				</div>

				{/* Progress Section */}
				<div className="mb-6">
					<div className="mb-4">
						<div className="mb-2 flex items-center gap-2">
							<Target className="size-5 text-default" />
							<span className="font-semibold text-lg">Quest Progress</span>
						</div>
						<div className="bg-white rounded-2xl p-4 border border-gray-200">
							<div className="flex justify-between items-center mb-2">
								<span className="text-sm font-medium text-gray-600">
									Overall Progress
								</span>
								<span className="text-sm font-bold text-gray-800">
									{user.challenges_completed.total}/
									{user.total_challenges.total}
								</span>
							</div>

							<div className="w-full h-3 bg-gray-300 rounded-full relative overflow-hidden">
								<div
									className="h-3 bg-gradient-to-r from-blue-500 to-blue-600 rounded-full transition-all duration-300 ease-out"
									style={{
										width: `${Math.min((user.challenges_completed.total / user.total_challenges.total) * 100, 100)}%`,
									}}
								/>
							</div>
						</div>
					</div>

					{/* Category Progress Bars */}
					<div className="mt-4">
						<CategoryProgressBar
							categories={categories.map((c) => ({
								name: c.label,
								percentage:
									((user.challenges_completed.by_category[c.label] || 0) /
										(user.total_challenges.by_category[c.label] ||
											user.total_challenges.total)) *
										100 || 0,
							}))}
						/>
					</div>
				</div>
			</div>

			{/* Leaderboard */}
			<div className="mb-2 flex items-center gap-2">
				<Trophy className="size-5 text-default" />
				<span className="font-semibold text-lg">Leaderboard</span>
			</div>
			<div className="mb-4">
				<CardFromUser to="/leaderboard" user={user} />
			</div>

			{/* Gallery */}
			<div className="mb-2 flex items-center gap-2">
				<Camera className="size-6 text-default" />
				<span className="font-semibold text-lg">Gallery</span>
			</div>

			{journalData?.entries?.length ? (
				isLoadingJournal ? (
					<div className="flex items-center justify-center h-64">
						<span className="text-gray-500">Loading gallery...</span>
					</div>
				) : (
					<div className="overflow-x-auto">
						<div className="flex gap-2 h-64">
							{journalData.entries.map((entry) => (
								<img
									key={entry.challenge_name + entry.note}
									src={entry.image_url || "/images/scotty-coin.svg"}
									alt={`${entry.challenge_name} - ${entry.note}`}
									className="w-80 h-64 object-cover rounded-2xl flex-shrink-0"
								/>
							))}
						</div>
					</div>
				)
			) : (
				<div className="overflow-x-auto">
					<div className="flex gap-2 h-64">
						<div className="w-[95vw] h-64 flex items-center justify-center text-center font-bold bg-gray-200 rounded-2xl text-3xl border">
							Start completing challenges to see your gallery!
						</div>
					</div>
				</div>
			)}
		</div>
	);
}
