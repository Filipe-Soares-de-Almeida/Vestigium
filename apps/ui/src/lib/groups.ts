import type { IconName } from "../components/Icon.svelte";
import { areaState, AREAS } from "../screens/health/areas";
import { formatBytes } from "../screens/occurrence/format";
import { dumps } from "./dumps.svelte";
import { health } from "./health.svelte";
import { t } from "./i18n.svelte";
import { href, type Route } from "./router.svelte";
import type { StatusKind } from "../components/StatusMark.svelte";
import { tweaks } from "./tweaks.svelte";
import { hwtests } from "./tests.svelte";
import type { DiskDto } from "./types";

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

const onActivity = (tab: "today" | "now") => (r: Route) => r.screen === "activity" && r.tab === tab;
const onHistory = (r: Route) => r.screen === "history" || (r.screen === "occurrence" && r.fromDay !== null);
const onHealth = (area: string | null) => (r: Route) => r.screen === "health" && (r.area ?? null) === area;
const onTests = (device: string | null) => (r: Route) => r.screen === "tests" && (r.device ?? null) === device;
const mark = (concern: "Critical" | "Attention" | "Ok" | "Unknown"): StatusKind | undefined =>
  concern === "Critical" ? "critical" : concern === "Attention" ? "attention" : undefined;

/** Builds every group from what the stores hold now; read it inside `$derived` to follow them. */
export function buildGroups(): Group[] {
  const disks = hwtests.dto?.disks ?? [];
  const areas = health.dto ? AREAS.map((a) => areaState(a, health.dto!)) : [];
  const dumpBytes = dumps.dto?.space_bytes ?? 0;
  const g = (key: GroupKey, icon: IconName, extra: Partial<Group> & Pick<Group, "items">): Group => ({
    key,
    icon,
    name: t(`ui.group.${key}`),
    sub: t(`ui.group.${key}.sub`),
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
          href: href.activity(),
          label: t("ui.item.events"),
          icon: "today",
          match: (r) => r.screen === "occurrence" && r.fromDay === null,
        },
      ],
    }),
    g("health", "health", {
      items: [
        { href: href.health(), label: t("ui.item.overview"), icon: "health", match: onHealth(null) },
        ...AREAS.map((a, i) => ({
          href: href.health(a.id),
          label: t(`ui.health.area.${a.id}`),
          icon: a.icon,
          status: areas[i] ? mark(areas[i].mark) : undefined,
          match: onHealth(a.id),
        })),
      ],
    }),
    g("disks", "disk", {
      cross: true,
      items: [
        { href: href.health("disks"), label: t("ui.item.disk_alerts"), icon: "health", match: onHealth("disks") },
        ...disks.map((d) => ({
          href: href.tests(String(d.number)),
          label: diskName(d),
          icon: (d.media === "Hdd" ? "hdd" : "ssd") as IconName,
          match: onTests(String(d.number)),
        })),
      ],
    }),
    g("memory", "memory", {
      cross: true,
      items: [
        { href: href.health("memory"), label: t("ui.item.memory_health"), icon: "health", match: onHealth("memory") },
        { href: href.tests("memory"), label: t("ui.item.memory_tests"), icon: "tests", match: onTests("memory") },
      ],
    }),
    g("tests", "tests", {
      items: [
        { href: href.tests(), label: t("ui.item.overview"), icon: "tests", match: onTests(null) },
        { href: href.tests("memory"), label: t("ui.item.memory_tests"), icon: "memory", match: onTests("memory") },
        ...disks.map((d) => ({
          href: href.tests(String(d.number)),
          label: diskName(d),
          icon: (d.media === "Hdd" ? "hdd" : "ssd") as IconName,
          match: onTests(String(d.number)),
        })),
      ],
    }),
    g("tweaks", "tweaks", {
      items: [
        {
          href: href.tweaks(),
          label: t("ui.item.recommended"),
          icon: "tweaks",
          meta: tweaks.recommended ? String(tweaks.recommended) : undefined,
          match: (r) => r.screen === "tweaks",
        },
      ],
    }),
    g("dumps", "dumps", {
      items: [
        {
          href: href.dumps(),
          label: t("ui.item.overview"),
          icon: "dumps",
          meta: dumpBytes ? formatBytes(dumpBytes) : undefined,
          match: (r) => r.screen === "dumps",
        },
      ],
    }),
    g("captures", "captures", {
      items: [
        {
          href: href.captures(),
          label: t("ui.shell.nav.captures"),
          icon: "captures",
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
  activity: "activity",
  history: "activity",
  programs: "now",
  occurrence: "freezes",
  captures: "captures",
  capture: "captures",
  health: "health",
  dumps: "dumps",
  tweaks: "tweaks",
  tests: "tests",
  settings: "settings",
};

/** The group the sidebar shows for a route; `from` is the `g` query, honoured when that group lists the route. */
export function groupOf(groups: Group[], route: Route, from: string | null): Group | null {
  const asked = from ? groups.find((g) => g.key === from) : undefined;
  if (asked?.items.some((i) => i.match(route))) return asked;
  const fallback =
    route.screen === "activity" && route.tab === "now"
      ? "now"
      : route.screen === "occurrence" && route.fromDay !== null
        ? "activity"
        : DEFAULT_GROUP[route.screen];
  return groups.find((g) => g.key === fallback) ?? null;
}
