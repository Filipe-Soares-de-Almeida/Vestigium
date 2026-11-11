import { formatDuration, formatNumber, formatPercent, formatTime } from "../../lib/format";
import { t, tn } from "../../lib/i18n.svelte";
import type {
  BootDelay,
  Concern,
  DetailLevel,
  FindingDto,
  FindingKind,
  HealthDto,
  HealthFact,
  Measure,
  NextStep,
  Reference,
  Subject,
} from "../../lib/types";
import { formatBytes, formatDayShort, formatMs, sameDay } from "../occurrence/format";
import { lineFor, type Line, type Texts } from "../occurrence/report";
import type { Check } from "./areas";

// The words of the health page, built from typed findings at the reader's detail level.

type Args = Record<string, string | number>;

const k = (key: string, args?: Args) => t(`ui.health.${key}`, args);
const kn = (key: string, count: number, args?: Args) => tn(`ui.health.${key}`, count, args);

/** The text of `key.<variant>` when the catalog has one, else of `key`. */
function byConcern(key: string, concern: Concern | "Hot", args?: Args): string {
  const specific = `ui.health.${key}.${concern}`;
  const text = t(specific, args);
  return text === specific ? k(key, args) : text;
}

export interface FindingView {
  title: string;
  since: string;
  why: string;
  value: string;
  reference: string;
  evidence: Line[];
  steps: { key: string; text: string }[];
  /** A security product slowed the boot: the page says it is never offered for removal. */
  security: boolean;
}

/** "D:" for the disk "1 D:", "C:" for the volume "C:\", nothing for the whole PC. */
export function subjectName(subject: Subject): string {
  if (subject === "Pc") return "";
  if ("Disk" in subject) {
    const match = /^(\d+)\s*(.*)$/.exec(subject.Disk.trim());
    return match ? match[2] || k("disk_n", { n: match[1] }) : subject.Disk;
  }
  return subject.Volume.replace(/\\$/, "");
}

/** "today, 19:40" or "Nov 2". */
export function when(ms: number): string {
  return sameDay(ms, Date.now()) ? k("today_at", { time: formatTime(ms) }) : formatDayShort(ms);
}

/** An NVMe disk that only ran hot: its one next step is the cooling check. */
function ranHot(f: FindingDto): boolean {
  return (
    f.kind === "DiskSmart" &&
    f.concern === "Attention" &&
    f.next_steps.length === 1 &&
    f.next_steps[0] === "CheckCooling"
  );
}

export function findingTitle(f: FindingDto, level: DetailLevel): string {
  const subject = subjectName(f.subject);
  return level === "Technical"
    ? k(`kind.${f.kind}.tech`, { subject })
    : byConcern(`kind.${f.kind}.title`, ranHot(f) ? "Hot" : f.concern, { subject });
}

export function findingView(f: FindingDto, level: DetailLevel): FindingView {
  const subject = subjectName(f.subject);
  return {
    title: findingTitle(f, level),
    since: k("since", { when: when(f.opened_ms) }),
    why:
      level === "Technical"
        ? k(`kind.${f.kind}.rule`)
        : byConcern(`kind.${f.kind}.why`, ranHot(f) ? "Hot" : f.concern),
    value: valueText(f.kind, f.value),
    reference: referenceText(f.kind, f.reference),
    evidence: evidenceTexts(f).map((texts) => lineFor(level, texts)),
    steps: f.next_steps.map((step) => ({ key: step, text: stepText(step, subject) })),
    security: f.evidence.some((fact) => typeof fact === "object" && "Delay" in fact && fact.Delay.security),
  };
}

export function stepText(step: NextStep, subject: string): string {
  const key = `ui.occurrence.step.${step}`;
  if (!subject) {
    const any = t(`${key}.any`);
    if (any !== `${key}.any`) return any;
  }
  return t(key, { disk: subject });
}

/** "24 damaged areas", "7.4% free", "1 min 48 s". */
export function valueText(kind: FindingKind, value: Measure): string {
  // pt-BR reads zero as singular, so a zero has its own sentence.
  if ("Count" in value) {
    return value.Count === 0 ? k(`kind.${kind}.count.zero`) : kn(`kind.${kind}.count`, value.Count);
  }
  if ("Percent" in value) return k(`kind.${kind}.percent`, { value: formatPercent(value.Percent, 1) });
  if ("Days" in value) return kn(`kind.${kind}.days`, value.Days);
  return plainMeasure(value);
}

