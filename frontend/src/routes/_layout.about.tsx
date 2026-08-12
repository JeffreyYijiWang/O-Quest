import { createFileRoute, useNavigate } from "@tanstack/react-router";
import type { ButtonHTMLAttributes } from "react";
import Scotty from "@/assets/about-page-scotty.svg?react";
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogHeader,
	DialogTitle,
	DialogTrigger,
} from "@/components/ui/dialog";
import { requireAuth } from "@/lib/auth";
import { contributors, contributorsByYear } from "@/lib/data/contrib";

export const Route = createFileRoute("/_layout/about")({
	beforeLoad: async ({ context }) => {
		return await requireAuth(context);
	},
	component: About,
});

function AboutButton({
	children,
	...props
}: ButtonHTMLAttributes<HTMLButtonElement>) {
	return (
		<button
			type="button"
			className="card-selected border-4 border-default-selected bg-default text-white cursor-pointer w-80 h-20 inline-flex justify-center items-center mb-4 px-4 py-2 text-2xl font-extrabold rounded-2xl"
			{...props}
		>
			{children}
		</button>
	);
}

function About() {
	const navigate = useNavigate();

	const currentYear = new Date().getFullYear();

	const currentTeam = contributors
		.filter((c) => c.contribution_year === currentYear)
		.sort((a, b) => a.lastName.localeCompare(b.lastName));

	return (
		<div className="relative flex justify-center [view-transition-name:main-content]">
			<Scotty className="w-[90%] h-[80%]" />
			<div className="absolute top-[5%] left-1/2 transform -translate-x-1/2 flex flex-col items-center leading-snug justify-start gap-12">
				<AboutButton
					onClick={() => navigate({ to: "/", search: { from: "about" } })}
				>
					How to Play
				</AboutButton>

				<Dialog>
					<DialogTrigger asChild>
						<AboutButton>Credits</AboutButton>
					</DialogTrigger>

					<DialogContent className="w-[90vw] max-w-md max-h-[90vh] bg-white overflow-y-auto mx-auto">
						<DialogHeader>
							<DialogTitle className="text-center text-2xl font-bold">
								Credits
							</DialogTitle>
							<DialogDescription className="text-center">
								Meet the people behind O-Quest
							</DialogDescription>
						</DialogHeader>

						{/* Current Team */}
						<div className="mt-2">
							<h2 className="text-xl font-semibold text-center mb-2">
								{currentYear} Team
							</h2>

							<div className="grid grid-cols-2 gap-2 mt-2">
								{currentTeam.map((c) => (
									<div
										key={`${c.firstName}-${c.lastName}-${c.role}`}
										className="contents"
									>
										<div className="text-center">{`${c.firstName} ${c.lastName} (${c.major} '${c.grad_year.toString().slice(-2)})`}</div>
										<div className="text-center">{c.role}</div>
									</div>
								))}
							</div>
						</div>

						{/* Previous Contributors */}
						{Object.keys(contributorsByYear).length > 0 && (
							<div className="mt-4">
								<h2 className="text-xl font-semibold text-center mb-2">
									Previous Contributors
								</h2>
								{Object.entries(contributorsByYear)
									.sort(([a], [b]) => Number.parseInt(b) - Number.parseInt(a))
									.map(([year, teamMembers]) => (
										<div key={year} className="mb-4">
											<h3 className="text-lg font-semibold text-center mb-2">
												{year} Team
											</h3>

											<div className="grid grid-cols-2 gap-2 mt-2">
												{teamMembers.map((c) => (
													<div
														key={`${c.firstName}-${c.lastName}-${c.role}`}
														className="contents"
													>
														<div className="text-center">{`${c.firstName} ${c.lastName} (${c.major} '${c.grad_year.toString().slice(-2)})`}</div>
														<div className="text-center">{c.role}</div>
													</div>
												))}
											</div>
										</div>
									))}
							</div>
						)}
					</DialogContent>
				</Dialog>

				<AboutButton
					onClick={() => {
						window.location.href =
							import.meta.env.VITE_FEEDBACK_URL ||
							"https://forms.gle/5AGBSfp6u1Boqrn7A";
					}}
				>
					Give Feedback
				</AboutButton>

				<AboutButton
					onClick={() => {
						const redirect =
							new URL("/dorm-select", window.location.origin).toString() +
							"?from=/about";

						window.location.href = redirect;
					}}
				>
					Change Housing Community
				</AboutButton>
			</div>
		</div>
	);
}
