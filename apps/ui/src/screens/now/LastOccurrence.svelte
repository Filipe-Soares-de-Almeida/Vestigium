<script lang="ts">
  import Block from "../../components/Block.svelte";
  import { type IconName } from "../../components/Icon.svelte";
  import RowCard from "../../components/RowCard.svelte";
  import RowList from "../../components/RowList.svelte";
  import StatusMark, { type StatusKind } from "../../components/StatusMark.svelte";
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

  const STATUS: Record<Severity, StatusKind> = { Light: "info", Medium: "attention", Severe: "critical" };
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

<Block title={t("ui.now.last_occurrence")}>
  {#if latest}
    <RowList>
      <RowCard href={href.occurrence(latest.id)} icon={icon(latest.cause)} title={title(latest)} sub={when(latest.start_ms)}>
        {#snippet right()}<StatusMark kind={STATUS[latest.severity]} reveal />{/snippet}
      </RowCard>
    </RowList>
  {:else}
    <p class="muted">
      {since === null
        ? t("ui.now.last.none")
        : t("ui.now.last.none_since", { time: moment(since) })}
    </p>
  {/if}
</Block>

<style>
  p {
    margin: 0;
  }
</style>
