<script lang="ts">
  import AdminPill from "../components/AdminPill.svelte";
  import BackLink from "../components/BackLink.svelte";
  import Block from "../components/Block.svelte";
  import Checkbox from "../components/Checkbox.svelte";
  import ErrorView from "../components/ErrorView.svelte";
  import HelperDialogs from "../components/HelperDialogs.svelte";
  import HelperLine from "../components/HelperLine.svelte";
  import Icon from "../components/Icon.svelte";
  import Loading from "../components/Loading.svelte";
  import PageHead from "../components/PageHead.svelte";
  import PickAll from "../components/PickAll.svelte";
  import StatusMark from "../components/StatusMark.svelte";
  import { app } from "../lib/app.svelte";
  import { dumps } from "../lib/dumps.svelte";
  import { t, tn } from "../lib/i18n.svelte";
  import { type DumpGroup, href } from "../lib/router.svelte";
  import { toast } from "../lib/toast.svelte";
  import { relatedLinks } from "../lib/related.svelte";
  import { keyOf, tweaks } from "../lib/tweaks.svelte";
  import DumpPage from "./dumps/DumpPage.svelte";
  import { at, type Entry, entries, fileName, type Group, GROUPS, isRead, lastRead, lede, partialDeleteText, present, setting, space, sub, title, TONE } from "./dumps/words";
  import { formatBytes } from "./occurrence/format";

  let { id, group }: { id: number | null; group: DumpGroup | null } = $props();

  let askingAll = $state(false);

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
  /** The dumps the select-all box covers: every one listed on this page. */
  const scope = $derived(
    shown.flatMap((e) => (e.dump && e.dump.deleted_ms === null && (!group || e.group === group) ? [e.dump.id] : [])),
  );

  relatedLinks(() => [
    { label: t("ui.related.app_dumps"), href: href.tweaks("AppDumps") },
    { label: t("ui.item.memory_tests"), href: href.tests("memory") },
    { label: t("ui.related.hardware"), href: href.health("hardware") },
  ]);

  function retry(): void {
    dumps.error = null;
    void dumps.load();
  }

  async function remove(): Promise<void> {
    askingAll = false;
    const freed = await dumps.remove();
    if (freed === null) {
      toast.show(t(dumps.deleting === "declined" ? "ui.dumps.delete.declined" : "ui.dumps.delete.failed"));
    } else if (freed.count < freed.asked) {
      toast.show(partialDeleteText(freed));
    } else {
      toast.show(t("ui.dumps.delete.toast", { size: formatBytes(freed.bytes) }));
    }
  }

  const groupLabel = (g: Group) => t(`ui.dumps.group.${g}`);
</script>

