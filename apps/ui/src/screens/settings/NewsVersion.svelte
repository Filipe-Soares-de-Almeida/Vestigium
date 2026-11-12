<script lang="ts">
  import { onMount, tick } from "svelte";

  import BackLink from "../../components/BackLink.svelte";
  import Icon from "../../components/Icon.svelte";
  import Loading from "../../components/Loading.svelte";
  import PageHead from "../../components/PageHead.svelte";
  import { captures } from "../../lib/capture.svelte";
  import { t } from "../../lib/i18n.svelte";
  import { changes, CUT_TO, heading, parseEntryKey, plainNotes, releaseDay, shownOf, tagOf, wallOf } from "../../lib/notes";
  import { href } from "../../lib/router.svelte";
  import { hwtests } from "../../lib/tests.svelte";
  import { toast } from "../../lib/toast.svelte";
  import { type Release, updates } from "../../lib/update.svelte";
  import { copyText } from "../occurrence/browser";

  // One version's notes in full, group by group; opened on an entry, it scrolls to it and marks it.
  let { version, entry }: { version: string; entry: string | null } = $props();

  let notes = $state.raw<Release[] | null>(null);
  /** The long groups asked to show every entry. */
  let opened = $state<string[]>([]);
  let doc = $state<HTMLElement>();

  onMount(() => {
    void updates.allNotes().then((found) => (notes = found));
  });

  const release = $derived.by((): Release | null | undefined => {
    if (notes === null) return undefined;
    return wallOf(notes, updates.offered).find((r) => r.version === version) ?? null;
  });
  const tag = $derived(tagOf(version, updates.version, updates.found?.version ?? null));
  const target = $derived(parseEntryKey(entry));
  const sub = $derived.by(() => {
    const parts = [release ? releaseDay(release.date) : null, tag ? t(`ui.news.tag.${tag}`) : null];
    return parts.filter((p) => p !== null).join(" · ") || undefined;
  });
  const busy = $derived(hwtests.running !== null || captures.run !== null);

  /** Whether a group shows every entry: a short one always, a long one once asked or opened on one of its entries. */
  const whole = (kind: string, count: number) =>
    shownOf(count) === count || opened.includes(kind) || target?.kind === kind;
  const anchor = (kind: string, index: number) => `entry-${kind}-${index}`;

  // The entry the page was opened on is brought into view once the notes are on screen.
  $effect(() => {
    const want = target;
    const box = doc;
    if (!want || !box || !release) return;
    void tick().then(() => {
      const item = box.querySelector<HTMLElement>(`#${anchor(want.kind, want.index)}`);
      if (item) box.scrollTop = Math.max(0, item.offsetTop - box.clientHeight / 3);
    });
  });

  async function showRest(kind: string): Promise<void> {
    opened = [...opened, kind];
    await tick();
    doc?.querySelector<HTMLElement>(`#${anchor(kind, CUT_TO)}`)?.focus();
  }

  async function copy(shown: Release): Promise<void> {
    const copied = await copyText(plainNotes(shown));
    toast.show(t(copied ? "ui.news.copied" : "ui.news.copy_failed"));
  }

  /** Starts the download the dialog follows; with a test or capture running, the dialog says to wait. */
  function install(): void {
    updates.dialog = "available";
    if (!busy) void updates.install();
  }
</script>

<div class="fill">
  <BackLink href={href.news()} label={t("ui.news.title")} />
  {#if release === undefined}
    <PageHead title={t("ui.update.version", { version })} />
    <Loading />
  {:else if release === null}
    <PageHead title={t("ui.update.version", { version })} />
    <p class="empty">{t("ui.news.missing", { version })}</p>
  {:else}
    {@const shown = release}
    {#snippet actions()}
      {#if tag === "available"}
        <button type="button" class="btn accent" onclick={install}>{t("ui.update.available.install")}</button>
      {/if}
      <button type="button" class="btn" onclick={() => copy(shown)}><Icon name="copy" />{t("ui.news.copy")}</button>
    {/snippet}
    <PageHead title={t("ui.update.version", { version })} {sub} {actions} />
    <div class="doc" bind:this={doc}>
      {#if shown.summary}<p class="sum">{shown.summary}</p>{/if}
      {#each shown.groups as group (group.kind)}
        {@const all = whole(group.kind, group.items.length)}
        <section class="grp" aria-labelledby="group-{group.kind}">
          <h2 id="group-{group.kind}">{heading(group.kind)} <small>· {changes(group.items.length)}</small></h2>
          <ul>
            {#each all ? group.items : group.items.slice(0, CUT_TO) as item, index (index)}
              <li
                id={anchor(group.kind, index)}
                tabindex="-1"
                class:target={target?.kind === group.kind && target.index === index}
              >
                {item}
              </li>
            {/each}
          </ul>
          {#if !all}
            <button type="button" class="btn sm" onclick={() => showRest(group.kind)}>
              {t("ui.news.show_rest", { count: group.items.length - CUT_TO })}
            </button>
          {/if}
        </section>
      {/each}
    </div>
  {/if}
</div>

<style>
  .fill {
    display: flex;
    flex: 1;
    flex-direction: column;
    min-height: 0;
  }
  .fill > :global(.back) {
    align-self: flex-start;
  }
  .doc {
    position: relative;
    flex: 1;
    min-height: 0;
    margin-top: 18px;
    padding: 4px 28px 48px;
    overflow: auto;
    border: 1px solid var(--stroke);
    border-radius: 10px;
    background: var(--card);
    scrollbar-gutter: stable;
  }
  .sum {
    max-width: 72ch;
    margin: 18px 0 0;
    color: var(--ink);
    font-size: 15px;
    line-height: 1.55;
  }
  .grp {
    margin-top: 22px;
  }
  h2 {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 4px 10px;
    margin: 0 0 10px;
  }
  h2 small {
    color: var(--ink2);
    font: 400 13px/1 var(--f);
  }
  ul {
    display: grid;
    gap: 10px;
    max-width: 76ch;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  li {
    position: relative;
    isolation: isolate;
    padding-left: 16px;
    border-radius: 4px;
    font-size: 14px;
    line-height: 1.5;
  }
  li::before {
    content: "";
    position: absolute;
    top: 0.62em;
    left: 3px;
    width: 5px;
    height: 5px;
    border-radius: 50%;
    background: var(--ink3);
  }
  /* The entry the page was opened on glows once, behind its text. */
  li.target::after {
    content: "";
    position: absolute;
    inset: -4px -8px;
    z-index: -1;
    border-radius: 6px;
    background: var(--accent-t);
    opacity: 0;
    animation: flash 1.6s ease-out;
  }
  @keyframes flash {
    from {
      opacity: 1;
    }
    to {
      opacity: 0;
    }
  }
  .btn.sm {
    min-height: 30px;
    margin: 12px 0 0 16px;
    padding: 0 12px;
    font-size: 13px;
  }
  .empty {
    margin-top: 24px;
  }
  @media (prefers-reduced-motion: reduce) {
    li.target::after {
      opacity: 1;
      animation: none;
    }
  }
  @media (forced-colors: active) {
    .doc {
      border-color: CanvasText;
    }
    li.target::after {
      border: 1px solid Highlight;
    }
  }
</style>
