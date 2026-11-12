<script lang="ts">
  import { tick } from "svelte";

  import { t } from "../lib/i18n.svelte";
  import type { RestorePoint } from "../lib/types";
  import { noPointTitle, pointCase, pointTrail } from "../screens/tweaks/words";
  import StatusMark from "./StatusMark.svelte";

  // Says why no restore point stands behind a change and how far the request got, and lets the
  // person try again, continue without one, or give up. It stays open while a new attempt runs.
  let {
    point,
    attempt,
    busy,
    note,
    onretry,
    ongo,
    oncancel,
  }: {
    point: RestorePoint;
    /** The attempt this describes, counting from 1. */
    attempt: number;
    /** A new attempt is under way. */
    busy: boolean;
    /** What the last attempt added, when it ended the same way. */
    note: string;
    onretry: () => void;
    ongo: () => void;
    oncancel: () => void;
  } = $props();

  let dialog = $state<HTMLDialogElement>();
  let again = $state<HTMLButtonElement>();

  const kind = $derived(pointCase(point));
  const trail = $derived(pointTrail(point));

  $effect(() => {
    if (dialog && !dialog.open) dialog.showModal();
  });

  // A finished attempt hands the focus back to Try again, which the busy state had disabled.
  $effect(() => {
    if (!busy && attempt > 0) void tick().then(() => again?.focus());
  });
</script>

<dialog
  class="point"
  bind:this={dialog}
  aria-labelledby="point-title"
  aria-describedby="point-body"
  oncancel={(event) => {
    event.preventDefault();
    if (!busy) oncancel();
  }}
>
  <h2 id="point-title">{noPointTitle(kind)}</h2>
  <p id="point-body">
    {t("ui.tweaks.no_point.lead")}
    {t(kind === "off" ? "ui.tweaks.no_point.tail.off" : "ui.tweaks.no_point.tail")}
  </p>
  <ol class="trail" aria-label={t("ui.tweaks.no_point.trail")}>
    {#each trail as step (step.name)}
      <li class:here={step.here} class:dim={step.mark === "info"}>
        <StatusMark kind={step.mark} size={18} word={step.word} />
        <span class="nm">{step.name}</span>
        <span class="sx">{step.word}</span>
      </li>
    {/each}
  </ol>
  {#if note}<p class="note" role="status">{note}</p>{/if}
  {#if busy}
    <p class="busy" role="status"><i class="spin" aria-hidden="true"></i>{t("ui.tweaks.no_point.retrying")}</p>
  {/if}
  <div class="acts">
    <button type="button" class="btn" disabled={busy} onclick={oncancel}>{t("ui.tweaks.cancel")}</button>
    <button type="button" class="btn" disabled={busy} onclick={ongo}>{t("ui.tweaks.no_point.go")}</button>
    <button
      type="button"
      class="btn accent"
      bind:this={again}
      disabled={busy}
      aria-label={t("ui.tweaks.no_point.retry.label")}
      onclick={onretry}
    >
      {t("ui.tweaks.no_point.retry")}
    </button>
  </div>
</dialog>

<style>
  .point {
    width: min(560px, calc(100vw - 32px));
    padding: 18px 20px;
    border: 1px solid var(--stroke);
    border-radius: 8px;
    background: var(--card);
    color: var(--ink);
    box-shadow: var(--shadow-float);
  }
  .point::backdrop {
    background: #0008;
  }
  h2 {
    margin: 0 0 10px;
  }
  p {
    margin: 0 0 12px;
    color: var(--ink2);
    font-size: 14px;
    line-height: 1.5;
  }
  .trail {
    display: grid;
    gap: 4px;
    margin: 0 0 14px;
    padding: 0;
    list-style: none;
  }
  .trail li {
    display: grid;
    grid-template-columns: 22px minmax(0, 1fr) auto;
    gap: 12px;
    align-items: center;
    min-height: 40px;
    padding: 6px 12px;
    border-radius: 6px;
    background: var(--row);
    font-size: 14px;
  }
  .trail li.here {
    box-shadow: inset 0 0 0 1px var(--stroke);
    background: var(--row-hover);
  }
  .trail li.dim .nm {
    color: var(--ink3);
  }
  .trail :global(.word) {
    display: none;
  }
  .trail :global(.cut) {
    stroke: var(--card);
  }
  .sx {
    color: var(--ink2);
    font-size: 13px;
    text-align: right;
  }
  .note {
    color: var(--ink);
    font-weight: 600;
  }
  .busy {
    display: flex;
    align-items: center;
    gap: 10px;
    color: var(--ink);
  }
  .spin {
    flex: none;
    width: 14px;
    height: 14px;
    border: 2px solid var(--stroke);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: turn 0.8s linear infinite;
  }
  @keyframes turn {
    to {
      transform: rotate(360deg);
    }
  }
  .acts {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    justify-content: flex-end;
    margin-top: 16px;
  }
  @media (prefers-reduced-motion: reduce) {
    .spin {
      animation: none;
    }
  }
  @media (forced-colors: active) {
    .point {
      border-color: CanvasText;
    }
  }
</style>
