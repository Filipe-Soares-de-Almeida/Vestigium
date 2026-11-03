<script lang="ts">
  import { onMount } from "svelte";

  import ConcernMark from "../../components/ConcernMark.svelte";
  import Icon from "../../components/Icon.svelte";
  import { t } from "../../lib/i18n.svelte";
  import { href } from "../../lib/router.svelte";
  import { keyOf, tweaks } from "../../lib/tweaks.svelte";
  import type { DetailLevel, FindingDto, TweakId } from "../../lib/types";
  import { loadDone, saveDone } from "../occurrence/browser";
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
  const pct = (x: number, max: number) => Math.max(0, Math.min(100, (x / max) * 100));

  // The ticks belong to this finding while it stays open; a new one starts with none.
  let done = $state<string[]>([]);

  onMount(() => {
    done = loadDone(finding.id, "finding");
  });

  function tick(key: string, on: boolean): void {
    done = on ? [...done, key] : done.filter((k) => k !== key);
    saveDone(finding.id, done, "finding");
  }
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
  {#if view.ruler}
    {@const r = view.ruler}
    <div
      class="ruler"
      role="img"
      aria-label={t("ui.health.ruler.label", { value: view.value, reference: view.reference })}
    >
      <div class="bar">
        <i
          class="ok"
          style:left="{pct(r.okFrom, r.max)}%"
          style:width="{Math.max(pct(r.okTo, r.max) - pct(r.okFrom, r.max), 1.2)}%"
        ></i>
        <i class="val" style:left="{pct(r.value, r.max)}%"></i>
      </div>
      <div class="lab" aria-hidden="true">
        <span>{t("ui.health.ruler.now")} <b>{view.value}</b></span>
        <span>{t("ui.health.ruler.healthy")} {view.reference}</span>
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
        <div class="todo">
          {#each view.steps as step (step.key)}
            <label class="chk">
              <input
                type="checkbox"
                checked={done.includes(step.key)}
                onchange={(event) => tick(step.key, event.currentTarget.checked)}
              />
              <span>{step.text}</span>
            </label>
          {/each}
        </div>
      {:else}
        <p class="muted none">{t("ui.health.todo.none")}</p>
      {/if}
      {#if tweak}
        <p class="secure"><Icon name="tweaks" /><a href={href.tweaks(keyOf(tweak))}>{t("ui.health.see_tweak")}</a></p>
      {/if}
      {#if test}
        <p class="secure"><Icon name="tests" /><a href={test}>{t(finding.kind === "HardwareErrors" || finding.kind === "MemoryTestErrors" ? "ui.health.see_test.memory" : "ui.health.see_test.disk")}</a></p>
      {/if}
      {#if finding.kind === "NoCrashDump"}
        <p class="secure"><Icon name="dumps" /><a href={href.dumps()}>{t("ui.health.see_dump_setting")}</a></p>
      {/if}
      {#if view.security}
        <p class="secure"><Icon name="shield" /><span>{t("ui.health.secure")}</span></p>
      {/if}
    </div>
  </div>
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
  .bar .ok {
    background: color-mix(in srgb, var(--ink) 22%, transparent);
  }
  .bar .val {
    width: 4px;
    margin-left: -2px;
    border-radius: 3px;
    background: var(--ink);
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
  .todo {
    display: grid;
    gap: 10px;
  }
  .chk {
    display: grid;
    grid-template-columns: 22px minmax(0, 1fr);
    gap: 10px;
    align-items: start;
    line-height: 1.45;
    cursor: pointer;
  }
  .chk input {
    display: grid;
    place-items: center;
    width: 20px;
    height: 20px;
    margin: 1px 0 0;
    border: 1.5px solid var(--ink3);
    border-radius: 6px;
    background: var(--card);
    appearance: none;
    cursor: pointer;
  }
  .chk input:checked {
    border-color: var(--accent);
    background: var(--accent);
  }
  .chk input:checked::after {
    content: "";
    width: 9px;
    height: 5px;
    border-bottom: 2px solid var(--on-accent);
    border-left: 2px solid var(--on-accent);
    transform: rotate(-45deg) translate(1px, -1px);
  }
  .chk input:checked + span {
    color: var(--ink3);
    text-decoration: line-through;
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
    .chk input:checked {
      forced-color-adjust: none;
      background: Highlight;
      border-color: Highlight;
    }
  }
</style>
