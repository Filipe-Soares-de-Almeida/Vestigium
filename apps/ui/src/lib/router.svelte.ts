// Screens are addressed by the URL hash: #/today, #/occurrence/<id>, #/now, #/settings[/<category>].

export type Route =
  | { screen: "today" }
  | { screen: "occurrence"; id: number }
  | { screen: "now" }
  | { screen: "settings"; category: string | null };

export type Screen = Route["screen"];

export const href = {
  today: () => "#/today",
  occurrence: (id: number) => `#/occurrence/${id}`,
  now: () => "#/now",
  settings: (category?: string) => (category ? `#/settings/${category}` : "#/settings"),
};

export function parse(hash: string): Route {
  const [screen, arg] = hash.replace(/^#\/?/, "").split("/");
  if (screen === "occurrence" && /^\d+$/.test(arg ?? "")) {
    return { screen: "occurrence", id: Number(arg) };
  }
  if (screen === "now") return { screen: "now" };
  if (screen === "settings") return { screen: "settings", category: arg || null };
  return { screen: "today" };
}

class Router {
  current = $state<Route>(parse(location.hash));

  /** Changes only when the screen changes, not when a screen moves inside itself. */
  get key(): string {
    return this.current.screen === "occurrence" ? `occurrence/${this.current.id}` : this.current.screen;
  }

  /** The navigation pane item a screen belongs to. */
  get section(): "today" | "now" | "settings" {
    return this.current.screen === "occurrence" ? "today" : this.current.screen;
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
