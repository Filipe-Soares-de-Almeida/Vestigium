import { svelte } from "@sveltejs/vite-plugin-svelte";
import { defineConfig } from "vitest/config";

export default defineConfig({
  plugins: [svelte()],
  // Svelte components resolve to their browser build, so mount() is available.
  resolve: { conditions: ["browser"] },
  test: {
    environment: "jsdom",
    include: ["src/**/*.spec.ts"],
    setupFiles: ["src/dev/test-setup.ts"],
  },
});