/** What a healthy PC shows, named in the finding's own words: "no damaged area", "up to 60 s". */
function referenceText(kind: FindingKind, reference: Reference): string {
  if ("AtMost" in reference) {
    const limit = reference.AtMost;
    if ("Count" in limit && limit.Count === 0) return valueText(kind, limit);
    return k("ref.at_most", { value: "Count" in limit ? valueText(kind, limit) : plainMeasure(limit) });
  }
  if ("AtLeast" in reference) {
    const limit = reference.AtLeast;
    return k("ref.at_least", { value: "Count" in limit ? valueText(kind, limit) : plainMeasure(limit) });
  }
  return plainMeasure(reference.Is);
}

function plainMeasure(m: Measure): string {
  if ("Count" in m) return formatNumber(m.Count);
  if ("Percent" in m) return formatPercent(m.Percent, m.Percent % 1 ? 1 : 0);
  if ("Days" in m) return kn("days", m.Days);
  if ("Seconds" in m) return formatDuration(m.Seconds * 1000);
  if ("Bytes" in m) return formatBytes(m.Bytes);
  return k(`media.${m.Media}`);
}

type EventFact = Extract<HealthFact, { Event: unknown }>["Event"];
type OtherFact = Exclude<HealthFact, { Event: unknown }>;

/** The facts of a finding; events with the same meaning become one line with their count and times. */
function evidenceTexts(f: FindingDto): Texts[] {
  const items: (HealthFact | EventFact[])[] = [];
  for (const fact of f.evidence) {
    if (typeof fact !== "object" || !("Event" in fact)) {
      items.push(fact);
      continue;
    }
    const group = items.find((x): x is EventFact[] => Array.isArray(x) && x[0].meaning === fact.Event.meaning);
    if (group) group.push(fact.Event);
    else items.push([fact.Event]);
  }
  return items.map((x) => (Array.isArray(x) ? eventTexts(f.kind, x) : factTexts(x as OtherFact)));
}

function eventTexts(kind: FindingKind, events: EventFact[]): Texts {
  const e = events[0];
  // A power loss during shutdown is not a fall while the PC is in use.
  const what =
    kind === "ShutdownUnfinished" && e.meaning === "PowerLoss"
      ? k("ev.shutdown_power_loss")
      : t(`ui.occurrence.event.${e.meaning}`);
  const stamp = (ms: number) => (sameDay(ms, Date.now()) ? when(ms) : `${when(ms)}, ${formatTime(ms)}`);
  return {
    plain: events.length > 1 ? kn("ev.times", events.length, { what }) : what,
    figures: events.map((ev) => stamp(ev.ts_ms)).join(" · "),
    tech: events.map((ev) => `${ev.provider} ${ev.id} · ${stamp(ev.ts_ms)}`).join("; "),
  };
}

