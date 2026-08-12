import { QRCode } from "react-qrcode-logo";
import CampusOfBridges from "@/assets/categories/campus-of-bridges.svg";
import CoolCornersOfCarnegie from "@/assets/categories/cool-corners-of-carnegie.svg";
import LetsEat from "@/assets/categories/lets-eat.svg";
import MinorMajorGeneral from "@/assets/categories/minor-major-general.svg";
import OffCampusAdventures from "@/assets/categories/off-campus-adventures.svg";
import TheEssentials from "@/assets/categories/the-essentials.svg";

// Icons are also defined in frontend/src/lib/data/categories.ts
const categoryInfo = {
	"The Essentials": {
		dark: "bg-challenge-1",
		light: "bg-challenge-1-light",
		arc: "[--arc-color:theme(colors.challenge-1-light)]",
		iconSrc: TheEssentials,
	},
	"Campus of Bridges": {
		dark: "bg-challenge-2",
		light: "bg-challenge-2-light",
		arc: "[--arc-color:theme(colors.challenge-2-light)]",
		iconSrc: CampusOfBridges,
	},
	"Let's Eat!": {
		dark: "bg-challenge-3",
		light: "bg-challenge-3-light",
		arc: "[--arc-color:theme(colors.challenge-3-light)]",
		iconSrc: LetsEat,
	},
	"Cool Corners of Carnegie": {
		dark: "bg-challenge-4",
		light: "bg-challenge-4-light",
		arc: "[--arc-color:theme(colors.challenge-4-light)]",
		iconSrc: CoolCornersOfCarnegie,
	},
	"Minor-Major General": {
		dark: "bg-challenge-5",
		light: "bg-challenge-5-light",
		arc: "[--arc-color:theme(colors.challenge-5-light)]",
		iconSrc: MinorMajorGeneral,
	},
	"Off-Campus Adventures": {
		dark: "bg-challenge-6",
		light: "bg-challenge-6-light",
		arc: "[--arc-color:theme(colors.challenge-6-light)]",
		iconSrc: OffCampusAdventures,
	},
} as const;

type Category = keyof typeof categoryInfo;

export interface Challenge {
	category: string;
	name: string;
	tagline: string;
	secret: string;
}

export function Poster({ category, name, tagline, secret }: Challenge) {
	const info = categoryInfo[category as Category];
	const codeValue = `O-Quest challenge verification: ${secret}`;

	return (
		<div className="mt-12 font-sans">
			<div
				className={`print-content relative mx-auto shadow-lg w-[8.5in] h-[11in] transform-[scale(0.65)] origin-top ${info.light}`}
			>
				{/* Top banner */}
				<div className={`absolute flex items-end w-full ${info.dark} h-[2in]`}>
					<svg
						className={info.arc}
						viewBox="0 0 375 60"
						width="100%"
						height="60"
						preserveAspectRatio="none"
						xmlns="http://www.w3.org/2000/svg"
					>
						<title>Arc</title>
						<path
							d="M0,60 Q178,-57 375,60 L375,60 L0,60 Z"
							fill="var(--arc-color, white)"
						/>
					</svg>
				</div>

				{/* Bottom banner */}
				<div className={`flex absolute bottom-0 w-full ${info.dark} h-[1.5in]`}>
					<div className="shrink-0 py-4 w-full text-white text-center">
						<p className="text-lg font-bold underline">
							Scan this code in the O-Quest mobile app
						</p>
						<p className="italic text-sm">
							Do not remove. Property of CMU's First-Year Orientation.
						</p>
					</div>

					<div className="flex-grow" />
				</div>

				{/* Work inside the margins */}
				<div className={`absolute top-0 left-0 size-full p-[0.75in]`}>
					{/* Center icon */}
					<div
						className={`rounded-full border-8 border-white size-[1.25in] mx-auto flex items-center justify-center ${info.dark}`}
					>
						<img
							alt="Category icon"
							src={info.iconSrc}
							className="text-white stroke-2 size-1/2"
						/>
					</div>

					{/* Category name */}
					<p className={`mt-2 text-center text-3xl font-black text-white`}>
						{category}
					</p>

					{/* Center content */}
					<div className="bg-white size-[6in] flex flex-col justify-around mx-auto mt-12 rounded-2xl p-6">
						<h1 className="text-4xl text-center font-bold mx-auto">{name}</h1>

						<QRCode
							value={codeValue}
							logoImage="/logo.png"
							logoWidth={300}
							logoHeight={300}
							size={1000}
							qrStyle="dots"
							style={{
								width: "4in",
								height: "4in",
								margin: "0 auto",
							}}
						/>

						<p className="mx-auto text-center italic">{tagline}</p>
					</div>
				</div>
			</div>
		</div>
	);
}
