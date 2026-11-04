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
  <p class="lab">
    <span>{t("ui.health.ruler.now")} <b>{view.value}</b></span>
    <span>{t("ui.health.ruler.healthy")} {view.reference}</span>
  </p>
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
        <Steps steps={view.steps.map((s) => (s.key === "RunDiskTest" && test ? { ...s, href: test } : s))} />
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
  .lab {
    display: flex;
    flex-wrap: wrap;
    gap: 4px 24px;
    margin: 0;
    color: var(--ink2);
    font-size: 13px;
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
    .bullet {
      forced-color-adjust: none;
      background: CanvasText;
    }
  }
</style>
