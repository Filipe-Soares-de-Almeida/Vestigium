<script lang="ts">
  import Icon, { type IconName } from "../../components/Icon.svelte";
  import { formatDuration, formatTime } from "../../lib/format";
  import { lang, t } from "../../lib/i18n.svelte";
  import { href } from "../../lib/router.svelte";
  import type { Cause, OccurrenceSummary, Severity } from "../../lib/types";

  let {
    latest,
    since,
  }: {
    latest: OccurrenceSummary | null;
    /** When watching began, on first use; `null` once there is history. */
    since: number | null;
  } = $props();

  const TONE: Record<Severity, string> = { Light: "busy", Medium: "press", Severe: "freeze" };
  const DAY_MS = 86_400_000;

  function icon(cause: Cause): IconName {
    if (cause === "GpuDriver") return "gpu";
    if (cause === "Throttling" || cause === "HardwareFault") return "chip";
    if (cause === "Unresponsive" || cause === "SleepTransition") return "power";
    if (typeof cause === "string") return "app";
    if ("Storage" in cause || "DiskIo" in cause) return "disk";
    if ("Memory" in cause) return "memory";
    if ("Cpu" in cause || "Driver" in cause) return "chip";
    if ("BlueScreen" in cause || "PowerCut" in cause) return "power";
    return "app";
  }

  function title(o: OccurrenceSummary): string {
    const duration = formatDuration(o.end_ms - o.start_ms);
    if (o.kind === "AppHang") {
      const name = typeof o.cause === "object" && "App" in o.cause ? o.cause.App.name : null;
      return name ? t("ui.now.last.AppHang", { name }) : t("ui.common.kind.AppHang");
    }
    return t(`ui.now.last.${o.kind}`, { duration });
  }

  /** "Today at 14:32", "Yesterday at 9:05", or the date. */
  function when(ms: number): string {
    const time = formatTime(ms);
    const midnight = new Date().setHours(0, 0, 0, 0);
    if (ms >= midnight) return t("ui.now.last.today", { time });
    if (ms >= midnight - DAY_MS) return t("ui.now.last.yesterday", { time });
    return t("ui.now.last.date", { date: day(ms), time });
  }

  /** The time alone today, the date and time before. */
  function moment(ms: number): string {
    const midnight = new Date().setHours(0, 0, 0, 0);
    if (ms >= midnight) return formatTime(ms);
    return new Intl.DateTimeFormat(lang(), {
      day: "numeric",
      month: "long",
      hour: "numeric",
      minute: "2-digit",
    }).format(ms);
  }

  function day(ms: number): string {
    return new Intl.DateTimeFormat(lang(), { day: "numeric", month: "long" }).format(ms);
  }
</script>

{#if latest}
  <a class="card link" href={href.occurrence(latest.id)}>
    <span class="icochip" style:background="var(--{TONE[latest.severity]}-t)" style:color="var(--{TONE[latest.severity]})">
      <Icon name={icon(latest.cause)} />
    </span>
    <span class="text">
      <small>{t("ui.now.last_occurrence")}</small>
      <b>{title(latest)}</b>
      <small>{when(latest.start_ms)}</small>
    </span>
    <Icon name="chevron" />
  </a>
{:else}
  <section class="card" aria-labelledby="last-title">
    <h2 id="last-title">{t("ui.now.last_occurrence")}</h2>
    <p class="muted">
      {since === null
        ? t("ui.now.last.none")
        : t("ui.now.last.none_since", { time: moment(since) })}
    </p>
  </section>
{/if}

<style>
  h2 {
    font: 600 15px/1.3 var(--f);
    margin: 0 0 8px;
  }
  p {
    margin: 0;
  }
  .link {
    display: grid;
    grid-template-columns: 40px minmax(0, 1fr) 12px;
    gap: 12px;
    align-items: center;
    color: inherit;
    text-decoration: none;
  }
  .link:hover {
    background: var(--card2);
  }
  .link:focus-visible {
    outline-offset: 2px;
  }
  .text {
    min-width: 0;
  }
  b {
    display: block;
    font-weight: 600;
    line-height: 1.3;
  }
  small {
    display: block;
    color: var(--ink2);
    font-size: 12.5px;
  }
  .link > :global(svg:last-child) {
    color: var(--ink3);
  }
</style>
