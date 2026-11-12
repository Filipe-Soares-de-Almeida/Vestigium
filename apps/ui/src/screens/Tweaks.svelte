<script lang="ts">
  import AdminPill from "../components/AdminPill.svelte";
  import BackLink from "../components/BackLink.svelte";
  import Block from "../components/Block.svelte";
  import ErrorView from "../components/ErrorView.svelte";
  import Icon from "../components/Icon.svelte";
  import Loading from "../components/Loading.svelte";
  import PageHead from "../components/PageHead.svelte";
  import RestorePointDialog from "../components/RestorePointDialog.svelte";
  import RowCard from "../components/RowCard.svelte";
  import RowList from "../components/RowList.svelte";
  import StatusMark from "../components/StatusMark.svelte";
  import { app } from "../lib/app.svelte";
  import { errorKind, errorText } from "../lib/errors";
  import { t, tn } from "../lib/i18n.svelte";
  import { tweakGroupOf } from "../lib/groups";
  import { relatedLinks } from "../lib/related.svelte";
  import { href, type TweakGroup } from "../lib/router.svelte";
  import { toast } from "../lib/toast.svelte";
  import { keyOf, type Outcome, tweaks } from "../lib/tweaks.svelte";
  import type { Advice, RestorePoint, TweakPick, TweakRow } from "../lib/types";
  import { at } from "./dumps/words";
  import { statusOf } from "./tests/status";
  import HelpedBars from "./tweaks/HelpedBars.svelte";
  import InterruptedNote from "./tweaks/InterruptedNote.svelte";
  import {
    againLine,
    changeOf,
    GROUP,
    ICON,
    inPlace,
    interruptedOf,
    interruptedWhen,
    metric,
    name,
    nowOf,
    openInterrupted,
    pointCase,
    restorePoint,
    risk,
    rowState,
    settleOf,
    shown,
    signal,
    unconfirmed,
    what,
    whyNot,
  } from "./tweaks/words";

  let { key, group }: { key: string | null; group: TweakGroup | null } = $props();

  const GROUPS = ["recommended", "quiet", "same", "done"] as const;

  let dialog = $state<HTMLDialogElement>();
  /** What the dialog asks: to apply these, to undo a row, or what to do about a missing restore point. */
  let asking = $state<
    | { kind: "apply"; items: Advice[] }
    | {
        kind: "no_point";
        point: RestorePoint;
        run: (without: boolean) => Promise<Outcome>;
        done: string;
        attempt: number;
        busy: boolean;
        note: string;
      }
    | { kind: "undo"; row: TweakRow; advice: Advice }
    | null
  >(null);

  const level = $derived(app.settings?.settings.detail_level ?? "Simple");
  const advice = $derived(tweaks.dto?.advice ?? []);
  const history = $derived(tweaks.dto?.history ?? []);
  const stateOf = (a: Advice) => shown(a, history);
  const withSignal = $derived(advice.filter((a) => stateOf(a) === "signal"));
  const quiet = $derived(advice.filter((a) => stateOf(a) === "quiet"));
  const settled = $derived(advice.filter((a) => !["signal", "quiet"].includes(stateOf(a))));
  const current = $derived(key === null ? null : (advice.find((a) => keyOf(a) === key) ?? null));
  const view = $derived<TweakGroup>(group ?? "recommended");
  const listed = $derived(view === "all" ? advice : advice.filter((a) => tweakGroupOf(stateOf(a)) === view));
  /** The list the open tweak belongs to now, so Back follows it when applying moves it. */
  const backTo = $derived<TweakGroup>(current ? tweakGroupOf(stateOf(current)) : view);
  const groupName = (g: TweakGroup) =>
    t(g === "recommended" ? "ui.item.recommended" : `ui.tweaks.nav.${g}`);
  const blocked = $derived(!app.running);
  const statusOfRow = (mark: Parameters<typeof statusOf>[0]) => statusOf(mark).kind;

  relatedLinks(() => [
    ...(view === "all" ? [{ label: t("ui.item.recommended"), href: href.tweaks() }] : [{ label: t("ui.tweaks.nav.all"), href: href.tweaks(null, "all") }]),
    { label: t("ui.health.title"), href: href.health() },
  ]);


  function retry(): void {
    tweaks.error = null;
    void tweaks.load();
  }

  const pickOf = (a: Advice): TweakPick => ({ tweak: a.tweak, subject: a.subject });

  function ask(next: NonNullable<typeof asking>): void {
    asking = next;
    if (next.kind !== "no_point" && !dialog?.open) dialog?.showModal();
  }

  function close(): void {
    dialog?.close();
    asking = null;
  }

  /** Runs a change; without a restore point it asks what to do. */
  async function carry(op: (without: boolean) => Promise<Outcome>, done: string): Promise<void> {
    close();
    const outcome = await op(false);
    if (outcome.kind === "no_point") {
      ask({ kind: "no_point", point: outcome.point, run: op, done, attempt: 1, busy: false, note: "" });
      return;
    }
    report(outcome, done);
  }

  /** Asks Windows for the point again with the dialog still open; a new failure updates it. */
  async function tryAgain(): Promise<void> {
    const now = asking;
    if (now?.kind !== "no_point" || now.busy) return;
    now.busy = true;
    now.note = "";
    const outcome = await now.run(false);
    if (asking !== now) return;
    if (outcome.kind === "no_point") {
      now.point = outcome.point;
      now.attempt += 1;
      now.note = againLine(pointCase(outcome.point), now.attempt);
      now.busy = false;
      return;
    }
    asking = null;
    report(outcome, now.done);
  }

  async function goWithout(): Promise<void> {
    const now = asking;
    if (now?.kind !== "no_point" || now.busy) return;
    asking = null;
    report(await now.run(true), now.done);
  }

  function giveUp(): void {
    if (asking?.kind !== "no_point") return;
    asking = null;
    toast.show(t("ui.tweaks.toast.nothing"));
  }

  function report(outcome: Outcome, done: string): void {
    if (outcome.kind === "declined") toast.show(t("ui.tweaks.toast.declined"));
    else if (outcome.kind === "failed")
      toast.show(errorKind(outcome.error) === "changed_since" ? errorText(outcome.error) : t("ui.tweaks.toast.failed"));
    else if (outcome.rows.some((r) => r.change.results.some((x) => x.error !== null))) toast.show(t("ui.tweaks.toast.partial"));
    else toast.show(done);
  }

  function applied(items: Advice[]): string {
    const later = items.filter((a) => a.takes !== "Now").length;
    return tn("ui.tweaks.toast.applied", items.length) + (later ? ` · ${tn("ui.tweaks.toast.later", later)}` : "");
  }

  async function confirmed(): Promise<void> {
    const now = asking;
    if (now?.kind === "apply") {
      await carry((without) => tweaks.apply(now.items.map(pickOf), without), applied(now.items));
    } else if (now?.kind === "undo") {
      await undo(now.row, now.advice);
    }
  }

  /** Puts back what a change wrote, and says so; settings the helper never confirmed are left alone. */
  async function undo(row: TweakRow, a: Advice): Promise<void> {
    const back = (changeOf(a, row.change.results.map((r) => r.change), "Simple")[0]?.from ?? "").toLocaleLowerCase();
    const left = row.change.results.some((r) => settleOf(r) === "unknown") ? ` · ${t("ui.tweaks.toast.undone.left")}` : "";
    await carry((without) => tweaks.undo(row.id, without), t("ui.tweaks.toast.undone", { name: name(a, "Simple"), value: back }) + left);
  }

  /** When a change was made, or stopped, as the page words it. */
  const whenText = (row: TweakRow) =>
    interruptedOf(row) === null ? t("ui.tweaks.sub.applied", { when: at(row.applied_ms) }) : interruptedWhen(row);

  function sub(a: Advice): string {
    const s = stateOf(a);
    if (s === "signal") return signal(a.signals[0]);
    if (s === "quiet") return whyNot(a);
    if (s === "applied") {
      const held = inPlace(a, history);
      return held ? whenText(held) : "";
    }
    if (s === "set") return t("ui.tweaks.status.set");
    return t(`ui.tweaks.sub.${s}`);
  }

  /** The change a tweak makes, "On → Off", or the value it holds when it makes none. */
  function changeText(a: Advice): string {
    const c = changeOf(a, a.changes, "Simple")[0];
    return c ? `${c.from} → ${c.to}` : nowOf(a, "Simple");
  }

  const rowsOf = (a: Advice) => history.filter((r) => r.change.tweak === a.tweak && r.change.subject === a.subject);
