// Screens are addressed by the URL hash: #/ (home), #/activity[?tab=now], #/history[?m=YYYY-MM&p=month|week|2w&d=YYYY-MM-DD],
// #/disks, #/memory, #/activity/programs, #/activity/all, #/period/<start ms>[?day=YYYY-MM-DD], #/occurrence/<id>[?day=YYYY-MM-DD], #/captures[?for=<occurrence id>],
// #/capture/<id>, #/health[/<area>], #/dumps[/<id>], #/tweaks[/<key>], #/tests[/<device>],
// #/settings[/<category>]. #/today and #/now open those tabs of #/activity. A `g=<group>` query names the
// sidebar group an entry shared by two groups was opened from.

export type ActivityTab = "today" | "now";

export type HistoryScope = "month" | "week" | "2w";

/** Which tweaks a list shows: those worth applying, with no sign here, already set, applied by Vestigium, or all. */
export type TweakGroup = "recommended" | "quiet" | "same" | "done" | "all";
const TWEAK_GROUPS: TweakGroup[] = ["recommended", "quiet", "same", "done", "all"];

export type Route =
  | { screen: "home" }
  | { screen: "activity"; tab: ActivityTab }
  | { screen: "programs" }
  | { screen: "disks" }
  | { screen: "memory" }
  | { screen: "events" }
  /** `start` is the start of the episode in Unix milliseconds. */
  | { screen: "period"; start: number; fromDay: string | null }
  | { screen: "history"; month: string | null; scope: HistoryScope; day: string | null }
  /** `fromDay` is set when the report was opened from a day of the History. */
  | { screen: "occurrence"; id: number; fromDay: string | null }
  | { screen: "captures"; forOccurrence: number | null }
  | { screen: "capture"; id: number }
  | { screen: "health"; area: string | null }
  | { screen: "dumps"; id: number | null }
  | { screen: "tweaks"; key: string | null; group: TweakGroup | null }
  /** `device` is "memory" or a physical disk number. */
  | { screen: "tests"; device: string | null }
  | { screen: "settings"; category: string | null };

export type Screen = Route["screen"];

export const href = {
  home: () => "#/",
  programs: () => "#/activity/programs",
  disks: () => "#/disks",
  memory: () => "#/memory",
  events: () => "#/activity/all",
  period: (start: number, fromDay?: string | null) => (fromDay ? `#/period/${start}?day=${fromDay}` : `#/period/${start}`),
  activity: (tab: ActivityTab = "today") => (tab === "now" ? "#/activity?tab=now" : "#/activity"),
  history: (opts: { month?: string | null; scope?: HistoryScope; day?: string | null } = {}) => {
    const query = new URLSearchParams();
    if (opts.month) query.set("m", opts.month);
    if (opts.scope && opts.scope !== "month") query.set("p", opts.scope);
    if (opts.day) query.set("d", opts.day);
    const text = query.toString();
    return text ? `#/history?${text}` : "#/history";
  },
  occurrence: (id: number, fromDay?: string | null) => (fromDay ? `#/occurrence/${id}?day=${fromDay}` : `#/occurrence/${id}`),
  captures: (forOccurrence?: number) => (forOccurrence ? `#/captures?for=${forOccurrence}` : "#/captures"),
  capture: (id: number) => `#/capture/${id}`,
  health: (area?: string) => (area ? `#/health/${area}` : "#/health"),
  dumps: (id?: number) => (id ? `#/dumps/${id}` : "#/dumps"),
  tweaks: (key?: string | null, group?: TweakGroup | null) =>
    key ? `#/tweaks/${encodeURIComponent(key)}` : group && group !== "recommended" ? `#/tweaks?group=${group}` : "#/tweaks",
  tests: (device?: string) => (device ? `#/tests/${device}` : "#/tests"),
  settings: (category?: string) => (category ? `#/settings/${category}` : "#/settings"),
};

export function parse(hash: string): Route {
  const [path, query = ""] = hash.replace(/^#\/?/, "").split("?");
  const [screen, arg] = path.split("/");
  const params = new URLSearchParams(query);
  const dayParam = (name: string) => (/^\d{4}-\d{2}-\d{2}$/.test(params.get(name) ?? "") ? params.get(name) : null);
  if (screen === "occurrence" && /^\d+$/.test(arg ?? "")) {
    return { screen: "occurrence", id: Number(arg), fromDay: dayParam("day") };
  }
  if (screen === "history") {
    const month = /^\d{4}-\d{2}$/.test(params.get("m") ?? "") ? params.get("m") : null;
    const given = params.get("p");
    return { screen: "history", month, scope: given === "week" || given === "2w" ? given : "month", day: dayParam("d") };
  }
  if (screen === "events" || (screen === "activity" && arg === "all")) return { screen: "events" };
  if (screen === "period" && /^\d+$/.test(arg ?? "")) return { screen: "period", start: Number(arg), fromDay: dayParam("day") };
  if (screen === "disks") return { screen: "disks" };
  if (screen === "memory") return { screen: "memory" };
  if (screen === "programs" || (screen === "activity" && arg === "programs")) return { screen: "programs" };
  if (screen === "now") return { screen: "activity", tab: "now" };
  if (screen === "today") return { screen: "activity", tab: "today" };
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
  if (screen === "tweaks") {
    const asked = params.get("group") as TweakGroup | null;
    return {
      screen: "tweaks",
      key: arg ? decodeURIComponent(arg) : null,
      group: asked && TWEAK_GROUPS.includes(asked) ? asked : null,
    };
  }
  if (screen === "tests") return { screen: "tests", device: arg === "memory" || /^\d+$/.test(arg ?? "") ? arg : null };
  if (screen === "settings") return { screen: "settings", category: arg || null };
  return { screen: "home" };
}

/** The sidebar group named by the `g` query, if any. */
export function groupParam(hash: string): string | null {
  return new URLSearchParams(hash.split("?")[1] ?? "").get("g");
}

/** Called before a change of screen; returns true when it took over and will call `commit` itself. */
export type NavigationHook = (next: Route, prev: Route, commit: () => void) => boolean;

class Router {
  current = $state<Route>(parse(location.hash));
  /** The `g` query of the current address. */
  group = $state<string | null>(groupParam(location.hash));
  hook: NavigationHook | null = null;

  commit(next: Route, group: string | null): void {
    this.current = next;
    this.group = group;
  }

  /** Changes only when the screen changes, not when a screen moves inside itself or between its tabs. */
  get key(): string {
    const route = this.current;
    if (route.screen === "occurrence" || route.screen === "capture") return `${route.screen}/${route.id}`;
    if (route.screen === "period") return `period/${route.start}`;
    if (route.screen === "health") return `health/${route.area ?? ""}`;
    return route.screen === "captures" ? `captures/${route.forOccurrence ?? ""}` : route.screen;
  }
}

export const router = new Router();

addEventListener("hashchange", () => {
  const next = parse(location.hash);
  const group = groupParam(location.hash);
  const commit = () => router.commit(next, group);
  if (!router.hook?.(next, router.current, commit)) commit();
});

/** Opens a screen, as in `go(href.activity("now"))`. */
export function go(path: string): void {
  location.hash = path;
}
