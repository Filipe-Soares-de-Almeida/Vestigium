import type { IconName } from "../components/Icon.svelte";
import { type AreaState, areaState, AREAS, statusKindOf } from "../screens/health/areas";
import { formatBytes } from "../screens/occurrence/format";
import { app } from "./app.svelte";
import { captures } from "./capture.svelte";
import { dumps } from "./dumps.svelte";
import { health } from "./health.svelte";
import { helperJobs, type JobKey } from "./helper.svelte";
import { passState } from "./monitor";
import { type DumpGroup, href, type Route, type TweakGroup } from "./router.svelte";
import type { StatusKind } from "../components/StatusMark.svelte";
import { keyOf, tweaks } from "./tweaks.svelte";
import { shown, type Shown } from "../screens/tweaks/words";
import { entries, GROUPS as DUMP_GROUPS, present } from "../screens/dumps/words";
import { hwtests } from "./tests.svelte";
import { today } from "./today.svelte";
import type { DiskDto } from "./types";
import { lang, t, tn } from "./i18n.svelte";
import { updates } from "./update.svelte";
import { statusOf } from "../screens/tests/status";
import { diskState, memoryState } from "../screens/tests/words";

// The sidebar groups: each subject opens into the pages that belong to it.

export type GroupKey =
  | "activity"
  | "now"
  | "freezes"
  | "health"
  | "disks"
  | "memory"
  | "tests"
  | "tweaks"
  | "dumps"
  | "captures"
  | "settings";

export interface NavItem {
  href: string;
  label: string;
  icon: IconName;
  /** A status shown as its symbol; the word appears on hover. */
  status?: StatusKind;
  /** Replaces the default word of the status. */
  statusWord?: string;
  /** The entry's helper job did not finish in time and nobody was told yet. */
  late?: boolean;
  /** The entry's job is running now. */
  live?: boolean;
  /** A short trailing value, such as a size. */
  meta?: string;
  /** True for the routes this entry stands for. */
  match: (route: Route) => boolean;
}

export interface Group {
  key: GroupKey;
  icon: IconName;
  name: string;
  sub: string;
  /** Kept out of the home grid; reachable from its own card or the search. */
  hidden?: boolean;
  /** Entries also listed in another group carry `g=<key>` so the sidebar knows where they were opened from. */
  cross?: boolean;
  items: NavItem[];
}

export const SETTINGS_PAGES = ["appearance", "reports", "language", "watching", "history", "updates", "privacy"] as const;

const SETTINGS_ICONS: Record<(typeof SETTINGS_PAGES)[number], IconName> = {
  appearance: "eye",
  reports: "dumps",
  language: "info",
  watching: "now",
  history: "clock",
  updates: "restart",
  privacy: "lock",
};

export const diskName = (d: DiskDto) => t(`ui.tests.disk.name.${d.media}`, { size: d.bytes ? formatBytes(d.bytes) : "" }).trim();
/** "C:, E:" for the volumes of a disk. */
const lettersOf = (d: DiskDto) => d.volumes.map((v) => v.replace(/\\$/, "")).join(", ");
/** A disk as the navigation pane names it: "SSD de 1 TB · C:". */
const diskLabel = (d: DiskDto) => (d.volumes.length ? `${diskName(d)} · ${lettersOf(d)}` : diskName(d));
/** The status a test device shows in the pane: only what needs a look. */
function deviceMark(mark: Parameters<typeof statusOf>[0]): StatusKind | undefined {
  const kind = statusOf(mark).kind;
  return kind === "critical" || kind === "attention" ? kind : undefined;
}

