<script lang="ts">
  import { toast } from "../lib/toast.svelte";
</script>

<div class="toast" class:on={toast.on} role="status" aria-live="polite">{toast.message}</div>

<style>
  .toast {
    position: fixed;
    left: 50%;
    bottom: 24px;
    z-index: 50;
    padding: 10px 18px;
    border-radius: 10px;
    background: var(--ink);
    color: var(--base);
    font: 600 14px/1.5 var(--f);
    transform: translate(-50%, 200%);
    visibility: hidden;
    transition:
      transform 0.22s,
      visibility 0s 0.22s;
  }
  .toast.on {
    transform: translate(-50%, 0);
    visibility: visible;
    transition: transform 0.22s cubic-bezier(0.2, 0.9, 0.3, 1);
  }
</style>
