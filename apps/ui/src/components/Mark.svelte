<script lang="ts">
  // The Lapidado: four facets of a cut V, filled from the colour theme's own ramp.
  let { size = 22 }: { size?: number } = $props();
</script>

<svg class="mark" width={size} height={size} viewBox="0 0 16 16" aria-hidden="true" focusable="false">
  <path class="f1" d="M0.46 1.47L2.67 1.47L8 14.73Z" />
  <path class="f2" d="M2.67 1.47L4.88 1.47L8 7.45L8 14.73Z" />
  <path class="f3" d="M11.12 1.47L13.33 1.47L8 14.73L8 7.45Z" />
  <path class="f4" d="M13.33 1.47L15.54 1.47L8 14.73Z" />
</svg>

<style>
  .mark {
    display: block;
    flex: none;
  }
  path {
    stroke-width: 0.3;
    stroke-linejoin: round;
  }
  .f1 {
    fill: var(--mark-1);
    stroke: var(--mark-1);
  }
  .f2 {
    fill: var(--mark-2);
    stroke: var(--mark-2);
  }
  .f3 {
    fill: var(--mark-3);
    stroke: var(--mark-3);
  }
  .f4 {
    fill: var(--mark-4);
    stroke: var(--mark-4);
  }
</style>
