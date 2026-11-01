<script lang="ts">
  // The 16 px Lapidado: four facets of a cut V, filled from the colour theme's own ramp.
</script>

<svg class="mark" width="16" height="16" viewBox="0 0 16 16" aria-hidden="true" focusable="false">
  <path class="f1" d="M2.2 3L3.9 3L8 13.2Z" />
  <path class="f2" d="M3.9 3L5.6 3L8 7.6L8 13.2Z" />
  <path class="f3" d="M10.4 3L12.1 3L8 13.2L8 7.6Z" />
  <path class="f4" d="M12.1 3L13.8 3L8 13.2Z" />
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
