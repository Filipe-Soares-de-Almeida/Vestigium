// Screens are addressed by the URL hash: #/today, #/occurrence/<id>, #/now, #/captures[?for=<occurrence id>],
// #/capture/<id>, #/health[/<area>], #/dumps[/<id>], #/settings[/<category>].

export type Route =
  | { screen: "today" }
  | { screen: "occurrence"; id: number }
  | { screen: "now" }
  | { screen: "captures"; forOccurrence: number | null }
  | { screen: "capture"; id: number }
  | { screen: "health"; area: string | null }
  | { screen: "dumps"; id: number | null }
  | { screen: "settings"; category: string | null };

export type Screen = Route["screen"];

export const href = {
  today: () => "#/today",
  occurrence: (id: number) => `#/occurrence/${id}`,
  now: () => "#/now",
  captures: (forOccurrence?: number) => (forOccurrence ? `#/captures?for=${forOccurrence}` : "#/captures"),
  capture: (id: number) => `#/capture/${id}`,
  health: (area?: string) => (area ? `#/health/${area}` : "#/health"),
  dumps: (id?: number) => (id ? `#/dumps/${id}` : "#/dumps"),
  settings: (category?: string) => (category ? `#/settings/${category}` : "#/settings"),
};

export function parse(hash: string): Route {
  const [path, query = ""] = hash.replace(/^#\/?/, "").split("?");
  const [screen, arg] = path.split("/");
  if (screen === "occurrence" && /^\d+$/.test(arg ?? "")) {
    return { screen: "occurrence", id: Number(arg) };
  }
  if (screen === "now") return { screen: "now" };
  if (screen === "capture" && /^\d+$/.test(arg ?? "")) return { screen: "capture", id: Number(arg) };
  if (screen === "captures") {
    const given = new URLSearchParams(query).get("for") ?? "";
    return { screen: "captures", forOccurrence: /^\d+$/.test(given) ? Number(given) : null };
  }
  if (screen === "health") return { screen: "health", area: arg || null };
  if (screen === "dumps") return { screen: "dumps", id: /^\d+$/.test(arg ?? "") ? Number(arg) : null };
  if (screen === "settings") return { screen: "settings", category: arg || null };
  return { screen: "today" };
}

class Router {
  current = $state<Route>(parse(location.hash));

  /** Changes only when the screen changes, not when a screen moves inside itself. */
  get key(): string {
    const route = this.current;
    if (route.screen === "occurrence" || route.screen === "capture") return `${route.screen}/${route.id}`;
    if (route.screen === "health") return `health/${route.area ?? ""}`;
    return route.screen === "captures" ? `captures/${route.forOccurrence ?? ""}` : route.screen;
  }

  /** The navigation pane item a screen belongs to. */
  get section(): "today" | "now" | "captures" | "health" | "dumps" | "settings" {
    const screen = this.current.screen;
    return screen === "occurrence" ? "today" : screen === "capture" ? "captures" : screen;
  }
}

export const router = new Router();

addEventListener("hashchange", () => {
  router.current = parse(location.hash);
});

/** Opens a screen, as in `go(href.now())`. */
export function go(path: string): void {
  location.hash = path;
}