{#snippet bar()}
  <div class="dtools">
    {#if askingAll && chosen.length}
      <div class="confirm" role="group" aria-label={t("ui.dumps.delete.bar")}>
        <span>{tn("ui.dumps.delete.ask_many", chosen.length, { size: formatBytes(chosenBytes) })}</span>
        <button type="button" class="btn" disabled={blocked || dumps.deleting === "running"} onclick={() => void remove()}>
          <Icon name="trash" />{t(dumps.deleting === "running" ? "ui.dumps.delete.running" : "ui.dumps.delete.confirm")}
        </button>
        <button type="button" class="btn" onclick={() => (askingAll = false)}>{t("ui.dumps.delete.cancel")}</button>
      </div>
    {:else}
      <div class="dbar">
        {#if scope.length > 1}
          <PickAll ids={scope} chosen={dumps.chosen} label={t("ui.dumps.pick_all_plain")} onchange={(on) => dumps.chooseAll(scope, on)} text={t("ui.dumps.pick_all_plain")} />
        {/if}
        <button type="button" class="btn" disabled={!chosen.length || blocked} onclick={() => (askingAll = true)}>
          <Icon name="trash" />{chosen.length ? t("ui.dumps.delete.chosen_n", { count: chosen.length }) : t("ui.dumps.delete.chosen_none")}
        </button>
        <span class="muted">{chosen.length ? t("ui.dumps.delete.size", { size: formatBytes(chosenBytes) }) : t("ui.dumps.delete.hint")}</span>
      </div>
    {/if}
  </div>
{/snippet}

{#snippet rows(members: Entry[])}
  {#if members.length}
    <ul class="dl">
      {#each members as entry (entry.key)}
        {@const d = entry.dump}
        <li class="dl-i">
          {#if d}
            <span class="pick">
              <Checkbox checked={dumps.chosen.includes(d.id)} onchange={(on) => dumps.choose(d.id, on)}>
                <span class="sr">{t("ui.dumps.pick", { file: fileName(d.path) })}</span>
              </Checkbox>
            </span>
          {:else}
            <span class="pick"></span>
          {/if}
          <a class="row" href={href.dumps(d?.id ?? null, entry.group)} data-reveal>
            <span class="m"><span class="t">{title(entry, level)}</span><span class="s">{sub(entry, level)}</span></span>
            <span class="r">
              {#if !isRead(entry)}<StatusMark kind="info" word={t("ui.dumps.unread")} reveal />{/if}
              <span>{at(entry.at)}</span>
              <span class="sz">{d ? formatBytes(d.size_bytes) : ""}</span>
            </span>
            <span class="ch"><Icon name="chevron" /></span>
          </a>
        </li>
      {/each}
    </ul>
  {:else}
    <p class="empty">{t("ui.dumps.group_empty")}</p>
  {/if}
{/snippet}

{#if dumps.dto === null && dumps.error !== null}
  <PageHead title={t("ui.dumps.title")} />
  <div class="gap"><ErrorView error={dumps.error} heading="h2" onretry={retry} /></div>
{:else if dumps.dto === null}
  <PageHead title={t("ui.dumps.title")} />
  <Loading />
{:else if id !== null}
  <BackLink href={href.dumps(null, opened?.group ?? null)} label={opened ? groupLabel(opened.group) : t("ui.dumps.title")} />
  {#if opened}
    <DumpPage entry={opened} {level} />
  {:else}
    <PageHead title={t("ui.dumps.title")} sub={t("ui.dumps.missing")} />
  {/if}
{:else if group}
  <PageHead title={groupLabel(group)} sub={tn("ui.dumps.count", shown.filter((e) => e.group === group).length)} />
  {@render bar()}
  <section class="list">{@render rows(shown.filter((e) => e.group === group))}</section>
  {#if gone.length}<p class="note">{tn("ui.dumps.gone", gone.length)}</p>{/if}
{:else}
  <PageHead title={t("ui.dumps.title")} />
  <p class="lede">
    {#each lede(list, dumps.dto, level) as part, i (i)}{#if part.bold}<b>{part.text}</b>{:else}{part.text}{/if}{/each}
  </p>
  {@render bar()}
  {#each GROUPS as g (g)}
    <Block title={groupLabel(g)}>
      {@render rows(shown.filter((e) => e.group === g))}
    </Block>
  {/each}
  {#if gone.length}<p class="note">{tn("ui.dumps.gone", gone.length)}</p>{/if}

  <div class="cols2">
    <Block title={t("ui.dumps.space.title")}>
      {#if parts.length}
        <div class="sbar" role="img" aria-label={parts.map((p) => `${groupLabel(p.group)}: ${formatBytes(p.bytes)}`).join("; ")}>
          {#each parts as p (p.group)}
            <i style:flex-grow={p.bytes} style:background="var(--{TONE[p.group]})"></i>
          {/each}
        </div>
        <ul class="skey">
          {#each parts as p (p.group)}
            <li><i class="dot" style:background="var(--{TONE[p.group]})"></i><span>{groupLabel(p.group)}</span><b>{formatBytes(p.bytes)}</b></li>
          {/each}
        </ul>
      {/if}
      <p class="note">{formatBytes(dumps.dto.space_bytes)} {tn("ui.dumps.space.in", shown.filter((e) => e.dump).length)}</p>
    </Block>

    <Block title={t("ui.dumps.read.title")}>
      {#if dumps.reading === "running"}
        <p class="p status" role="status"><span class="spin" aria-hidden="true"></span>{t("ui.dumps.read.running")}</p>
        <div class="acts">
          <button type="button" class="btn" onclick={() => void dumps.cancelRead()}>{t("ui.helper.cancel.dumps")}</button>
        </div>
      {:else}
        <HelperLine job="dumps" />
        {#if dumps.reading === "declined"}
          <p class="p" role="status"><b>{t("ui.dumps.read.declined.title")}</b> {t("ui.dumps.read.declined.body")}</p>
        {:else if dumps.reading === "failed"}
          <p class="p" role="status"><b>{t("ui.dumps.read.failed.title")}</b> {t("ui.dumps.read.failed.body")}</p>
        {:else if dumps.reading === "done"}
          <p class="p" role="status"><b>{t("ui.dumps.read.done")}</b></p>
        {/if}
        <p class="p">{t(level === "Technical" ? "ui.dumps.read.lede.tech" : "ui.dumps.read.lede")}</p>
        <p class="last">
          {blocked ? t("ui.dumps.read.blocked") : last === null ? t("ui.dumps.read.never") : t("ui.dumps.read.last", { when: at(last) })}
        </p>
        <div class="acts">
          <button type="button" class="btn" class:accent={last === null} disabled={blocked} onclick={() => void dumps.read()}>
            <Icon name="lock" />
            {t(
              dumps.reading === "declined" || dumps.reading === "failed"
                ? "ui.dumps.read.again"
                : last === null
                  ? "ui.dumps.read.start"
                  : "ui.dumps.read.repeat",
            )}
          </button>
          <AdminPill />
        </div>
      {/if}
      <HelperDialogs job="dumps" onretry={() => void dumps.read()} />
    </Block>
  </div>

  <Block title={t("ui.dumps.setting.title")}>
    <p class="state">
      <StatusMark kind={crash.ok ? "ok" : "attention"} word={t(crash.ok ? "ui.dumps.setting.on" : "ui.health.mark.Attention")} />
      <span>{crash.text}</span>
    </p>
    {#each dumpTweaks as a (keyOf(a))}
      <p class="more"><a href={href.tweaks(keyOf(a))}>{t(`ui.dumps.tweak.${a.tweak}`)}</a></p>
    {/each}
  </Block>
{/if}

<style>
  .gap {
    margin-top: 14px;
  }
  .dtools {
    margin-top: 22px;
  }
  .dbar {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 10px 16px;
  }
  .muted {
    font-size: 13px;
  }
  .confirm {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 10px 14px;
    max-width: 840px;
    padding: 12px 14px;
    border-radius: 8px;
    background: var(--hover);
    font-size: 14px;
  }
  .confirm > span {
    flex: 1 1 280px;
  }
  .list {
    margin-top: 32px;
    padding-top: 28px;
    border-top: 1px solid var(--rule);
  }
  .dl {
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    gap: 4px;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .dl-i {
    display: flex;
    align-items: center;
    gap: 4px;
    border-radius: 6px;
    background: var(--row);
  }
  .dl-i:hover {
    background: var(--row-hover);
  }
  .pick {
    display: flex;
    padding: 10px;
  }
  .row {
    display: flex;
    flex: 1;
    align-items: center;
    gap: 16px;
    min-width: 0;
    min-height: 56px;
    padding: 10px 12px 10px 0;
    border-radius: 6px;
    color: var(--ink);
    text-decoration: none;
  }
  .row:hover {
    text-decoration: none;
  }
  .row:focus-visible {
    outline: 2px solid var(--ink);
    outline-offset: -2px;
  }
  .m {
    display: grid;
    flex: 1;
    gap: 1px;
    min-width: 0;
  }
  .t {
    overflow: hidden;
    font-size: 14.5px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .s {
    overflow: hidden;
    color: var(--ink2);
    font-size: 13px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .r {
    display: flex;
    flex: none;
    align-items: center;
    gap: 16px;
    color: var(--ink2);
    font-size: 13px;
    white-space: nowrap;
  }
  .sz {
    min-width: 58px;
    color: var(--ink);
    text-align: right;
  }
  .ch {
    display: flex;
    flex: none;
    color: var(--ink3);
  }
  .sbar {
    display: flex;
    gap: 2px;
    height: 10px;
    overflow: hidden;
    border-radius: 6px;
  }
  .sbar i {
    display: block;
  }
  .skey {
    display: grid;
    gap: 6px;
    margin: 12px 0 0;
    padding: 0;
    font-size: 13px;
    list-style: none;
  }
  .skey li {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--ink2);
  }
  .skey span {
    flex: 1;
  }
  .skey b {
    color: var(--ink);
    font-weight: 600;
  }
  .dot {
    display: inline-block;
    flex: none;
    width: 8px;
    height: 8px;
    border-radius: 50%;
  }
  .last {
    margin: 0;
    color: var(--ink2);
    font-size: 13px;
  }
  .state {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px 14px;
    margin: 0 0 10px;
    color: var(--ink2);
    font-size: 13px;
  }
  .status {
    display: flex;
    gap: 10px;
    align-items: center;
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
  @media (forced-colors: active) {
    .sbar i,
    .dot {
      forced-color-adjust: none;
      background: CanvasText !important;
    }
  }
</style>
