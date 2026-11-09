import "./app.css";

import { isTauri } from "@tauri-apps/api/core";
import { mount } from "svelte";

import App from "./App.svelte";
import { restorePrefs } from "./lib/prefs";

// Outside the Tauri window (npm run dev in a browser), a stand-in answers the commands.
if (import.meta.env.DEV && !isTauri()) {
  await import("./dev/mock");
}

restorePrefs();

const target = document.getElementById("app");
if (target) {
  mount(App, { target });
}
