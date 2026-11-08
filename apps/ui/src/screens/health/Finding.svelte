<script lang="ts">
  import StatusMark from "../../components/StatusMark.svelte";
  import Icon from "../../components/Icon.svelte";
  import Steps from "../../components/Steps.svelte";
  import { formatDayShort } from "../occurrence/format";
  import { t, tn } from "../../lib/i18n.svelte";
  import { href } from "../../lib/router.svelte";
  import { hwtests } from "../../lib/tests.svelte";
  import { keyOf, tweaks } from "../../lib/tweaks.svelte";
  import type { DetailLevel, FindingDto, TweakId } from "../../lib/types";
  import DiskMap from "../tests/DiskMap.svelte";
  import { diskSurface } from "../tests/words";
  import { diskName } from "../../lib/groups";
  import { findingView, subjectName, valueText } from "./words";

  let { finding, level, first = false }: { finding: FindingDto; level: DetailLevel; first?: boolean } = $props();

  const view = $derived(findingView(finding, level));
  /** The tweak that answers each kind of point, where one does. */
  const TWEAK_OF: Partial<Record<FindingDto["kind"], TweakId>> = {
    WindowsOnHdd: "SysMain",
    LowSpace: "Hibernation",
    ShutdownUnfinished: "FastStartup",
    DisplayResets: "GpuScheduling",
    SlowBoot: "StartupApp",
  };
  /** The tweak the optimizations page recommends for this point, when it has a signal for it. */
  const tweak = $derived.by(() => {
    const wanted = TWEAK_OF[finding.kind];
    return tweaks.dto?.advice.find((a) => a.tweak === wanted && a.status === "Ready" && a.signals.length > 0) ?? null;
  });
  /** Where on the Tests page a point can be checked: the memory, a disk by number, or the page. */
  const test = $derived.by((): string | null => {
    if (["HardwareErrors", "MemoryTestErrors"].includes(finding.kind)) return href.tests("memory");
    if (!["DiskSmart", "StorageErrors", "DiskUnreadable", "VolumeErrors"].includes(finding.kind)) return null;
    const s = finding.subject;
    const number = typeof s === "object" && "Disk" in s ? s.Disk.split(" ")[0] : null;
    return href.tests(number && /^\d+$/.test(number) ? number : undefined);
  });
  const DAY_MS = 86_400_000;
  const midnight = (ms: number) => new Date(ms).setHours(0, 0, 0, 0);
  /** What the point measured when it opened, to compare with now; none while it opened today. */
  const then = $derived.by(() => {
    const days = Math.round((midnight(Date.now()) - midnight(finding.opened_ms)) / DAY_MS);
    if (!finding.first_value || days < 1) return null;
    return {
      label: tn("ui.health.then", days, { date: formatDayShort(finding.opened_ms) }),
      value: valueText(finding.kind, finding.first_value),
    };
  });
  /** The last read of the whole disk this point is about, when one covered all of it. */
  const map = $derived.by(() => {
    const s = finding.subject;
    const number = typeof s === "object" && "Disk" in s ? Number(s.Disk.split(" ")[0]) : NaN;
    const disk = hwtests.dto?.disks.find((d) => d.number === number);
    const scan = disk && hwtests.dto ? diskSurface(hwtests.dto, disk) : null;
    if (!scan || scan.from_bytes !== 0 || scan.to_bytes < scan.disk_bytes) return null;
    const letter = subjectName(finding.subject);
    return { scan, name: disk!.name, title: letter ? `${diskName(disk!)} (${letter})` : diskName(disk!) };
  });
  /** Steps already link to Tests, so a test needs no button of its own. */
  const linked = $derived(view.steps.some((s) => s.key === "RunDiskTest" || s.key === "RunMemoryDiagnostic"));
  /** Where the point can be acted on elsewhere, when no step already leads there. */
  const actions = $derived.by(() => {
    const out: { key: string; label: string; href: string; icon: "tweaks" | "tests" | "dumps" }[] = [];
    if (tweak) out.push({ key: "tweak", label: t("ui.health.see_tweak"), href: href.tweaks(keyOf(tweak)), icon: "tweaks" });
    if (test && !linked) {
      const memory = finding.kind === "HardwareErrors" || finding.kind === "MemoryTestErrors";
      out.push({ key: "test", label: t(memory ? "ui.health.see_test.memory" : "ui.health.see_test.disk"), href: test, icon: "tests" });
    }
    if (finding.kind === "NoCrashDump") out.push({ key: "dump", label: t("ui.health.see_dump_setting"), href: href.dumps(), icon: "dumps" });
    return out;
  });