function factTexts(fact: OtherFact): Texts {
  if (fact === "PredictFailure") return { plain: k("ev.predict"), figures: null, tech: "PredictFailure = 1" };
  if (fact === "NoPageFile") return { plain: k("ev.no_page_file"), figures: null, tech: null };
  if ("Attribute" in fact) {
    const a = fact.Attribute;
    const known = [5, 187, 188, 197, 198].includes(a.id);
    return {
      plain: known ? kn(`ev.attr.${a.id}`, a.raw) : k("ev.attr.other", { id: a.id, raw: formatNumber(a.raw) }),
      figures: k("ev.attr.figures", { id: a.id }),
      tech: k("ev.attr.tech", { id: a.id, raw: formatNumber(a.raw), current: a.current, worst: a.worst }),
    };
  }
  if ("Nvme" in fact) {
    const n = fact.Nvme;
    return {
      plain: k("ev.nvme", { used: formatPercent(n.percentage_used), spare: formatPercent(n.available_spare_pct) }),
      figures: null,
      tech: k("ev.nvme.tech", {
        warning: `0x${n.critical_warning.toString(16).padStart(2, "0")}`,
        spare: formatPercent(n.available_spare_pct),
        threshold: formatPercent(n.spare_threshold_pct),
        used: formatPercent(n.percentage_used),
        errors: formatNumber(n.media_errors),
        temp: n.temperature_c,
      }),
    };
  }
  if ("Space" in fact) {
    const s = fact.Space;
    return {
      plain: k("ev.space", { free: formatBytes(s.free_bytes), total: formatBytes(s.total_bytes) }),
      figures: null,
      tech: null,
    };
  }
  if ("PageFile" in fact) {
    const p = fact.PageFile;
    return {
      plain: k("ev.page_file", { path: p.path, size: formatBytes(p.size_bytes), peak: formatBytes(p.peak_bytes) }),
      figures: null,
      tech: null,
    };
  }
  if ("CommitPeak" in fact) {
    const pct = formatPercent(fact.CommitPeak.pct);
    return { plain: k("ev.commit", { pct }), figures: null, tech: k("ev.commit.tech", { pct }) };
  }
  if ("MemoryHighDays" in fact) {
    return { plain: kn("ev.memory_days", fact.MemoryHighDays.days), figures: null, tech: null };
  }
  if ("Product" in fact) return { plain: k("ev.product", { name: fact.Product.name }), figures: null, tech: null };
  if ("Boot" in fact) {
    const b = fact.Boot;
    return {
      plain: k("ev.boot", { total: formatMs(b.boot_ms) }),
      figures: k("ev.boot.figures", { main: formatMs(b.main_path_ms), post: formatMs(b.post_boot_ms) }),
      tech: k("ev.boot.tech", {
        total: formatNumber(b.boot_ms),
        main: formatNumber(b.main_path_ms),
        post: formatNumber(b.post_boot_ms),
      }),
    };
  }
  if ("Delay" in fact) return delayTexts(fact.Delay);
  if ("Failures" in fact) {
    const f = fact.Failures;
    return {
      plain: kn("ev.failures", f.last_week, { before: formatNumber(f.weekly_before, f.weekly_before % 1 ? 1 : 0) }),
      figures: null,
      tech: null,
    };
  }
  if ("MemoryDiagnostic" in fact) {
    const m = fact.MemoryDiagnostic;
    const date = when(m.ts_ms);
    return {
      plain: m.bad_pages ? kn("ev.memdiag.pages", m.bad_pages, { date }) : k("ev.memdiag", { date }),
      figures: null,
      tech: k("ev.memdiag.tech", { pages: formatNumber(m.bad_pages ?? 0) }),
    };
  }
  if ("MemoryQuickTest" in fact) {
    const q = fact.MemoryQuickTest;
    return {
      plain: kn("ev.memquick", q.errors, { tested: formatBytes(q.tested_bytes), date: when(q.ts_ms) }),
      figures: null,
      tech: null,
    };
  }
  if ("SurfaceScan" in fact) {
    const v = fact.SurfaceScan;
    const date = when(v.ts_ms);
    return {
      plain:
        v.unreadable_blocks > 0
          ? kn("ev.surface.unreadable", v.unreadable_blocks, { date })
          : kn("ev.surface.slow", v.slow_blocks, { date }),
      figures: v.whole_disk ? null : k("ev.surface.partial"),
      tech: k("ev.surface.tech", { unreadable: formatNumber(v.unreadable_blocks), slow: formatNumber(v.slow_blocks) }),
    };
  }
  if ("VolumeCheck" in fact) {
    return { plain: k("ev.volume", { date: when(fact.VolumeCheck.ts_ms) }), figures: null, tech: "chkdsk /scan /forceofflinefix" };
  }
  const d = fact.SystemDisk;
  return {
    plain: k(`ev.system_disk.${d.media}`, { disk: subjectName({ Disk: d.disk }) }),
    figures: null,
    tech: k("ev.system_disk.tech", { disk: d.disk, media: d.media }),
  };
}

function delayTexts(d: BootDelay): Texts {
  const extra = formatMs(d.degradation_ms);
  return {
    plain: k(`ev.delay.${d.kind}`, { name: d.name, extra }),
    figures: d.security ? k("ev.delay.security") : k("ev.delay.figures", { total: formatMs(d.total_ms), extra }),
    tech: k("ev.delay.tech", {
      kind: d.kind,
      name: d.name,
      total: formatNumber(d.total_ms),
      extra: formatNumber(d.degradation_ms),
    }),
  };
}

/** The line of a kind that has no open finding. */
export function checkText(check: Check): string {
  return check.mark === "Unknown" ? k(`check.${check.kind}.Unknown`) : k(`check.${check.kind}`);
}

/** One part of the page's opening sentence; bold parts carry the news. */
export interface Part {
  text: string;
  bold: boolean;
}

export function lede(dto: HealthDto, level: DetailLevel, checks: number): Part[] {
  const critical = dto.open.filter((f) => f.concern === "Critical");
  const attention = dto.open.length - critical.length;
  if (level === "Technical") {
    return [{ text: k("lede.tech", { critical: critical.length, attention, ok: checks }), bold: false }];
  }
  if (critical.length) {
    const rest = dto.open.length - 1;
    const parts: Part[] = [{ text: `${findingTitle(critical[0], level)}.`, bold: true }];
    if (rest) parts.push({ text: ` ${kn("lede.also", rest)}`, bold: false });
    return parts;
  }
  if (attention) {
    return [
      { text: `${k("lede.calm")} `, bold: false },
      { text: kn("lede.attention", attention), bold: true },
      { text: k("lede.rest"), bold: false },
    ];
  }
  if (dto.last_scan_ms === null) {
    return [
      { text: k("lede.unknown"), bold: true },
      { text: ` ${k("lede.unknown.rest")}`, bold: false },
    ];
  }
  return [
    { text: k("lede.ok"), bold: true },
    { text: ` ${k("lede.ok.rest")}`, bold: false },
  ];
}
