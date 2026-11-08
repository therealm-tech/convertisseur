import { resolve } from "node:path";
import { minimal2023Preset } from "@vite-pwa/assets-generator/config";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";
import { VitePWA } from "vite-plugin-pwa";

const cream = "#fefbf5";

export default defineConfig({
	// The repository's assets/ holds the logo, the source of the favicon and
	// of the PWA icons, which the build generates into the output directory.
	publicDir: "../assets",
	plugins: [
		react(),
		VitePWA({
			registerType: "autoUpdate",
			injectRegister: false,
			pwaAssets: {
				injectThemeColor: false,
				image: "../assets/logo.svg",
				preset: {
					...minimal2023Preset,
					maskable: {
						...minimal2023Preset.maskable,
						resizeOptions: { background: cream },
					},
					apple: {
						...minimal2023Preset.apple,
						resizeOptions: { background: cream },
					},
				},
			},
			manifest: {
				name: "Convertisseur",
				short_name: "Convertisseur",
				description: "L'arabe et les bases de l'Islam, un petit jeu à la fois.",
				lang: "fr",
				start_url: "/",
				display: "standalone",
				background_color: cream,
				theme_color: "#0c6a3c",
			},
			workbox: {
				globPatterns: ["**/*.{js,css,html,svg,png,ico,woff2}"],
				navigateFallbackDenylist: [
					/^\/api(\/|$)/,
					/^\/docs$/,
					/^\/openapi\.json$/,
				],
			},
		}),
	],
	resolve: {
		alias: {
			"@": resolve(import.meta.dirname, "src"),
		},
	},
	server: {
		proxy: {
			"/api": "http://localhost:3000",
		},
	},
});
