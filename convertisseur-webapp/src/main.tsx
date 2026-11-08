import "@fontsource-variable/nunito";
import "@fontsource/amiri/400.css";
import "@fontsource/amiri/700.css";
import "./styles.css";
import { registerSW } from "virtual:pwa-register";
import { RouterProvider } from "@tanstack/react-router";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { logger } from "@/lib/logger";
import { createAppRouter } from "@/router";

registerSW({
	immediate: true,
	onRegisteredSW: (swUrl) => logger.debug("sw.registered", { swUrl }),
	onRegisterError: (error: unknown) =>
		logger.error("sw.register_failed", { error }),
});

const router = createAppRouter();

const root = document.getElementById("root");
if (root) {
	createRoot(root).render(
		<StrictMode>
			<RouterProvider router={router} />
		</StrictMode>,
	);
}