const onActivity = (tab: "today" | "now") => (r: Route) => r.screen === "activity" && r.tab === tab;
const fromHistory = (r: Route) => (r.screen === "occurrence" || r.screen === "period") && r.fromDay !== null;
const onHistory = (r: Route) => r.screen === "history" || fromHistory(r);
const onHealth = (area: string | null) => (r: Route) => r.screen === "health" && (r.area ?? null) === area;
const onTests = (device: string | null) => (r: Route) => r.screen === "tests" && (r.device ?? null) === device;
/** What an area shows in the pane: only what needs a look, which includes what was not verified. */
const mark = (state: AreaState): StatusKind | undefined => {
  const kind = statusKindOf(state.mark, state.why);
  return kind === "critical" || kind === "attention" || kind === "unverified" || kind === "silent" ? kind : undefined;
};

const DUMP_ICON: Record<DumpGroup, IconName> = { blue: "pc", live: "chip", app: "app" };

/** The attention mark an entry takes while its job's helper did not finish in time and nobody was told. */
const lateMark = (job: JobKey): Pick<NavItem, "status" | "statusWord" | "late"> =>
  helperJobs.unseen(job) ? { status: "attention", statusWord: t("ui.helper.timeout.title"), late: true } : {};

/** The information mark the captures entry takes while a deep capture runs, whoever started it. */
const liveMark = (): Pick<NavItem, "status" | "statusWord" | "live"> =>
  captures.run !== null ? { status: "info", statusWord: t("ui.captures.nav.running"), live: true } : {};

/** Whether a group holds an entry whose helper job did not finish in time and nobody was told yet. */
export const hasLate = (group: Group): boolean => group.items.some((item) => item.late);

/** Whether a group holds an entry whose job runs now and has no late mark of its own. */
export const hasLive = (group: Group): boolean => group.items.some((item) => item.live && !item.late);

/** The dump list a route shows: the one asked for, or the one of the dump it opens; none for the overview. */
function dumpListOf(r: Route): DumpGroup | null {
  if (r.screen !== "dumps") return null;
  if (r.id !== null && dumps.dto) return entries(dumps.dto).find((e) => e.dump?.id === r.id)?.group ?? null;
  return r.group;
}

/** The list a tweak belongs to, by how it stands. */
export function tweakGroupOf(state: Shown): Exclude<TweakGroup, "all"> {
  return state === "signal" ? "recommended" : state === "quiet" ? "quiet" : state === "applied" ? "done" : "same";
}

/** The list a route shows: the one asked for, or the one of the tweak it opens. */
function tweakListOf(r: Route): TweakGroup | null {
  if (r.screen !== "tweaks") return null;
  if (r.key !== null) {
    const advice = tweaks.dto?.advice.find((a) => keyOf(a) === r.key);
    return advice ? tweakGroupOf(shown(advice, tweaks.dto?.history ?? [])) : "recommended";
  }
  return r.group ?? "recommended";
}

