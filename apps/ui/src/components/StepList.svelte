<script lang="ts">
  /** What to do, in order: a numbered list with nothing to tick; a step may link to a screen. */
  let { steps }: { steps: { text: string; link?: { label: string; href: string } }[] } = $props();
</script>

<ol class="steps">
  {#each steps as step (step.text)}
    <li>
      <span>{step.text}</span>
      {#if step.link}<a href={step.link.href}>{step.link.label}</a>{/if}
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
  li {
    padding-left: 6px;
  }
  li::marker {
    color: var(--ink2);
  }
  a {
    display: block;
    width: fit-content;
    margin-top: 4px;
    color: var(--accent);
    font-weight: 600;
  }
</style>
