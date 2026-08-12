import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { ReactQueryDevtools } from "@tanstack/react-query-devtools";
import { createRouter, RouterProvider } from "@tanstack/react-router";
import { TanStackRouterDevtools } from "@tanstack/react-router-devtools";
import { StrictMode } from "react";
import ReactDOM from "react-dom/client";
import { AppProvider, useApi } from "@/lib/app-context";
import { routeTree } from "@/routeTree.gen";
import "@/main.css";

const queryClient = new QueryClient({
	defaultOptions: {
		queries: {
			// Cache data for 15 minutes before considering it stale
			staleTime: 15 * 60 * 1000,
			// Keep data in cache for 30 minutes after component unmounts
			gcTime: 30 * 60 * 1000,
			refetchOnWindowFocus: false,
			refetchOnMount: false,
			refetchOnReconnect: false,
			retry: 1,
		},
	},
});

const router = createRouter({
	routeTree,
	scrollRestoration: true,
	defaultViewTransition: true,
	// AppProvider supplies the actual value before any route is rendered. Keep the
	// type-level default local so initialization can never contact a retired API.
	context: { baseUrl: "http://localhost:3000", queryClient },
});

// Register the router instance for type safety
declare module "@tanstack/react-router" {
	interface Register {
		router: typeof router;
	}
}

function AppWithRouter() {
	const { baseUrl } = useApi();

	return <RouterProvider router={router} context={{ baseUrl, queryClient }} />;
}

console.log(
	"%cScottyLabs\n%cJoin us! %chttps://scottylabs.org",
	"color: #01A4F8; font-weight: bold; font-size: 48px;",
	"color: #fff; font-size: 14px;",
	"color: #01A4F8; font-size: 14px; text-decoration: underline;",
);

// biome-ignore lint/style/noNonNullAssertion: guaranteed to exist in index.html
const rootElement = document.getElementById("root")!;

if (!rootElement.innerHTML) {
	const root = ReactDOM.createRoot(rootElement);
	root.render(
		<StrictMode>
			<QueryClientProvider client={queryClient}>
				<AppProvider>
					<AppWithRouter />

					<ReactQueryDevtools initialIsOpen={false} />
					<TanStackRouterDevtools router={router} />
				</AppProvider>
			</QueryClientProvider>
		</StrictMode>,
	);
}