/** Builds every group from what the stores hold now; read it inside `$derived` to follow them. */
export function buildGroups(): Group[] {
  const disks = hwtests.dto?.disks ?? [];
  const pass = passState(app.link, Date.now());
  const areas = health.dto ? AREAS.map((a) => areaState(a, health.dto!, pass)) : [];
  const dumpBytes = dumps.dto?.space_bytes ?? 0;
  const dumpEntries = dumps.dto ? entries(dumps.dto) : [];
  const occurrences = today.occurrences.length;
  const periods = today.periods;
  // Until today's marks have been read, the counts are unknown and say nothing.
  const counted = today.loaded;
  const list = new Intl.ListFormat(lang(), { type: "conjunction" });
  const memory = hwtests.dto?.memory ?? null;
  const subs: Partial<Record<GroupKey, string>> = {
    freezes: counted
      ? t("ui.group.freezes.sub_n", {
          occurrences: occurrences ? tn("ui.events.occurrences", occurrences) : t("ui.events.occurrences.none"),
          periods: periods ? tn("ui.events.periods", periods) : t("ui.events.periods.none"),
        })
      : undefined,
    disks: disks.length
      ? list.format(disks.map((d) => (d.volumes.length ? `${diskName(d)} (${lettersOf(d)})` : diskName(d))))
      : undefined,
    memory: memory
      ? t("ui.group.memory.sub_n", { total: formatBytes(memory.total_bytes), free: formatBytes(memory.available_bytes) })
      : undefined,
    dumps: dumpBytes ? t("ui.group.dumps.sub_n", { size: formatBytes(dumpBytes) }) : undefined,
    settings: t("ui.group.settings.sub_n", { version: updates.version }),
  };
  const diskArea = areas.find((a) => a.area.id === "disks");
  const g = (key: GroupKey, icon: IconName, extra: Partial<Group> & Pick<Group, "items">): Group => ({
    key,
    icon,
    name: t(`ui.group.${key}`),
    sub: subs[key] ?? t(`ui.group.${key}.sub`),
    ...extra,
  });
  return [
    g("activity", "today", {
      items: [
        { href: href.activity(), label: t("ui.item.today"), icon: "today", match: onActivity("today") },
        { href: href.history(), label: t("ui.item.history"), icon: "clock", match: onHistory },
      ],
    }),
    g("now", "now", {
      items: [
        { href: href.activity("now"), label: t("ui.item.live"), icon: "now", match: onActivity("now") },
        { href: href.programs(), label: t("ui.item.programs"), icon: "app", match: (r) => r.screen === "programs" },
      ],
    }),
    g("freezes", "wave", {
      items: [
        {
          href: href.events(),
          label: t("ui.item.events"),
          icon: "today",
          meta: counted ? String(occurrences + periods) : undefined,
          match: (r) => r.screen === "events" || ((r.screen === "occurrence" || r.screen === "period") && r.fromDay === null),
        },
      ],
    }),
    g("health", "health", {
      items: [
        { href: href.health(), label: t("ui.item.overview"), icon: "health", ...lateMark("health"), match: onHealth(null) },
        ...AREAS.map((a, i) => ({
          href: href.health(a.id),
          label: t(`ui.health.area.${a.id}`),
          icon: a.icon,
          status: areas[i] ? mark(areas[i]) : undefined,
          match: onHealth(a.id),
        })),
      ],
    }),
    g("disks", "disk", {
      cross: true,
      items: [
        { href: href.disks(), label: t("ui.item.overview"), icon: "disk", match: (r) => r.screen === "disks" },
        {
          href: href.health("disks"),
          label: t("ui.item.disk_alerts"),
          icon: "health",
          status: diskArea ? mark(diskArea) : undefined,
          match: onHealth("disks"),
        },
        ...disks.map((d) => ({
          href: href.tests(String(d.number)),
          label: diskLabel(d),
          icon: (d.media === "Hdd" ? "hdd" : "ssd") as IconName,
          match: onTests(String(d.number)),
        })),
      ],
    }),
    g("memory", "memory", {
      cross: true,
      items: [
        { href: href.memory(), label: t("ui.item.overview"), icon: "memory", match: (r) => r.screen === "memory" },
        { href: href.health("memory"), label: t("ui.item.memory_health"), icon: "health", match: onHealth("memory") },
        { href: href.tests("memory"), label: t("ui.item.memory_tests"), icon: "tests", match: onTests("memory") },
      ],
    }),
    g("tests", "tests", {
      items: [
        { href: href.tests(), label: t("ui.item.overview"), icon: "tests", match: onTests(null) },
        {
          href: href.tests("memory"),
          label: t("ui.group.memory"),
          icon: "memory",
          status: hwtests.dto ? deviceMark(memoryState(hwtests.dto, false).mark) : undefined,
          match: onTests("memory"),
        },
        ...disks.map((d) => ({
          href: href.tests(String(d.number)),
          label: diskName(d),
          icon: (d.media === "Hdd" ? "hdd" : "ssd") as IconName,
          status: hwtests.dto ? deviceMark(diskState(hwtests.dto, d, hwtests.details[d.number] ?? null, false).mark) : undefined,
          match: onTests(String(d.number)),
        })),
      ],
    }),
    g("tweaks", "tweaks", {
      items: (
        [
          ["recommended", "ui.item.recommended", "tweaks"],
          ["quiet", "ui.tweaks.nav.quiet", "info"],
          ["same", "ui.tweaks.nav.same", "shield"],
          ["done", "ui.tweaks.nav.done", "undo"],
          ["all", "ui.tweaks.nav.all", "settings"],
        ] as const
      ).map(([key, label, icon]) => {
        const advice = tweaks.dto?.advice ?? [];
        const history = tweaks.dto?.history ?? [];
        const count = key === "all" ? advice.length : advice.filter((a) => tweakGroupOf(shown(a, history)) === key).length;
        return {
          href: href.tweaks(null, key),
          label: t(label),
          icon,
          meta: tweaks.dto ? String(count) : undefined,
          match: (r: Route) => tweakListOf(r) === key,
        };
      }),
    }),
    g("dumps", "dumps", {
      items: [
        {
          href: href.dumps(),
          label: t("ui.item.overview"),
          icon: "dumps",
          ...lateMark("dumps"),
          meta: dumpBytes ? formatBytes(dumpBytes) : undefined,
          match: (r) => r.screen === "dumps" && dumpListOf(r) === null,
        },
        ...DUMP_GROUPS.map((group) => ({
          href: href.dumps(null, group),
          label: t(`ui.dumps.group.${group}`),
          icon: DUMP_ICON[group],
          meta: dumps.dto ? String(dumpEntries.filter((e) => e.group === group && present(e)).length) : undefined,
          match: (r: Route) => r.screen === "dumps" && dumpListOf(r) === group,
        })),
      ],
    }),
    g("captures", "captures", {
      items: [
        {
          href: href.captures(),
          label: t("ui.shell.nav.captures"),
          icon: "captures",
          ...(helperJobs.unseen("capture") ? lateMark("capture") : liveMark()),
          match: (r) => r.screen === "captures" || r.screen === "capture",
        },
      ],
    }),
    g("settings", "settings", {
      hidden: true,
      items: SETTINGS_PAGES.map((c) => ({
        href: href.settings(c),
        label: t(`ui.settings.category.${c}`),
        icon: SETTINGS_ICONS[c],
        match: (r: Route) => r.screen === "settings" && (r.category ?? "appearance") === c,
      })),
    }),
  ];
}

