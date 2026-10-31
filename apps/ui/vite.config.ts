import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
    // The UI text catalogs live at the repository root, shared with the agent.
    fs: { allow: [".", "../../locales"] },
  },
  build: { target: "es2022", outDir: "dist", emptyOutDir: true },
});
