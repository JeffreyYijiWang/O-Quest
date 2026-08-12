import createFetchClient from "openapi-fetch";
import createClient from "openapi-react-query";
import { createContext, type ReactNode, useContext, useState } from "react";
import type { components, paths } from "@/lib/schema.gen";

const CLIENT = "quest";

// App state types
export type FilterOption = components["schemas"]["ChallengeStatus"] | "all";
export type AdminMode = "challenges" | "verify";

const getApiConfig = (): string => {
	const configuredBaseUrl = import.meta.env.VITE_API_BASE_URL?.trim();
	if (configuredBaseUrl) {
		return configuredBaseUrl.replace(/\/$/, "");
	}

	const { hostname } = window.location;

	if (
		hostname === "localhost" ||
		hostname === "127.0.0.1" ||
		hostname === "tauri.localhost"
	) {
		return "http://localhost:3000";
	}

	// Web deployments use a same-origin API by default. Native builds should set
	// VITE_API_BASE_URL to the address of the replacement backend.
	return window.location.origin;
};

const createApiClient = (baseUrl: string) => {
	const fetchClient = createFetchClient<paths>({
		baseUrl,
		credentials: "include",
	});
	return {
		baseUrl,
		$api: createClient(fetchClient),

		async logout() {
			const form = document.createElement("form");
			form.method = "POST";
			form.action = `${baseUrl}/logout`;

			document.body.appendChild(form);
			form.submit();
		},

		async login(path: string) {
			const isAuthd = await new Promise<boolean>((resolve) => {
				fetch(`${baseUrl}/api/profile`, { credentials: "include" }).then((res) => {
					if (res.ok) {
						resolve(true);
					} else {
						resolve(false);
					}
				});
			});

			if (isAuthd) {
				window.location.href = new URL(path, window.location.origin).toString();
				return;
			}

			const redirect = new URL(path, window.location.origin).toString();
			const loginUrl = `${baseUrl}/oauth2/authorization/${CLIENT}?redirect_uri=${encodeURIComponent(redirect)}`;

			window.location.href = loginUrl;
		},
	};
};

// Context setup
type AppContextType = ReturnType<typeof createApiClient> & {
	filter: FilterOption;
	setFilter: (filter: FilterOption) => void;
	adminMode: AdminMode;
	setAdminMode: (mode: AdminMode) => void;
};
const AppContext = createContext<AppContextType | null>(null);

export function AppProvider({ children }: { children: ReactNode }) {
	const client = createApiClient(getApiConfig());
	const [filter, setFilter] = useState<FilterOption>("all");
	const [adminMode, setAdminMode] = useState<AdminMode>("challenges");

	const contextValue: AppContextType = {
		...client,
		filter,
		setFilter,
		adminMode,
		setAdminMode,
	};

	return (
		<AppContext.Provider value={contextValue}>{children}</AppContext.Provider>
	);
}

// Hooks
export function useApi() {
	const context = useContext(AppContext);
	if (!context) {
		throw new Error("useApi must be used within an AppProvider");
	}

	return context;
}

export const useAppContext = () => {
	const context = useContext(AppContext);
	if (!context) {
		throw new Error("useAppContext must be used within an AppProvider");
	}

	return {
		filter: context.filter,
		setFilter: context.setFilter,
		adminMode: context.adminMode,
		setAdminMode: context.setAdminMode,
	};
};