/** The address of an entry, with the group it is opened from when two groups share it. */
export function itemHref(group: Group, item: NavItem): string {
  return group.cross ? `${item.href}${item.href.includes("?") ? "&" : "?"}g=${group.key}` : item.href;
}

/** Where a card of the home grid leads: the group's first page. */
export function entryHref(group: Group): string {
  return itemHref(group, group.items[0]);
}

const DEFAULT_GROUP: Record<Route["screen"], GroupKey | null> = {
  home: null,
  detail: null,
  activity: "activity",
  history: "activity",
  programs: "now",
  disks: "disks",
  memory: "memory",
  events: "freezes",
  period: "freezes",
  occurrence: "freezes",
  captures: "captures",
  capture: "captures",
  health: "health",
  dumps: "dumps",
  tweaks: "tweaks",
  tests: "tests",
  settings: "settings",
};

/** The route the sidebar follows: a detail page counts as the page it was opened from. */
export function followed(route: Route): Route {
  return route.screen === "detail" && route.origin ? followed(route.origin) : route;
}

/** The group the sidebar shows for a route; `from` is the `g` query, honoured when that group lists the route. */
export function groupOf(groups: Group[], current: Route, from: string | null): Group | null {
  const route = followed(current);
  const asked = from ? groups.find((g) => g.key === from) : undefined;
  if (asked?.items.some((i) => i.match(route))) return asked;
  const fallback =
    route.screen === "activity" && route.tab === "now"
      ? "now"
      : fromHistory(route)
        ? "activity"
        : DEFAULT_GROUP[route.screen];
  return groups.find((g) => g.key === fallback) ?? null;
}