</script>

<section class="finding" class:first aria-labelledby="finding-{finding.id}">
  <header class="sec-h">
    <h2 id="finding-{finding.id}">{view.title}</h2>
  </header>
  <div class="grid" class:map={map !== null}>
    <div class="text">
      <p class="state">
        <StatusMark kind={finding.concern === "Critical" ? "critical" : "attention"} />
        <span>{view.since}</span>
      </p>
      <p class="p">{view.why}</p>
      <p class="trend">
        <span><b>{t("ui.health.ruler.now")}</b> {view.value}</span>
        {#if then}<span class="sep" aria-hidden="true">·</span><span><b>{then.label}</b> {then.value}</span>{/if}
      </p>
      <h3 class="h3">{t("ui.health.evidence")}</h3>
      <ul class="ev-l">
        {#each view.evidence as line, i (i)}
          <li>{line.main}{#if line.sub}<small>{line.sub}</small>{/if}</li>
        {/each}
      </ul>
      <h3 class="h3">{t("ui.occurrence.todo.title")}</h3>
      {#if view.steps.length}
        <Steps steps={view.steps.map((s) => (s.key === "RunDiskTest" && test ? { ...s, href: test } : s))} />
      {:else}
        <p class="empty">{t("ui.health.todo.none")}</p>
      {/if}
      {#if view.security}
        <p class="note secure"><Icon name="shield" /><span>{t("ui.health.secure")}</span></p>
      {/if}
      {#each actions as action (action.key)}
        <p class="more"><a href={action.href}>{action.label}</a></p>
      {/each}
    </div>
    {#if map}
      <div class="side">
        <h3 class="h3">{t("ui.health.surface", { name: map.title })}</h3>
        <DiskMap scan={map.scan} diskBytes={map.scan.disk_bytes} name={map.name} {level} />
      </div>
    {/if}
  </div>
</section>

<style>
  .finding {
    min-width: 0;
    margin-top: 32px;
    padding-top: 28px;
    border-top: 1px solid var(--rule);
  }
  .finding.first {
    margin-top: 24px;
    padding-top: 0;
    border-top: 0;
  }
  .sec-h {
    display: flex;
    align-items: center;
    min-height: 30px;
    margin-bottom: 12px;
  }
  h2 {
    margin: 0;
  }
  .grid {
    display: grid;
    gap: 24px 48px;
  }
  @container content (min-width: 900px) {
    .grid.map {
      grid-template-columns: minmax(0, 1fr) minmax(320px, 440px);
    }
  }
  .text {
    min-width: 0;
  }
  .side .h3 {
    margin-top: 0;
  }
  .state {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px 14px;
    margin: 0 0 10px;
    color: var(--ink2);
    font-size: 13px;
  }
  .trend {
    display: inline-flex;
    flex-wrap: wrap;
    gap: 4px 10px;
    margin: 2px 0 4px;
    padding: 8px 14px;
    border-radius: 6px;
    background: var(--hover);
    font-size: 14px;
  }
  .trend b {
    font-weight: 600;
  }
  .sep {
    color: var(--ink3);
  }
  .h3 {
    margin: 20px 0 8px;
  }
  .ev-l small {
    display: block;
    color: var(--ink2);
    font-size: 13px;
  }
  .secure {
    display: flex;
    gap: 10px;
    align-items: flex-start;
  }
  .secure :global(svg) {
    flex: none;
    width: 16px;
    height: 16px;
    margin-top: 1px;
  }
</style>
