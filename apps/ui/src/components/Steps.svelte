<script lang="ts">
  /** What to do, in order: a numbered list, nothing to tick. */
  let { steps }: { steps: { key: string; text: string }[] } = $props();
</script>

<ol class="steps">
  {#each steps as step (step.key)}
    <li>{step.text}</li>
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
  .steps li::before {
    content: counter(step) ".";
    color: var(--ink3);
    font-weight: 600;
    font-variant-numeric: tabular-nums;
    text-align: right;
  }
</style>
