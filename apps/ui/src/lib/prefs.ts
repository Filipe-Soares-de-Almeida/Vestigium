import { langFor, setLang } from "./i18n.svelte";
import type { ColorTheme, LanguagePref, Settings, ThemePref } from "./types";

interface Prefs {
  theme: ThemePref;
  color_theme: ColorTheme;
  language: LanguagePref;
}

const KEY = "vestigium.prefs";
const SCHEMES: Record<ColorTheme, string | null> = {
  Red: null,
  Graphite: "graphite",
  Slate: "slate",
  Indigo: "indigo",
};
const systemDark = matchMedia("(prefers-color-scheme: dark)");

let theme: ThemePref = "System";

/** Sets light or dark and the colour theme on the root element. */
export function applyAppearance(next: ThemePref, color: ColorTheme): void {
  theme = next;
  const root = document.documentElement;
  const dark = next === "Dark" || (next === "System" && systemDark.matches);
  root.dataset.theme = dark ? "dark" : "light";
  const scheme = SCHEMES[color];
  if (scheme) root.dataset.scheme = scheme;
  else delete root.dataset.scheme;
}

systemDark.addEventListener("change", () => {
  if (theme === "System") {
    document.documentElement.dataset.theme = systemDark.matches ? "dark" : "light";
  }
});

/** Applies the settings' appearance at once, and their language only when `language` is set. */
export function applySettings(settings: Settings, { language }: { language: boolean }): void {
  applyAppearance(settings.theme, settings.color_theme);
  if (language) setLang(langFor(settings.language));
  remember({ theme: settings.theme, color_theme: settings.color_theme, language: settings.language });
}

/** Applies what the last session saw, before the watcher answers. */
export function restorePrefs(): void {
  const prefs = recall();
  applyAppearance(prefs?.theme ?? "System", prefs?.color_theme ?? "Red");
  setLang(langFor(prefs?.language ?? "System"));
}

function remember(prefs: Prefs): void {
  try {
    localStorage.setItem(KEY, JSON.stringify(prefs));
  } catch {
    // Without storage the next start follows Windows until the watcher answers.
  }
}

function recall(): Prefs | null {
  try {
    const raw = localStorage.getItem(KEY);
    return raw ? (JSON.parse(raw) as Prefs) : null;
  } catch {
    return null;
  }
}
