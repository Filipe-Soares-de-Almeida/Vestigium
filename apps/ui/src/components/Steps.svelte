<script lang="ts">
  import { t } from "../lib/i18n.svelte";
  import { href } from "../lib/router.svelte";

  /** What to do, in order: a numbered list, nothing to tick; a step that names a screen links to it. */
  let { steps }: { steps: { key: string; text: string; href?: string }[] } = $props();

  const PLACE: Record<string, { href: () => string; name: string }> = {
    RunDiskTest: { href: () => href.tests(), name: "ui.shell.nav.tests" },
    RunMemoryDiagnostic: { href: () => href.tests("memory"), name: "ui.shell.nav.tests" },
    FindDiskHeavyApp: { href: () => href.activity("now"), name: "ui.activity.tab.now" },
    SeeNow: { href: () => href.activity("now"), name: "ui.activity.tab.now" },
  };
  const placeOf = (step: { key: string; href?: string }) => {
    const place = PLACE[step.key.split(":")[0]];
    return place ? { href: step.href ?? place.href(), name: t(place.name) } : null;
  };
</script>

<ol class="steps">
  {#each steps as step (step.key)}
    {@const place = placeOf(step)}
    <li>
      <span>
        {step.text}
        {#if place}<a class="go" href={place.href}>{t("ui.steps.open", { place: place.name })}</a>{/if}
      </span>
    </li>
  {/each}
</ol>

<style>
  .steps {
    display: grid;
    gap: 10px;
    margin: 0;
    padding: 0;
    list-style: none;
    counter-reset: step;
  }
  .steps li {
    display: grid;
    grid-template-columns: 22px minmax(0, 1fr);
    gap: 10px;
    align-items: baseline;
    line-height: 1.45;
    counter-increment: step;
  }
  .go {
    margin-left: 4px;
    color: var(--accent);
    font-weight: 600;
    white-space: nowrap;
  }
  .steps li::before {
    content: counter(step) ".";
    color: var(--ink3);
    font-weight: 600;
    font-variant-numeric: tabular-nums;
    text-align: right;
  }
</style>
