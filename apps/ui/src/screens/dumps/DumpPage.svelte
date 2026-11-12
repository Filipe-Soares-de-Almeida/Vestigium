<script lang="ts">
  import AdminPill from "../../components/AdminPill.svelte";
  import Block from "../../components/Block.svelte";
  import Icon from "../../components/Icon.svelte";
  import PageHead from "../../components/PageHead.svelte";
  import Steps from "../../components/Steps.svelte";
  import { marks } from "../../lib/api";
  import { app } from "../../lib/app.svelte";
  import { dumps } from "../../lib/dumps.svelte";
  import { t } from "../../lib/i18n.svelte";
  import { href } from "../../lib/router.svelte";
  import { toast } from "../../lib/toast.svelte";
  import type { DetailLevel } from "../../lib/types";
  import { formatBytes } from "../occurrence/format";
  import { at, culpritView, type Entry, fileName, hex, isRead, partialDeleteText, said, steps, stopName, stopSentence, title } from "./words";

  let { entry, level }: { entry: Entry; level: DetailLevel } = $props();

  const FALL_LOOK_MS = 10 * 60_000;

  const d = $derived(entry.dump);
  const gone = $derived(d !== null && d.deleted_ms !== null);
  const read = $derived(isRead(entry));
  const culprit = $derived(d && read && d.culprit ? culpritView(d, d.culprit, level) : null);
  const todo = $derived(d && read ? steps(d) : []);
  const fault = $derived(d?.fault ?? null);
  const blocked = $derived(!app.running);
  let asking = $state(false);
  /** The post-mortem report of the fall this dump belongs to, once looked up. */
  let fall = $state<number | null>(null);

  const sub = $derived(
    [t(`ui.dumps.group.${entry.group}`), at(entry.at), d ? formatBytes(d.size_bytes) : ""].filter(Boolean).join(" · "),
  );

  $effect(() => {
    if (!d || (d.kind !== "KernelMemory" && d.kind !== "KernelMini")) return;
    const around = d.crash_ms;
    void marks(around - FALL_LOOK_MS, around + FALL_LOOK_MS)
      .then((m) => {
        fall = m.occurrences.find((o) => o.kind === "PostMortem")?.id ?? null;
      })
      .catch(() => (fall = null));
  });

  async function remove(): Promise<void> {
    if (!d) return;
    asking = false;
    dumps.chosen = [d.id];
    const freed = await dumps.remove();
    if (freed === null) {
      toast.show(t(dumps.deleting === "declined" ? "ui.dumps.delete.declined" : "ui.dumps.delete.failed"));
    } else if (freed.count < freed.asked) {
      toast.show(partialDeleteText(freed));
    } else {
      toast.show(t("ui.dumps.delete.toast", { size: formatBytes(freed.bytes) }));
    }
  }
</script>

<PageHead title={title(entry, level)} {sub} />

