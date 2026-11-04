<script lang="ts">
  import ConcernMark from "../../components/ConcernMark.svelte";
  import Icon from "../../components/Icon.svelte";
  import Steps from "../../components/Steps.svelte";
  import { t } from "../../lib/i18n.svelte";
  import { href } from "../../lib/router.svelte";
  import { keyOf, tweaks } from "../../lib/tweaks.svelte";
  import type { DetailLevel, FindingDto, TweakId } from "../../lib/types";
  import { areaOf } from "./areas";
  import { findingView } from "./words";

  let { finding, level }: { finding: FindingDto; level: DetailLevel } = $props();

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
  const icon = $derived(areaOf(finding.kind).icon);
  const tone = $derived(finding.concern === "Critical" ? "freeze" : "press");
  const clamp = (x: number) => Math.max(0, Math.min(100, x));
  /** The healthy stretch never takes less of the bar than this, in percent, so it can be seen. */
  const MIN_ZONE = 12;
  /** The bar in two named stretches, healthy and not, and where the value sits on it. */
  const scale = $derived.by(() => {
    const r = view.ruler;
    if (!r) return null;
    const raw = (x: number) => (x / r.max) * 100;
    const span = (from: number, to: number) => Math.max(1e-9, to - from);
    if (r.okFrom === 0) {
      const end = Math.min(Math.max(raw(r.okTo), MIN_ZONE), 100 - MIN_ZONE);
      const at =
        r.value <= r.okTo
          ? (r.value / span(0, r.okTo)) * end
          : end + ((r.value - r.okTo) / span(r.okTo, r.max)) * (100 - end);
      return { healthyFirst: true, cut: end, marker: clamp(at) };
    }
    const start = Math.max(Math.min(raw(r.okFrom), 100 - MIN_ZONE), MIN_ZONE);
    const at =
      r.value >= r.okFrom
        ? start + ((r.value - r.okFrom) / span(r.okFrom, r.max)) * (100 - start)
        : (r.value / span(0, r.okFrom)) * start;
    return { healthyFirst: false, cut: start, marker: clamp(at) };
  });
  const outsideWord = $derived(
    finding.concern === "Attention" || finding.concern === "Critical"
      ? t(`ui.health.mark.${finding.concern}`)
      : t("ui.health.ruler.outside"),
  );
  /** Where the point can be acted on elsewhere, each with a line on what that screen does. */
  const actions = $derived.by(() => {
    const out: { key: string; hint: string; label: string; href: string; icon: "tweaks" | "tests" | "dumps" }[] = [];
    if (tweak) {
      out.push({ key: "tweak", hint: t("ui.health.foot.tweak"), label: t("ui.health.see_tweak"), href: href.tweaks(keyOf(tweak)), icon: "tweaks" });
    }
    if (test) {
      const memory = finding.kind === "HardwareErrors" || finding.kind === "MemoryTestErrors";
      out.push({
        key: "test",
        hint: t(memory ? "ui.health.foot.test.memory" : "ui.health.foot.test.disk"),
        label: t(memory ? "ui.health.see_test.memory" : "ui.health.see_test.disk"),
        href: test,
        icon: "tests",
      });
    }
    if (finding.kind === "NoCrashDump") {
      out.push({ key: "dump", hint: t("ui.health.foot.dump"), label: t("ui.health.see_dump_setting"), href: href.dumps(), icon: "dumps" });
    }
    return out;
  });
</script>

