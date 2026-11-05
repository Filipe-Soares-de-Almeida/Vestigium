// Screens are addressed by the URL hash: #/activity[?tab=now], #/occurrence/<id>, #/captures[?for=<occurrence id>],
// #/capture/<id>, #/health[/<area>], #/dumps[/<id>], #/tweaks[/<key>], #/tests[/<device>],
// #/settings[/<category>]. #/today and #/now open those tabs of #/activity.

export type ActivityTab = "today" | "now";

export type Route =
  | { screen: "activity"; tab: ActivityTab }
  | { screen: "occurrence"; id: number }
  | { screen: "captures"; forOccurrence: number | null }
  | { screen: "capture"; id: number }
  | { screen: "health"; area: string | null }
  | { screen: "dumps"; id: number | null }
  | { screen: "tweaks"; key: string | null }
  /** `device` is "memory" or a physical disk number. */
  | { screen: "tests"; device: string | null }
  | { screen: "settings"; category: string | null };

export type Screen = Route["screen"];

export const href = {
  activity: (tab: ActivityTab = "today") => (tab === "now" ? "#/activity?tab=now" : "#/activity"),
  occurrence: (id: number) => `#/occurrence/${id}`,
  captures: (forOccurrence?: number) => (forOccurrence ? `#/captures?for=${forOccurrence}` : "#/captures"),
  capture: (id: number) => `#/capture/${id}`,
  health: (area?: string) => (area ? `#/health/${area}` : "#/health"),
  dumps: (id?: number) => (id ? `#/dumps/${id}` : "#/dumps"),
  tweaks: (key?: string) => (key ? `#/tweaks/${encodeURIComponent(key)}` : "#/tweaks"),
  tests: (device?: string) => (device ? `#/tests/${device}` : "#/tests"),
  settings: (category?: string) => (category ? `#/settings/${category}` : "#/settings"),
};

export function parse(hash: string): Route {
  const [path, query = ""] = hash.replace(/^#\/?/, "").split("?");
  const [screen, arg] = path.split("/");
  if (screen === "occurrence" && /^\d+$/.test(arg ?? "")) {
    return { screen: "occurrence", id: Number(arg) };
  }
  if (screen === "now") return { screen: "activity", tab: "now" };
  if (screen === "activity") {
    return { screen: "activity", tab: new URLSearchParams(query).get("tab") === "now" ? "now" : "today" };
  }
  if (screen === "capture" && /^\d+$/.test(arg ?? "")) return { screen: "capture", id: Number(arg) };
  if (screen === "captures") {
    const given = new URLSearchParams(query).get("for") ?? "";
    return { screen: "captures", forOccurrence: /^\d+$/.test(given) ? Number(given) : null };
  }
  if (screen === "health") return { screen: "health", area: arg || null };
  if (screen === "dumps") return { screen: "dumps", id: /^\d+$/.test(arg ?? "") ? Number(arg) : null };
  if (screen === "tweaks") return { screen: "tweaks", key: arg ? decodeURIComponent(arg) : null };
  if (screen === "tests") return { screen: "tests", device: arg === "memory" || /^\d+$/.test(arg ?? "") ? arg : null };
  if (screen === "settings") return { screen: "settings", category: arg || null };
  return { screen: "activity", tab: "today" };
}

class Router {
  current = $state<Route>(parse(location.hash));

  /** Changes only when the screen changes, not when a screen moves inside itself or between its tabs. */
  get key(): string {
    const route = this.current;
    if (route.screen === "occurrence" || route.screen === "capture") return `${route.screen}/${route.id}`;
    if (route.screen === "health") return `health/${route.area ?? ""}`;
    return route.screen === "captures" ? `captures/${route.forOccurrence ?? ""}` : route.screen;
  }

  /** The navigation pane item a screen belongs to. */
  get section(): "activity" | "captures" | "health" | "dumps" | "tweaks" | "tests" | "settings" {
    const screen = this.current.screen;
    return screen === "occurrence" ? "activity" : screen === "capture" ? "captures" : screen;
  }
}

export const router = new Router();

addEventListener("hashchange", () => {
  router.current = parse(location.hash);
});

/** Opens a screen, as in `go(href.activity("now"))`. */
export function go(path: string): void {
  location.hash = path;
}