{#if gone}
  <p class="lede">{t("ui.dumps.detail.deleted", { when: at(d?.deleted_ms ?? 0) })}</p>
{/if}

{#if d && read}
  <div class="cols2">
    <Block title={t("ui.dumps.detail.culprit")}>
      {#if culprit}
        <p class="cz-n">{culprit.name}</p>
        <p class="cz-m">{culprit.sub}</p>
      {:else}
        <p class="p">{t("ui.dumps.detail.no_culprit")}</p>
      {/if}
    </Block>
    <Block title={t("ui.dumps.detail.code")}>
      {#if fault && "BugCheck" in fault}
        <p class="code">{hex(fault.BugCheck.code)}</p>
        <p class="code">{stopName(fault.BugCheck.code)}</p>
        <p class="p">{stopSentence(fault.BugCheck.code)}</p>
        {#if level !== "Simple" && fault.BugCheck.params.length}<p class="note mono">{fault.BugCheck.params.join(", ")}</p>{/if}
      {:else}
        {#each said(d, level).slice(0, 1) as line, i (i)}
          <p class="code">{line.main}</p>
          {#if line.sub}<p class="note mono">{line.sub}</p>{/if}
        {/each}
      {/if}
    </Block>
  </div>

  {#if d.stack_modules.length}
    <Block title={t("ui.dumps.said.stack")}>
      <p class="stack">
        {#each d.stack_modules as module, i (i)}
          {#if i}<Icon name="chevron" />{/if}<code>{module}</code>
        {/each}
      </p>
      <p class="note">{t("ui.dumps.detail.stack_note")}</p>
    </Block>
  {/if}

  <div class="cols2">
    <Block title={t("ui.occurrence.todo.title")}>
      <Steps steps={todo} />
    </Block>
    <Block title={t("ui.dumps.detail.file")}>
      <dl class="kv">
        <div><dt>{t("ui.dumps.detail.file")}</dt><dd><code class="code">{level === "Simple" ? fileName(d.path) : d.path}</code></dd></div>
        <div><dt>{t("ui.dumps.detail.when")}</dt><dd>{at(d.file_ms)}</dd></div>
        {#if level !== "Simple" && d.process}<div><dt>{t("ui.dumps.detail.process")}</dt><dd>{d.process}</dd></div>{/if}
      </dl>
      {#if fall !== null}
        <p class="more"><a href={href.occurrence(fall)}>{t("ui.dumps.detail.fall_open")}</a></p>
      {/if}
    </Block>
  </div>
{:else}
  <div class="unread">
    <Icon name="info" />
    <div>
      <p class="cz-n">
        {#if !d && !entry.report?.dump_path}
          {t("ui.dumps.detail.no_dump")}
        {:else if d && d.analyzed_ms !== null}
          {t("ui.dumps.detail.unreadable")}
        {:else}
          {t("ui.dumps.detail.unread_title")}
        {/if}
      </p>
      {#if entry.report}
        <p class="p">{t("ui.dumps.detail.reported", { code: hex(entry.report.code), what: stopSentence(entry.report.code) })}</p>
      {/if}
      {#if d && d.analyzed_ms === null && !gone}
        <p class="p">{t(level === "Technical" ? "ui.dumps.read.lede.tech" : "ui.dumps.read.lede")}</p>
        <div class="acts">
          <button type="button" class="btn accent" disabled={blocked || dumps.reading === "running"} onclick={() => void dumps.read()}>
            <Icon name="lock" />{t("ui.dumps.read.start")}
          </button>
          <AdminPill />
        </div>
      {/if}
    </div>
  </div>
{/if}

{#if d && !gone}
  <Block title={t("ui.dumps.delete.title")}>
    {#if asking}
      <div class="confirm" role="group" aria-label={t("ui.dumps.delete.title")}>
        <span>{t("ui.dumps.delete.ask_one", { size: formatBytes(d.size_bytes) })}</span>
        <button type="button" class="btn" disabled={blocked || dumps.deleting === "running"} onclick={() => void remove()}>
          <Icon name="trash" />{t("ui.dumps.delete.confirm")}
        </button>
        <button type="button" class="btn" onclick={() => (asking = false)}>{t("ui.dumps.delete.cancel")}</button>
      </div>
    {:else}
      <div class="acts">
        <button type="button" class="btn" disabled={blocked} onclick={() => (asking = true)}>
          <Icon name="trash" />{t("ui.dumps.delete.one")}
        </button>
        <span class="muted">{formatBytes(d.size_bytes)}</span>
      </div>
    {/if}
  </Block>
{/if}

<style>
  .cz-n {
    margin: 0;
    font: 600 16px/1.35 var(--f);
  }
  .cz-m {
    margin: 2px 0 10px;
    color: var(--ink2);
    font-size: 13px;
  }
  .code {
    margin: 0 0 4px;
    font: 13.5px/1.5 var(--mono);
  }
  .mono {
    font-family: var(--mono);
  }
  .stack {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
    margin: 0;
  }
  .stack code {
    padding: 4px 10px;
    border-radius: 6px;
    background: var(--hover);
    font: 13px var(--mono);
  }
  .stack :global(svg) {
    color: var(--ink3);
  }
  .kv {
    display: grid;
    grid-template-columns: max-content minmax(0, 1fr);
    gap: 8px 32px;
    margin: 0;
    font-size: 14px;
  }
  .kv div {
    display: contents;
  }
  .kv dt {
    color: var(--ink2);
  }
  .kv dd {
    min-width: 0;
    margin: 0;
    overflow-wrap: anywhere;
  }
  .unread {
    display: flex;
    gap: 16px;
    align-items: flex-start;
    max-width: 76ch;
    margin-top: 28px;
  }
  .unread > :global(svg) {
    flex: none;
    width: 24px;
    height: 24px;
    color: var(--ink2);
  }
  .unread .cz-n {
    margin-bottom: 8px;
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
</style>
