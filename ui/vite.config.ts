import { paraglideVitePlugin } from "@inlang/paraglide-js";
import tailwindcss from "@tailwindcss/vite";
import { tanstackRouter } from "@tanstack/router-plugin/vite";
import react from "@vitejs/plugin-react";
import { VitePWA } from "vite-plugin-pwa";
import { defineConfig } from "vitest/config";

/** Der Rust-Server (`bun run dev:server`); im Betrieb liefert er die gebaute UI selbst aus. */
const server = process.env.FIXPUNKT_SERVER ?? "http://127.0.0.1:8080";

export default defineConfig({
  plugins: [
    // Sprache: Wahl in den Einstellungen (dieses Gerät), sonst Browser, sonst Deutsch
    paraglideVitePlugin({
      project: "./project.inlang",
      outdir: "./src/paraglide",
      outputStructure: process.env.NODE_ENV === "production" ? "message-modules" : "locale-modules",
      strategy: ["localStorage", "preferredLanguage", "baseLocale"],
    }),
    tanstackRouter({ target: "react", autoCodeSplitting: true }),
    react(),
    tailwindcss(),
    // Auf dem Tablet/Handy der Baustelle installierbar; API-Daten werden nie zwischengespeichert
    VitePWA({
      registerType: "autoUpdate",
      injectRegister: "script-defer",
      manifest: {
        name: "Fixpunkt",
        short_name: "Fixpunkt",
        description: "Georeferenzierte 3D-Baustellendokumentation und Massenermittlung",
        lang: "de",
        start_url: "/",
        scope: "/",
        display: "standalone",
        theme_color: "#ffffff",
        background_color: "#ffffff",
        icons: [{ src: "/favicon.svg", sizes: "any", type: "image/svg+xml", purpose: "any" }],
      },
      workbox: {
        globPatterns: ["**/*.{js,css,html,svg,woff2}"],
        globIgnores: ["**/geist-*-{cyrillic,greek,vietnamese}*"],
        navigateFallbackDenylist: [/^\/api\//],
      },
    }),
  ],
  resolve: { alias: { "@": new URL("./src", import.meta.url).pathname } },
  server: {
    port: Number(process.env.PORT ?? 5173),
    strictPort: true,
    proxy: { "/api": { target: server } },
  },
  build: { outDir: "dist", emptyOutDir: true },
  test: {
    environment: "jsdom",
    include: ["src/**/*.test.{ts,tsx}"],
    setupFiles: ["./src/test-setup.ts"],
  },
});
