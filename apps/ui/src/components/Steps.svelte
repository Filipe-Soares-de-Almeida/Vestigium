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
    gap: 12px;
    max-width: 72ch;
    margin: 0;
    padding: 0 0 0 22px;
    font-size: 14px;
    line-height: 1.5;
    list-style: decimal;
  }
  .steps li {
    padding-left: 6px;
  }
  .steps li::marker {
    color: var(--ink2);
  }
  .go {
    display: block;
    width: fit-content;
    margin-top: 4px;
    color: var(--accent);
    font-weight: 600;
  }
</style>
