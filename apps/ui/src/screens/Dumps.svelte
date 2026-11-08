<script lang="ts">
  import AdminPill from "../components/AdminPill.svelte";
  import BackLink from "../components/BackLink.svelte";
  import Block from "../components/Block.svelte";
  import ConcernMark from "../components/ConcernMark.svelte";
  import ErrorView from "../components/ErrorView.svelte";
  import Icon from "../components/Icon.svelte";
  import Loading from "../components/Loading.svelte";
  import PickAll from "../components/PickAll.svelte";
  import { app } from "../lib/app.svelte";
  import { dumps } from "../lib/dumps.svelte";
  import { t, tn } from "../lib/i18n.svelte";
  import { type DumpGroup, href } from "../lib/router.svelte";
  import { toast } from "../lib/toast.svelte";
  import { keyOf, tweaks } from "../lib/tweaks.svelte";
  import Row from "./dumps/Row.svelte";
  import { at, entries, fileName, type Group, GROUPS, lastRead, lede, present, setting, space, TONE } from "./dumps/words";
  import { formatBytes } from "./occurrence/format";

  let { id, group }: { id: number | null; group: DumpGroup | null } = $props();

  let dialog = $state<HTMLDialogElement>();

  const level = $derived(app.settings?.settings.detail_level ?? "Simple");
  const list = $derived(dumps.dto ? entries(dumps.dto) : []);
  const shown = $derived(list.filter(present));
  const gone = $derived(list.filter((e) => !present(e)));
  const parts = $derived(dumps.dto ? space(list) : []);
  const chosen = $derived(list.flatMap((e) => (e.dump && dumps.chosen.includes(e.dump.id) ? [e.dump] : [])));
  const chosenBytes = $derived(chosen.reduce((sum, d) => sum + d.size_bytes, 0));
  const last = $derived(dumps.dto ? lastRead(dumps.dto) : null);
  const crash = $derived(setting(dumps.dto?.crash ?? null, level));
  // The helper's result reaches the history through the watcher.
  const blocked = $derived(!app.running);
  /** The dump tweaks the optimizations page recommends here. */
  const dumpTweaks = $derived(
    tweaks.dto?.advice.filter(
      (a) => (a.tweak === "CrashDump" || a.tweak === "AppDumps") && a.status === "Ready" && a.signals.length > 0,
    ) ?? [],
  );
  /** The entry whose page is open, when the address names a dump. */
  const opened = $derived(id === null ? null : (list.find((e) => e.dump?.id === id) ?? null));
  const groups = $derived<Group[]>(group ? [group] : GROUPS);

  function retry(): void {
    dumps.error = null;
    void dumps.load();
  }

  async function remove(): Promise<void> {
    dialog?.close();
    const freed = await dumps.remove();
    if (freed === null) {
      toast.show(t(dumps.deleting === "declined" ? "ui.dumps.delete.declined" : "ui.dumps.delete.failed"));
    } else if (freed.count < freed.asked) {
      toast.show(t("ui.dumps.delete.toast.partial", { count: freed.count, asked: freed.asked, size: formatBytes(freed.bytes) }));
    } else {
      toast.show(t("ui.dumps.delete.toast", { size: formatBytes(freed.bytes) }));
    }
  }

  const groupLabel = (g: Group) => t(`ui.dumps.group.${g}`);
</script>