<section class="card finding" aria-labelledby="finding-{finding.id}">
  <div class="head">
    <span class="icochip" style:background="var(--{tone}-t)" style:color="var(--{tone})"><Icon name={icon} /></span>
    <div>
      <h2 id="finding-{finding.id}">{view.title}</h2>
      <p class="meta"><ConcernMark mark={finding.concern} /><span>{view.since}</span></p>
    </div>
  </div>
  <p class="why">{view.why}</p>
  {#if scale}
    {@const healthy = scale.healthyFirst ? { left: 0, width: scale.cut } : { left: scale.cut, width: 100 - scale.cut }}
    {@const outside = scale.healthyFirst ? { left: scale.cut, width: 100 - scale.cut } : { left: 0, width: scale.cut }}
    <div
      class="ruler"
      role="img"
      aria-label={t("ui.health.ruler.label", { value: view.value, reference: view.reference })}
    >
      <p class="lab now" aria-hidden="true">{t("ui.health.ruler.now")} <b>{view.value}</b></p>
      <div class="bar" aria-hidden="true">
        <i class="zone ok" class:first={healthy.left === 0} style:left="{healthy.left}%" style:width="{healthy.width}%"></i>
        <i class="zone out {tone}" class:first={outside.left === 0} style:left="{outside.left}%" style:width="{outside.width}%"></i>
        <i class="val" style:left="{scale.marker}%"></i>
      </div>
      <div class="zones" aria-hidden="true" style:grid-template-columns={scale.healthyFirst
          ? `minmax(max-content, ${healthy.width}fr) ${outside.width}fr`
          : `${outside.width}fr minmax(max-content, ${healthy.width}fr)`}>
        {#if scale.healthyFirst}
          <span>{t("ui.health.ruler.healthy")} {view.reference}</span>
          <span>{outsideWord}</span>
        {:else}
          <span>{outsideWord}</span>
          <span class="end">{t("ui.health.ruler.healthy")} {view.reference}</span>
        {/if}
      </div>
    </div>
  {:else}
    <p class="lab plain">
      <span>{t("ui.health.ruler.now")} <b>{view.value}</b></span>
      <span>{t("ui.health.ruler.healthy")} {view.reference}</span>
    </p>
  {/if}
  <div class="two">
    <div>
      <h3>{t("ui.health.evidence")}</h3>
      <ul class="ev">
        {#each view.evidence as line, i (i)}
          <li>
            <span class="bullet" class:first={i === 0} aria-hidden="true"></span>
            <span>{line.main}{#if line.sub}<small>{line.sub}</small>{/if}</span>
          </li>
        {/each}
      </ul>
    </div>
    <div>
      <h3>{t("ui.occurrence.todo.title")}</h3>
      {#if view.steps.length}
        <Steps steps={view.steps} />
      {:else}
        <p class="muted none">{t("ui.health.todo.none")}</p>
      {/if}
      {#if view.security}
        <p class="secure"><Icon name="shield" /><span>{t("ui.health.secure")}</span></p>
      {/if}
    </div>
  </div>
  {#if actions.length}
    <div class="foot">
      {#each actions as action (action.key)}
        <div class="act">
          <p>{action.hint}</p>
          <a class="btn" href={action.href}><Icon name={action.icon} />{action.label}</a>
        </div>
      {/each}
    </div>
  {/if}
</section>

<style>
  .finding {
    display: grid;
    gap: 14px;
    margin-top: 14px;
  }
  .head {
    display: grid;
    grid-template-columns: 40px minmax(0, 1fr);
    gap: 14px;
    align-items: center;
  }
  h2 {
    margin: 0;
    font: 600 18px/1.3 var(--fd);
  }
  .meta {
    display: flex;
    flex-wrap: wrap;
    gap: 4px 12px;
    align-items: center;
    margin: 4px 0 0;
    color: var(--ink2);
    font-size: 13px;
  }
  .why {
    max-width: 75ch;
    margin: 0;
    font-size: 15px;
    line-height: 1.55;
  }
  .ruler {
    display: grid;
    gap: 6px;
  }
  .bar {
    position: relative;
    height: 10px;
    overflow: hidden;
    border: 1px solid var(--stroke2);
    border-radius: 999px;
    background: var(--card2);
  }
  .bar i {
    position: absolute;
    top: 0;
    bottom: 0;
  }
  .bar {
    overflow: visible;
  }
  .bar .zone {
    border-radius: 0 999px 999px 0;
  }
  .bar .zone.first {
    border-radius: 999px 0 0 999px;
  }
  .bar .zone.ok {
    background: color-mix(in srgb, var(--ink3) 35%, transparent);
  }
  .bar .zone.out.press {
    background: color-mix(in srgb, var(--press) 45%, transparent);
  }
  .bar .zone.out.freeze {
    background: color-mix(in srgb, var(--freeze) 65%, transparent);
  }
  .bar .val {
    top: -4px;
    bottom: -4px;
    width: 4px;
    margin-left: -2px;
    border-radius: 2px;
    background: var(--ink);
    box-shadow: 0 0 0 2px var(--card);
  }
  .zones {
    display: grid;
    color: var(--ink2);
    font-size: 12.5px;
  }
  .zones span {
    white-space: nowrap;
  }
  .zones span + span {
    padding-left: 12px;
  }
  .zones .end {
    text-align: right;
  }
  .lab.now {
    display: block;
    margin: 0;
  }
  .lab {
    display: flex;
    flex-wrap: wrap;
    justify-content: space-between;
    gap: 4px 12px;
    color: var(--ink2);
    font-size: 13px;
  }
  .lab.plain {
    justify-content: start;
    gap: 4px 24px;
    margin: 0;
  }
  .lab b {
    color: var(--ink);
    font-weight: 600;
  }
  .two {
    display: grid;
    grid-template-columns: minmax(0, 1.2fr) minmax(0, 1fr);
    gap: 18px;
  }
  h3 {
    margin: 0 0 8px;
    color: var(--ink2);
    font: 600 13px/1.35 var(--f);
  }
  .ev {
    display: grid;
    gap: 8px;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .ev li {
    display: grid;
    grid-template-columns: 18px minmax(0, 1fr);
    gap: 8px;
    line-height: 1.45;
    overflow-wrap: anywhere;
  }
  .ev small {
    display: block;
    color: var(--ink2);
    font-size: 13px;
  }
  .bullet {
    width: 6px;
    height: 6px;
    margin: 0.5em 0 0 6px;
    border-radius: 50%;
    background: var(--ink3);
  }
  .bullet.first {
    background: var(--accent);
  }
  .none {
    margin: 0;
  }
  .secure {
    display: flex;
    gap: 8px;
    align-items: flex-start;
    margin: 12px 0 0;
    padding: 8px 10px;
    border-radius: 10px;
    background: var(--card2);
    color: var(--ink2);
    font-size: 13px;
    line-height: 1.45;
  }
  .secure :global(svg) {
    flex: none;
    width: 16px;
    height: 16px;
  }
  .foot {
    display: grid;
    gap: 10px;
    margin: 0 -18px -16px;
    padding: 12px 18px;
    border-top: 1px solid var(--stroke2);
    border-radius: 0 0 14px 14px;
    background: color-mix(in srgb, var(--card2) 55%, transparent);
  }
  .act {
    display: flex;
    flex-wrap: wrap;
    gap: 10px 16px;
    align-items: center;
    justify-content: space-between;
  }
  .act p {
    margin: 0;
    color: var(--ink2);
    font-size: 13.5px;
    line-height: 1.45;
  }
  @container content (max-width: 720px) {
    .two {
      grid-template-columns: minmax(0, 1fr);
    }
  }
  @media (forced-colors: active) {
    .bar .ok,
    .bar .val,
    .bullet {
      forced-color-adjust: none;
    }
    .bar .ok {
      background: GrayText;
    }
    .bar .val,
    .bullet {
      background: CanvasText;
    }
  }
</style>