</script>

{#snippet rows(list: Advice[])}
  {#if list.length}
    <RowList label={t("ui.tweaks.list")}>
      {#each list as a (keyOf(a))}
        <RowCard href={href.tweaks(keyOf(a))} icon={ICON[GROUP[a.tweak]]} title={name(a, level)} sub={sub(a)}>
          {#snippet right()}<span>{changeText(a)}</span>{/snippet}
        </RowCard>
      {/each}
    </RowList>
  {:else}
    <p class="empty">{t(view === "recommended" ? "ui.tweaks.lede.none" : "ui.tweaks.group_empty")}</p>
  {/if}
{/snippet}

{#snippet restore()}
  <p class="restore">
    <Icon name="restart" />
    <span>
      {tweaks.dto?.last_restore_point_ms
        ? t("ui.tweaks.point.last", { when: at(tweaks.dto.last_restore_point_ms) })
        : t("ui.tweaks.point.none")}
    </span>
  </p>
{/snippet}

{#snippet lede()}
  <p class="lede">
    {#if level === "Technical"}
      {t("ui.tweaks.lede.tech", { signal: withSignal.length, quiet: quiet.length, settled: settled.length })}
    {:else if withSignal.length}
      <b>{tn("ui.tweaks.lede.signals", withSignal.length)}</b> {t("ui.tweaks.lede.signals.tail")}
    {:else if history.some((r) => r.undone_ms === null && r.change.undo_of === null)}
      <b>{t("ui.tweaks.lede.done")}</b> {t("ui.tweaks.lede.done.tail")}
    {:else}
      <b>{t("ui.tweaks.lede.none")}</b> {t("ui.tweaks.lede.none.tail")}
    {/if}
  </p>
{/snippet}

{#if tweaks.dto === null && tweaks.error !== null}
  <PageHead title={t("ui.tweaks.title")} />
  <div class="gap"><ErrorView error={tweaks.error} heading="h2" onretry={retry} /></div>
{:else if tweaks.dto === null}
  <PageHead title={t("ui.tweaks.title")} />
  <Loading />
{:else if key !== null}
  <BackLink href={href.tweaks(null, backTo)} label={groupName(backTo)} />
  {#if current}
    {@const s = stateOf(current)}
    {@const rowsHere = rowsOf(current)}
    {@const holding = inPlace(current, history)}
    {@const stopped = openInterrupted(rowsHere)}
    {@const unconf = stopped !== null && interruptedOf(stopped) === "unknown" && holding === null}
    {@const change = changeOf(current, current.changes, level).map((c, i) =>
      unconf ? { ...c, from: unconfirmed(current.changes[i]?.setting ?? current.changes[0].setting, level) } : c,
    )}
    <PageHead title={name(current, level)} sub="{groupName(backTo)} · {stopped ? interruptedWhen(stopped) : sub(current)}" />
    <div class="cols2">
      <div>
        <Block title={t("ui.tweaks.section.what")}>
          <p class="p">{what(current, level)}</p>
        </Block>
        {#if s === "signal"}
          <Block title={t("ui.tweaks.why")}>
            <ul class="ev-l">
              {#each current.signals as sig, i (i)}<li>{signal(sig)}</li>{/each}
            </ul>
          </Block>
        {:else if s === "quiet"}
          <Block title={t("ui.tweaks.why_not")}>
            <p class="p">{whyNot(current)}</p>
          </Block>
        {:else if s === "applied"}
          <Block title={t("ui.tweaks.section.applied")}>
            <p class="p">{holding ? whenText(holding) : ""}</p>
          </Block>
        {:else}
          <Block title={t("ui.tweaks.section.state")}>
            <p class="p">{t(`ui.tweaks.note.${s}`)}</p>
          </Block>
        {/if}
      </div>

      <Block title={t("ui.tweaks.section.change")}>
        {#if stopped}
          {#key stopped.id}
            <InterruptedNote
              row={stopped}
              advice={current}
              {level}
              disabled={blocked || tweaks.busy}
              onundo={() => void undo(stopped, current)}
            />
          {/key}
        {/if}
        {#if change.length}
          {#each change as c, i (i)}
            <div class="chg">
              <div><span class="chg-l">{t(s === "applied" ? "ui.tweaks.fact.before" : "ui.tweaks.fact.now")}</span><b>{c.from}</b></div>
              <Icon name="chevron" />
              <div><span class="chg-l">{t(s === "applied" ? "ui.tweaks.fact.since" : "ui.tweaks.fact.after")}</span><b>{c.to}</b></div>
            </div>
          {/each}
        {:else if nowOf(current, level)}
          <div class="chg"><div><span class="chg-l">{t("ui.tweaks.fact.now")}</span><b>{nowOf(current, level)}</b></div></div>
        {/if}
        {#if s === "signal" || s === "quiet"}
          <dl class="kv">
            <div><dt>{t("ui.tweaks.fact.when")}</dt><dd>{t(`ui.tweaks.takes.${current.takes}`)}</dd></div>
            <div><dt>{t("ui.tweaks.fact.permission")}</dt><dd>{t(`ui.tweaks.applier.${current.applier}`)}</dd></div>
            <div><dt>{t("ui.tweaks.fact.cost")}</dt><dd>{risk(current)}</dd></div>
          </dl>
        {/if}
        {#if holding && stopped?.id === holding.id}
          <div class="acts"><span class="muted">{whenText(holding)}</span></div>
        {:else if holding}
          <div class="acts">
            <button type="button" class="btn" disabled={blocked || tweaks.busy} onclick={() => ask({ kind: "undo", row: holding, advice: current })}>
              <Icon name="undo" />{t("ui.tweaks.undo")}
            </button>
            <span class="muted">{whenText(holding)}</span>
          </div>
        {:else if current.status === "Ready" && s !== "set"}
          <div class="acts">
            <button
              type="button"
              class="btn"
              class:accent={s === "signal"}
              disabled={blocked || tweaks.busy}
              onclick={() => ask({ kind: "apply", items: [current] })}
            >
              {t(tweaks.busy ? "ui.tweaks.applying" : "ui.tweaks.apply_one")}
            </button>
            {#if current.applier === "Elevated"}<AdminPill />{:else}<span class="muted">{t(`ui.tweaks.applier.${current.applier}`)}</span>{/if}
          </div>
          {#if s === "quiet"}<p class="note">{t("ui.tweaks.quiet_hint")}</p>{/if}
          {#if blocked}<p class="note">{t("ui.tweaks.blocked")}</p>{/if}
        {:else if s === "set"}
          <p class="muted">{t("ui.tweaks.nothing", { value: nowOf(current, "Simple").toLocaleLowerCase() })}</p>
        {/if}
      </Block>
    </div>

    {#if rowsHere.length}
      <Block title={t("ui.tweaks.history")}>
        {#each rowsHere as r (r.id)}
          {@const st = rowState(r)}
          {@const lines = changeOf(current, r.change.results.map((x) => x.change), level)}
          <div class="hrow">
            <p class="hline">
              <span class="d">{at(r.applied_ms)}</span>
              <span>{#each lines as c, i (i)}<span class="chg-t">{c.from} → {c.to}</span>{/each}</span>
              <StatusMark kind={statusOfRow(st.mark)} word={st.text} />
            </p>
            {#if r.outside && current.status === "Ready" && !(holding && r.id === holding.id)}
              <div class="acts">
                <button type="button" class="btn" disabled={blocked || tweaks.busy} onclick={() => ask({ kind: "apply", items: [current] })}>
                  <Icon name="tweaks" />{t("ui.tweaks.reapply")}
                </button>
              </div>
            {/if}
            {#if r.helped && r.undone_ms === null && r.change.undo_of === null}
              <HelpedBars helped={r.helped} metric={metric(current.tweak)} />
            {/if}
            {#if level === "Technical" && r.change.restore_point}
              <small class="mono">{restorePoint(r.change.restore_point)}</small>
            {/if}
          </div>
        {/each}
      </Block>
    {/if}
  {:else}
    <PageHead title={t("ui.tweaks.title")} />
    <p class="lede">{t("ui.tweaks.empty")}</p>
  {/if}
{:else if view === "recommended"}
  <PageHead title={t("ui.tweaks.title")} />
  {@render lede()}
  {@render restore()}
  <Block title={t("ui.tweaks.group.recommended")}>
    {@render rows(listed)}
  </Block>
{:else if view === "all"}
  <PageHead title={t("ui.tweaks.nav.all")} sub={withSignal.length ? tn("ui.tweaks.lede.signals", withSignal.length) : t("ui.tweaks.lede.none")} />
  {@render restore()}
  {#each GROUPS as g (g)}
    {@const list = advice.filter((a) => tweakGroupOf(stateOf(a)) === g)}
    <Block title="{groupName(g)} ({list.length})">
      {#if list.length}
        <RowList>
          {#each list as a (keyOf(a))}
            <RowCard href={href.tweaks(keyOf(a))} icon={ICON[GROUP[a.tweak]]} title={name(a, level)} sub={sub(a)}>
              {#snippet right()}<span>{changeText(a)}</span>{/snippet}
            </RowCard>
          {/each}
        </RowList>
      {:else}
        <p class="empty">{t("ui.tweaks.group_empty")}</p>
      {/if}
    </Block>
  {/each}
{:else}
  <PageHead title={groupName(view)} sub={t(`ui.tweaks.desc.${view}`)} />
  <section class="list">{@render rows(listed)}</section>
{/if}

{#if tweaks.dto !== null}
  <dialog class="confirm" bind:this={dialog} aria-labelledby="ask-title" onclose={() => { if (asking?.kind !== "no_point") asking = null; }}>
    {#if asking?.kind === "apply"}
      {@const admin = asking.items.some((a) => a.applier === "Elevated")}
      <h2 id="ask-title">{tn("ui.tweaks.ask", asking.items.length)}</h2>
      <ul>
        {#each asking.items as a (keyOf(a))}
          {@const c = changeOf(a, a.changes, "Simple")[0]}
          <li>
            <b>{name(a, "Simple")}</b>
            {#if c}<span class="chg-t">{c.from} → {c.to}</span>{/if}
            <span class="tags">
              <span class="tag">{t(`ui.tweaks.applier.${a.applier}`)}</span>
              <span class="tag">{t(`ui.tweaks.takes.${a.takes}`)}</span>
            </span>
          </li>
        {/each}
      </ul>
      <p>{admin ? `${t("ui.tweaks.ask.point")} ` : ""}{t("ui.tweaks.ask.kept")}{admin ? ` ${t("ui.tweaks.ask.prompt")}` : ""}</p>
      <div class="acts end">
        <button type="button" class="btn" onclick={close}>{t("ui.tweaks.cancel")}</button>
        <button type="button" class="btn accent" onclick={() => void confirmed()}>{t("ui.tweaks.bar.apply")}</button>
      </div>
    {:else if asking?.kind === "undo"}
      {@const back = (changeOf(asking.advice, asking.row.change.results.map((r) => r.change), "Simple")[0]?.from ?? "").toLocaleLowerCase()}
      <h2 id="ask-title">{t("ui.tweaks.undo.title")}</h2>
      <p>
        {t("ui.tweaks.undo.body", { name: name(asking.advice, "Simple"), value: back })}{asking.row.applier === "Elevated" ? ` ${t("ui.tweaks.ask.prompt")}` : ""}
      </p>
      <div class="acts end">
        <button type="button" class="btn" onclick={close}>{t("ui.tweaks.cancel")}</button>
        <button type="button" class="btn accent" onclick={() => void confirmed()}><Icon name="undo" />{t("ui.tweaks.undo")}</button>
      </div>
    {/if}
  </dialog>
  {#if asking?.kind === "no_point"}
    <RestorePointDialog
      point={asking.point}
      attempt={asking.attempt}
      busy={asking.busy}
      note={asking.note}
      onretry={() => void tryAgain()}
      ongo={() => void goWithout()}
      oncancel={giveUp}
    />
  {/if}
{/if}

<style>
  .gap {
    margin-top: 14px;
  }
  .restore {
    display: flex;
    gap: 10px;
    align-items: flex-start;
    max-width: 74ch;
    margin: 12px 0 0;
    color: var(--ink2);
    font-size: 13px;
  }
  .restore :global(svg) {
    flex: none;
    margin-top: 2px;
  }
  .list {
    margin-top: 32px;
    padding-top: 28px;
    border-top: 1px solid var(--rule);
  }
  .chg {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 14px;
    margin-bottom: 18px;
  }
  .chg > div {
    display: grid;
    gap: 2px;
    min-width: 150px;
    padding: 10px 14px;
    border: 1px solid var(--stroke);
    border-radius: 8px;
  }
  .chg > :global(svg) {
    color: var(--ink3);
  }
  .chg-l {
    color: var(--ink2);
    font-size: 12.5px;
  }
  .chg b {
    font-size: 15px;
    font-weight: 600;
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
  }
  .muted {
    font-size: 13px;
  }
  .hrow + .hrow {
    margin-top: 10px;
    padding-top: 10px;
    border-top: 1px solid var(--stroke2);
  }
  .hline {
    display: grid;
    grid-template-columns: 128px minmax(0, 1fr) auto;
    gap: 16px;
    align-items: center;
    margin: 0;
    font-size: 14px;
  }
  .d {
    color: var(--ink2);
    font-size: 13px;
  }
  .chg-t + .chg-t::before {
    content: " · ";
    color: var(--ink3);
  }
  .mono {
    display: block;
    margin-top: 6px;
    color: var(--ink2);
    font: 12.5px var(--mono);
  }
  .tags {
    display: inline-flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .tag {
    display: inline-block;
    padding: 2px 10px;
    border-radius: 999px;
    background: var(--hover);
    color: var(--ink2);
    font-size: 12px;
    white-space: nowrap;
  }
  .confirm {
    max-width: min(560px, calc(100vw - 32px));
    padding: 18px 20px;
    border: 1px solid var(--stroke);
    border-radius: 8px;
    background: var(--card);
    color: var(--ink);
    box-shadow: var(--shadow-float);
  }
  .confirm::backdrop {
    background: #0008;
  }
  .confirm h2 {
    margin: 0 0 10px;
  }
  .confirm ul {
    display: grid;
    gap: 10px;
    margin: 0 0 12px;
    padding: 0;
    list-style: none;
    font-size: 14px;
  }
  .confirm li {
    display: grid;
    gap: 4px;
  }
  .confirm p {
    margin: 0 0 12px;
    color: var(--ink2);
    font-size: 14px;
    line-height: 1.5;
  }
  .acts.end {
    justify-content: flex-end;
  }
</style>