{#if id === null}
  <div class="top">
    <h1>{group ? groupLabel(group) : t("ui.dumps.title")}</h1>
  </div>
{/if}

{#if dumps.dto === null && dumps.error !== null}
  <div class="gap"><ErrorView error={dumps.error} heading="h2" onretry={retry} /></div>
{:else if dumps.dto === null}
  <Loading />
{:else if id !== null}
  <BackLink
    href={href.dumps(null, opened?.group ?? null)}
    label={t("ui.common.back_to", { place: opened ? groupLabel(opened.group) : t("ui.dumps.title") })}
  />
  {#if opened}
    <Row entry={opened} {level} open />
  {:else}
    <h1>{t("ui.dumps.title")}</h1>
    <p class="muted">{t("ui.dumps.missing")}</p>
  {/if}
{:else}
  {#if !group}
    <p class="lede">
      {#each lede(list, dumps.dto, level) as part, i (i)}{#if part.bold}<b>{part.text}</b>{:else}{part.text}{/if}{/each}
    </p>
  {/if}

  {#each groups as g, index (g)}
    {@const members = shown.filter((e) => e.group === g)}
    {#if members.length}
      {@const pickable = members.flatMap((e) => (e.dump && e.dump.deleted_ms === null ? [e.dump.id] : []))}
      <Block title={groupLabel(g)} first={index === 0}>
        {#snippet actions()}
          {#if pickable.length}
            <PickAll
              ids={pickable}
              chosen={dumps.chosen}
              label={t("ui.dumps.pick_all", { group: groupLabel(g) })}
              onchange={(on) => dumps.chooseAll(pickable, on)}
            />
          {/if}
        {/snippet}
        <ul class="rows">
          {#each members as entry (entry.key)}
            <li><Row {entry} {level} href={href.dumps(entry.dump?.id ?? null, entry.group)} /></li>
          {/each}
        </ul>
      </Block>
    {/if}
  {/each}
  {#if !shown.filter((e) => groups.includes(e.group)).length}
    <p class="muted empty">{t("ui.dumps.empty")}</p>
  {/if}
  {#if gone.length && !group}
    <Block title={tn("ui.dumps.gone", gone.length)}>
      <ul class="rows">
        {#each gone as entry (entry.key)}
          <li><Row {entry} {level} href={href.dumps(entry.dump?.id ?? null, entry.group)} /></li>
        {/each}
      </ul>
    </Block>
  {/if}

  {#if !group}
    {#if parts.length}
      <Block title={t("ui.dumps.space.title")}>
        <p class="big">
          {formatBytes(dumps.dto.space_bytes)}
          <small>{tn("ui.dumps.space.in", shown.filter((e) => e.dump).length)}</small>
        </p>
        <div
          class="bar"
          role="img"
          aria-label={parts.map((p) => `${groupLabel(p.group)}: ${formatBytes(p.bytes)}`).join("; ")}
        >
          {#each parts as p (p.group)}
            <i style:flex-grow={p.bytes} style:background="var(--{TONE[p.group]})"></i>
          {/each}
        </div>
        <div class="key" aria-hidden="true">
          {#each parts as p (p.group)}
            <span><i style:background="var(--{TONE[p.group]})"></i>{groupLabel(p.group)} {formatBytes(p.bytes)}</span>
          {/each}
        </div>
      </Block>
    {/if}

    <Block title={t("ui.dumps.read.title")}>
      <div class="read">
        {#if dumps.reading === "running"}
          <p class="status" role="status"><span class="spin" aria-hidden="true"></span>{t("ui.dumps.read.running")}</p>
        {:else}
          {#if dumps.reading === "declined"}
            <p role="status"><b>{t("ui.dumps.read.declined.title")}</b> {t("ui.dumps.read.declined.body")}</p>
          {:else if dumps.reading === "failed"}
            <p role="status"><b>{t("ui.dumps.read.failed.title")}</b> {t("ui.dumps.read.failed.body")}</p>
          {:else if dumps.reading === "done"}
            <p role="status"><b>{t("ui.dumps.read.done")}</b></p>
          {/if}
          <p>{t(level === "Technical" ? "ui.dumps.read.lede.tech" : "ui.dumps.read.lede")}</p>
          <div class="line">
            <AdminPill />
            <button
              type="button"
              class="btn"
              class:accent={last === null}
              disabled={blocked}
              onclick={() => void dumps.read()}
            >
              <Icon name="shield" />
              {t(
                dumps.reading === "declined" || dumps.reading === "failed"
                  ? "ui.dumps.read.again"
                  : last === null
                    ? "ui.dumps.read.start"
                    : "ui.dumps.read.repeat",
              )}
            </button>
            <span>
              {blocked
                ? t("ui.dumps.read.blocked")
                : last === null
                  ? t("ui.dumps.read.never")
                  : t("ui.dumps.read.last", { when: at(last) })}
            </span>
          </div>
        {/if}
      </div>
    </Block>

    <Block title={t("ui.dumps.setting.title")}>
      <p class="setting">
        <ConcernMark mark={crash.ok ? "Ok" : "Attention"} word={!crash.ok} />
        <span>{crash.text}</span>
      </p>
      {#each dumpTweaks as a (keyOf(a))}
        <p class="tweak"><Icon name="tweaks" /><a href={href.tweaks(keyOf(a))}>{t(`ui.dumps.tweak.${a.tweak}`)}</a></p>
      {/each}
    </Block>
  {/if}
{/if}

{#if dumps.dto !== null}
  {#if chosen.length}
    <div class="delbar" role="region" aria-label={t("ui.dumps.delete.bar")}>
      <p>
        {tn("ui.dumps.delete.chosen", chosen.length)}
        <small>{t("ui.dumps.delete.kept", { size: formatBytes(chosenBytes) })}</small>
      </p>
      <div class="acts">
        <button type="button" class="btn" onclick={() => (dumps.chosen = [])}>{t("ui.dumps.delete.clear")}</button>
        <button
          type="button"
          class="btn danger"
          disabled={blocked || dumps.deleting === "running"}
          onclick={() => dialog?.showModal()}
        >
          <Icon name="trash" />
          {t(dumps.deleting === "running" ? "ui.dumps.delete.running" : "ui.dumps.delete.start")}
        </button>
      </div>
    </div>
  {/if}

  <dialog class="confirm" bind:this={dialog} aria-labelledby="confirm-title">
    <h2 id="confirm-title">{tn("ui.dumps.delete.ask", chosen.length)}</h2>
    <p>{t("ui.dumps.delete.ask.body", { size: formatBytes(chosenBytes) })}</p>
    <ul>
      {#each chosen as d (d.id)}
        <li>{fileName(d.path)} · {formatBytes(d.size_bytes)}</li>
      {/each}
    </ul>
    <div class="acts">
      <button type="button" class="btn" onclick={() => dialog?.close()}>{t("ui.dumps.delete.cancel")}</button>
      <button type="button" class="btn danger" onclick={() => void remove()}>
        <Icon name="shield" />{t("ui.dumps.delete.confirm")}
      </button>
    </div>
  </dialog>
{/if}

<style>
  .top {
    display: flex;
    flex-wrap: wrap;
    gap: 12px;
    align-items: center;
    justify-content: space-between;
  }
  .gap {
    margin-top: 14px;
  }
  .lede {
    max-width: 70ch;
    margin: 6px 0 18px;
    color: var(--ink2);
    font-size: 16px;
  }
  .lede b {
    color: var(--ink);
    font-weight: 600;
  }
  .rows {
    display: grid;
    gap: 4px;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .muted {
    margin: 0;
    color: var(--ink2);
    font-size: 14px;
  }
  .empty {
    margin-top: 16px;
  }
  .big {
    margin: 0 0 12px;
    font: 600 34px/1 var(--fd);
    letter-spacing: -0.02em;
  }
  .big small {
    margin-left: 6px;
    color: var(--ink2);
    font: 400 14px var(--f);
    letter-spacing: 0;
  }
  .bar {
    display: flex;
    gap: 2px;
    height: 12px;
    margin-bottom: 8px;
    overflow: hidden;
    border: 1px solid var(--stroke2);
    border-radius: 999px;
    background: var(--card2);
  }
  .bar i {
    flex-basis: 0;
    min-width: 4px;
  }
  .key {
    display: flex;
    flex-wrap: wrap;
    gap: 4px 14px;
    color: var(--ink2);
    font-size: 13px;
  }
  .key i {
    display: inline-block;
    width: 8px;
    height: 8px;
    margin-right: 6px;
    border-radius: 2px;
  }
  .read {
    display: grid;
    gap: 10px;
  }
  .read p {
    margin: 0;
    color: var(--ink2);
    font-size: 14px;
    line-height: 1.45;
  }
  .read p b {
    color: var(--ink);
    font-weight: 600;
  }
  .status {
    display: flex;
    gap: 10px;
    align-items: center;
  }
  .read .status {
    color: var(--ink);
  }
  .spin {
    flex: none;
    width: 14px;
    height: 14px;
    border: 2px solid var(--stroke);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin 1s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  .line {
    display: flex;
    flex-wrap: wrap;
    gap: 8px 14px;
    align-items: center;
    color: var(--ink2);
    font-size: 13px;
  }
  .setting {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
    gap: 10px;
    align-items: start;
    margin: 0;
    color: var(--ink2);
    font-size: 13.5px;
    line-height: 1.45;
    overflow-wrap: anywhere;
  }
  .tweak {
    display: flex;
    gap: 8px;
    align-items: center;
    margin: 10px 0 0;
    font-size: 13.5px;
  }
  .tweak a {
    color: var(--accent);
    font-weight: 600;
  }
  .setting :global(.cn) {
    padding-top: 1px;
  }
  .delbar {
    position: sticky;
    bottom: 0;
    display: flex;
    flex-wrap: wrap;
    gap: 12px;
    align-items: center;
    justify-content: space-between;
    margin-top: 14px;
    padding: 12px 16px;
    border: 1px solid var(--stroke);
    border-radius: 14px;
    background: var(--card);
    box-shadow: 0 8px 28px #0003;
  }
  .delbar p {
    margin: 0;
    font-size: 14px;
    font-weight: 600;
  }
  .delbar small {
    display: block;
    color: var(--ink2);
    font-size: 13px;
    font-weight: 400;
  }
  .acts {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    justify-content: flex-end;
  }
  .btn.danger {
    border-color: transparent;
    background: var(--freeze);
    color: var(--on-accent);
  }
  .btn.danger:hover {
    filter: brightness(1.07);
  }
  .confirm {
    max-width: min(520px, calc(100vw - 32px));
    padding: 18px 20px;
    border: 1px solid var(--stroke);
    border-radius: 14px;
    background: var(--card);
    color: var(--ink);
    box-shadow: 0 18px 60px #0006;
  }
  .confirm::backdrop {
    background: #0008;
  }
  .confirm h2 {
    margin: 0 0 8px;
    font: 600 18px/1.3 var(--fd);
  }
  .confirm p {
    margin: 0 0 10px;
    color: var(--ink2);
    font-size: 14px;
    line-height: 1.5;
  }
  .confirm ul {
    display: grid;
    gap: 4px;
    max-height: 160px;
    margin: 0 0 12px;
    padding-left: 18px;
    overflow: auto;
    font-size: 13px;
  }
  @media (prefers-reduced-motion: reduce) {
    .spin {
      animation-duration: 3s;
    }
  }
  @media (forced-colors: active) {
    .bar i,
    .key i,
    .spin {
      forced-color-adjust: none;
    }
    .bar i,
    .key i {
      background: CanvasText;
    }
    .spin {
      border-color: GrayText;
      border-top-color: Highlight;
    }
  }
</style>
