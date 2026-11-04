<script lang="ts">
  import ConcernMark from "../components/ConcernMark.svelte";
  import ErrorView from "../components/ErrorView.svelte";
  import Icon from "../components/Icon.svelte";
  import Loading from "../components/Loading.svelte";
  import PickAll from "../components/PickAll.svelte";
  import { app } from "../lib/app.svelte";
  import { t, tn } from "../lib/i18n.svelte";
  import { go, href } from "../lib/router.svelte";
  import { toast } from "../lib/toast.svelte";
  import { keyOf, type Outcome, tweaks } from "../lib/tweaks.svelte";
  import type { Advice, TweakPick, TweakRow } from "../lib/types";
  import { at } from "./dumps/words";
  import {
    changeOf,
    GROUP,
    helpedVerdict,
    ICON,
    inPlace,
    metric,
    name,
    nowOf,
    restorePoint,
    risk,
    rowState,
    type Shown,
    shown,
    signal,
    TONE,
    what,
    whyNot,
  } from "./tweaks/words";

  let { key }: { key: string | null } = $props();

  let detail = $state<HTMLElement>();

  // Another tweak opens at the top of its detail.
  $effect(() => {
    void key;
    detail?.scrollTo(0, 0);
  });

  const MARK: Record<Shown, "Recommended" | "Unknown" | "Ok" | "Closed"> = {
    signal: "Recommended",
    quiet: "Unknown",
    applied: "Ok",
    set: "Ok",
    unavailable: "Unknown",
    protected: "Closed",
  };

  let dialog = $state<HTMLDialogElement>();
  /** What the dialog asks: to apply these, to go on without a restore point, or to undo a row. */
  let asking = $state<
    | { kind: "apply"; items: Advice[] }
    | { kind: "no_point"; retry: () => Promise<Outcome>; items: Advice[] }
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
  /** Only a tweak with a sign on this PC can be chosen with others; the rest apply one at a time. */
  const choosable = (a: Advice) => a.status === "Ready" && stateOf(a) === "signal";

  // A choice that lost its sign leaves the bar.
  $effect(() => {
    const allowed = new Set(withSignal.filter((a) => a.status === "Ready").map(keyOf));
    if (tweaks.chosen.some((k) => !allowed.has(k))) tweaks.chosen = tweaks.chosen.filter((k) => allowed.has(k));
  });
  const current = $derived(
    advice.find((a) => keyOf(a) === key) ?? withSignal[0] ?? quiet[0] ?? advice[0] ?? null,
  );
  const chosen = $derived(advice.filter((a) => tweaks.chosen.includes(keyOf(a))));
  const blocked = $derived(!app.running);


  function retry(): void {
    tweaks.error = null;
    void tweaks.load();
  }

  const pickOf = (a: Advice): TweakPick => ({ tweak: a.tweak, subject: a.subject });

  function ask(next: NonNullable<typeof asking>): void {
    // The tweak in view stays in view once applied, so its history shows beside it.
    if (key === null && current) go(href.tweaks(keyOf(current)));
    asking = next;
    if (!dialog?.open) dialog?.showModal();
  }

  function close(): void {
    dialog?.close();
    asking = null;
  }

  /** Runs a change; without a restore point it asks before trying again. */
  async function carry(items: Advice[], op: (without: boolean) => Promise<Outcome>, done: string): Promise<void> {
    close();
    const outcome = await op(false);
    if (outcome.kind === "no_point") {
      ask({ kind: "no_point", retry: () => op(true), items });
      return;
    }
    report(outcome, done);
  }

  function report(outcome: Outcome, done: string): void {
    if (outcome.kind === "declined") toast.show(t("ui.tweaks.toast.declined"));
    else if (outcome.kind === "failed") toast.show(t("ui.tweaks.toast.failed"));
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
      tweaks.chosen = tweaks.chosen.filter((k) => !now.items.some((a) => keyOf(a) === k));
      await carry(now.items, (without) => tweaks.apply(now.items.map(pickOf), without), applied(now.items));
    } else if (now?.kind === "no_point") {
      close();
      report(await now.retry(), applied(now.items));
    } else if (now?.kind === "undo") {
      const back = (changeOf(now.advice, now.row.change.results.map((r) => r.change), "Simple")[0]?.from ?? "").toLocaleLowerCase();
      await carry([now.advice], (without) => tweaks.undo(now.row.id, without), t("ui.tweaks.toast.undone", { name: name(now.advice, "Simple"), value: back }));
    }
  }

  function sub(a: Advice): string {
    const s = stateOf(a);
    if (s === "signal") return signal(a.signals[0]);
    if (s === "quiet") {
      const c = changeOf(a, a.changes, "Simple")[0];
      return c ? `${c.from} → ${c.to}` : "";
    }
    if (s === "applied") return t("ui.tweaks.sub.applied", { when: at(inPlace(a, history)?.applied_ms ?? 0) });
    if (s === "set") return nowOf(a, "Simple");
    return t(`ui.tweaks.sub.${s}`);
  }

  const rowsOf = (a: Advice) => history.filter((r) => r.change.tweak === a.tweak && r.change.subject === a.subject);
</script>

{#snippet item(a: Advice)}
  {@const s = stateOf(a)}
  {@const tone = TONE[GROUP[a.tweak]]}
  <div class="row" class:current={current && keyOf(a) === keyOf(current)}>
    {#if choosable(a)}
      <input
        type="checkbox"
        class="pick"
        checked={tweaks.chosen.includes(keyOf(a))}
        aria-label={t("ui.tweaks.pick", { name: name(a, "Simple") })}
        onchange={(event) => tweaks.choose(keyOf(a), event.currentTarget.checked)}
      />
    {:else}
      <span></span>
    {/if}
    <span
      class="icochip"
      style:background={s === "signal" ? `var(--${tone}-t)` : "var(--card2)"}
      style:color={s === "signal" ? `var(--${tone})` : "var(--ink2)"}
    >
      <Icon name={ICON[GROUP[a.tweak]]} />
    </span>
    <button
      type="button"
      class="open"
      aria-current={current && keyOf(a) === keyOf(current) ? "true" : undefined}
      onclick={() => go(href.tweaks(keyOf(a)))}
    >
      <span class="what"><b>{name(a, level)}</b><small>{sub(a)}</small></span>
      <span class="chev"><Icon name="chevron" /></span>
    </button>
  </div>
{/snippet}

{#snippet group(title: string, list: Advice[])}
  {#if list.length}
    {@const pickable = list.filter(choosable).map(keyOf)}
    <div class="ghead">
      {#if pickable.length}
        <PickAll
          ids={pickable}
          chosen={tweaks.chosen}
          label={t("ui.tweaks.pick_all", { group: title })}
          onchange={(on) => tweaks.chooseAll(pickable, on)}
        />
      {/if}
      <h2>{title}</h2>
    </div>
    {#each list as a (keyOf(a))}
      {@render item(a)}
    {/each}
  {/if}
{/snippet}

<div class="top">
  <h1>{t("ui.tweaks.title")}</h1>
</div>

{#if tweaks.dto === null && tweaks.error !== null}
  <div class="gap"><ErrorView error={tweaks.error} heading="h2" onretry={retry} /></div>
{:else if tweaks.dto === null}
  <Loading />
{:else}
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

  <div class="pane2 panes">
    <div class="stack scrolls">
      <section class="card list" aria-label={t("ui.tweaks.list")}>
        {@render group(t("ui.tweaks.group.signal"), withSignal)}
        {@render group(t("ui.tweaks.group.quiet"), quiet)}
        {@render group(t("ui.tweaks.group.settled"), settled)}
      </section>
      <section class="card point" aria-labelledby="point-title">
        <h2 id="point-title">{t("ui.tweaks.point.title")}</h2>
        <p>
          {tweaks.dto.last_restore_point_ms
            ? t("ui.tweaks.point.last", { when: at(tweaks.dto.last_restore_point_ms) })
            : t("ui.tweaks.point.none")}
        </p>
      </section>
    </div>

    {#if current}
      {@const s = stateOf(current)}
      {@const tone = TONE[GROUP[current.tweak]]}
      {@const change = changeOf(current, current.changes, level)}
      {@const rows = rowsOf(current)}
      {@const holding = inPlace(current, history)}
      <section class="card detail scrolls" aria-labelledby="tweak-title" bind:this={detail}>
        <div class="head">
          <span
            class="icochip"
            style:background={s === "signal" ? `var(--${tone}-t)` : "var(--card2)"}
            style:color={s === "signal" ? `var(--${tone})` : "var(--ink2)"}
          >
            <Icon name={ICON[GROUP[current.tweak]]} />
          </span>
          <div>
            <h2 id="tweak-title">{name(current, level)}</h2>
            <p class="meta"><ConcernMark mark={MARK[s]} word={false} silent /><span>{t(`ui.tweaks.status.${s}`)}</span></p>
          </div>
        </div>
        <p class="about">{what(current, level)}</p>

        {#if s === "signal"}
          <div>
            <h3>{t("ui.tweaks.why")}</h3>
            <ul class="ev">
              {#each current.signals as sig, i (i)}
                <li><span class="bullet" class:first={i === 0} aria-hidden="true"></span><span>{signal(sig)}</span></li>
              {/each}
            </ul>
          </div>
        {:else if s === "quiet"}
          <div>
            <h3>{t("ui.tweaks.why_not")}</h3>
            <p class="plain">{whyNot(current)}</p>
          </div>
        {:else if s !== "applied"}
          <p class="plain">{t(`ui.tweaks.note.${s}`)}</p>
        {/if}

        {#if change.length || nowOf(current, level)}
          <dl class="facts">
            {#if change.length}
              {#each change as c, i (i)}
                <dt>{i === 0 ? t("ui.tweaks.fact.now") : ""}</dt>
                <dd>{c.from}</dd>
                <dt>{i === 0 ? t("ui.tweaks.fact.after") : ""}</dt>
                <dd><b>{c.to}</b></dd>
              {/each}
            {:else}
              <dt>{t("ui.tweaks.fact.now")}</dt>
              <dd>{nowOf(current, level)}</dd>
            {/if}
            {#if s === "signal" || s === "quiet"}
              <dt>{t("ui.tweaks.fact.when")}</dt>
              <dd><span class="pill2"><Icon name={current.takes === "Now" ? "clock" : "restart"} />{t(`ui.tweaks.takes.${current.takes}`)}</span></dd>
              <dt>{t("ui.tweaks.fact.permission")}</dt>
              <dd><span class="pill2"><Icon name={current.applier === "Elevated" ? "shield" : "person"} />{t(`ui.tweaks.applier.${current.applier}`)}</span></dd>
              <dt>{t("ui.tweaks.fact.cost")}</dt>
              <dd>{risk(current)}</dd>
            {/if}
          </dl>
        {/if}

        {#if current.status === "Ready"}
          <div class="acts">
            <button
              type="button"
              class="btn"
              class:accent={s === "signal" && tweaks.chosen.length === 0}
              disabled={blocked || tweaks.busy}
              onclick={() => ask({ kind: "apply", items: [current] })}
            >
              <Icon name="tweaks" />{t(
                tweaks.busy ? "ui.tweaks.applying" : tweaks.chosen.length ? "ui.tweaks.apply_only" : "ui.tweaks.apply_one",
              )}
            </button>
            {#if s === "quiet"}<span class="hint">{t("ui.tweaks.quiet_hint")}</span>{/if}
            {#if blocked}<span class="hint">{t("ui.tweaks.blocked")}</span>{/if}
          </div>
        {/if}

        {#if rows.length}
          <div class="hist">
            <h3>{t("ui.tweaks.history")}</h3>
            {#each rows as r (r.id)}
              {@const st = rowState(r)}
              {@const lines = changeOf(current, r.change.results.map((x) => x.change), level)}
              <div class="hrow">
                <p class="hline">
                  <b>{at(r.applied_ms)}</b>
                  {#each lines as c, i (i)}<span class="chg">{c.from} → {c.to}</span>{/each}
                </p>
                <div class="hstate">
                  <span class="meta"><ConcernMark mark={st.mark} word={false} silent /><span>{st.text}</span></span>
                  {#if holding && r.id === holding.id}
                    <button type="button" class="btn" disabled={blocked || tweaks.busy} onclick={() => ask({ kind: "undo", row: r, advice: current })}>
                      <Icon name="undo" />{t("ui.tweaks.undo")}
                    </button>
                  {:else if r.outside && current.status === "Ready"}
                    <button type="button" class="btn" disabled={blocked || tweaks.busy} onclick={() => ask({ kind: "apply", items: [current] })}>
                      <Icon name="tweaks" />{t("ui.tweaks.reapply")}
                    </button>
                  {/if}
                </div>
                {#if r.helped && r.undone_ms === null && r.change.undo_of === null}
                  {@const h = r.helped}
                  {@const max = Math.max(h.before, h.after, 1)}
                  <div
                    class="helped"
                    role="img"
                    aria-label={t("ui.tweaks.helped.label", { metric: metric(current.tweak), before: h.before, after: h.after, days: h.days })}
                  >
                    <div><span>{t("ui.tweaks.helped.before")}</span><i style:width="{Math.max((h.before / max) * 100, 3)}%"></i><b>{h.before}</b></div>
                    <div><span>{t("ui.tweaks.helped.after")}</span><i class="after" style:width="{Math.max((h.after / max) * 100, 3)}%"></i><b>{h.after}</b></div>
                    <small>{helpedVerdict(h.before, h.after)} · {tn("ui.tweaks.helped.span", h.days, { metric: metric(current.tweak) })}</small>
                  </div>
                {/if}
                {#if level === "Technical" && r.change.restore_point}
                  <small class="mono">{restorePoint(r.change.restore_point)}</small>
                {/if}
              </div>
            {/each}
          </div>
        {/if}
      </section>
    {/if}
  </div>

  {#if chosen.length}
    {@const admin = chosen.filter((a) => a.applier === "Elevated").length}
    {@const restart = chosen.filter((a) => a.takes !== "Now").length}
    <div class="delbar" role="region" aria-label={t("ui.tweaks.bar")}>
      <p>
        {tn("ui.tweaks.bar.chosen", chosen.length)}
        <small>
          {admin ? tn("ui.tweaks.bar.admin", admin) : t("ui.tweaks.bar.no_admin")}{restart ? ` · ${tn("ui.tweaks.bar.later", restart)}` : ""}
        </small>
      </p>
      <div class="acts">
        <button type="button" class="btn" onclick={() => (tweaks.chosen = [])}>{t("ui.tweaks.bar.clear")}</button>
        <button
          type="button"
          class="btn accent"
          disabled={blocked || tweaks.busy}
          onclick={() => ask({ kind: "apply", items: chosen })}
        >
          <Icon name="tweaks" />{t(tweaks.busy ? "ui.tweaks.applying" : "ui.tweaks.bar.apply")}
        </button>
      </div>
    </div>
  {/if}

  <dialog class="confirm" bind:this={dialog} aria-labelledby="ask-title" onclose={() => (asking = null)}>
    {#if asking?.kind === "apply"}
      {@const admin = asking.items.some((a) => a.applier === "Elevated")}
      <h2 id="ask-title">{tn("ui.tweaks.ask", asking.items.length)}</h2>
      <ul>
        {#each asking.items as a (keyOf(a))}
          {@const c = changeOf(a, a.changes, "Simple")[0]}
          <li>
            <b>{name(a, "Simple")}</b>
            {#if c}<span class="chg">{c.from} → {c.to}</span>{/if}
            <span class="pills">
              <span class="pill2"><Icon name={a.applier === "Elevated" ? "shield" : "person"} />{t(`ui.tweaks.applier.${a.applier}`)}</span>
              <span class="pill2"><Icon name={a.takes === "Now" ? "clock" : "restart"} />{t(`ui.tweaks.takes.${a.takes}`)}</span>
            </span>
          </li>
        {/each}
      </ul>
      <p>{admin ? `${t("ui.tweaks.ask.point")} ` : ""}{t("ui.tweaks.ask.kept")}{admin ? ` ${t("ui.tweaks.ask.prompt")}` : ""}</p>
      <div class="acts">
        <button type="button" class="btn" onclick={close}>{t("ui.tweaks.cancel")}</button>
        <button type="button" class="btn accent" onclick={() => void confirmed()}><Icon name={admin ? "shield" : "tweaks"} />{t("ui.tweaks.bar.apply")}</button>
      </div>
    {:else if asking?.kind === "no_point"}
      <h2 id="ask-title">{t("ui.tweaks.no_point.title")}</h2>
      <p>{t("ui.tweaks.no_point.body")}</p>
      <div class="acts">
        <button type="button" class="btn" onclick={() => { close(); toast.show(t("ui.tweaks.toast.nothing")); }}>{t("ui.tweaks.cancel")}</button>
        <button type="button" class="btn accent" onclick={() => void confirmed()}>{t("ui.tweaks.no_point.go")}</button>
      </div>
    {:else if asking?.kind === "undo"}
      {@const back = (changeOf(asking.advice, asking.row.change.results.map((r) => r.change), "Simple")[0]?.from ?? "").toLocaleLowerCase()}
      <h2 id="ask-title">{t("ui.tweaks.undo.title")}</h2>
      <p>
        {t("ui.tweaks.undo.body", { name: name(asking.advice, "Simple"), value: back })}{asking.row.applier === "Elevated" ? ` ${t("ui.tweaks.ask.prompt")}` : ""}
      </p>
      <div class="acts">
        <button type="button" class="btn" onclick={close}>{t("ui.tweaks.cancel")}</button>
        <button type="button" class="btn accent" onclick={() => void confirmed()}><Icon name={asking.row.applier === "Elevated" ? "shield" : "undo"} />{t("ui.tweaks.undo")}</button>
      </div>
    {/if}
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
  .pane2 {
    display: grid;
    grid-template-columns: minmax(0, 0.9fr) minmax(0, 1.3fr);
    gap: 14px;
    align-items: start;
  }
  .list {
    padding: 6px;
  }
  .ghead {
    display: flex;
    gap: 10px;
    align-items: center;
    padding: 10px 10px 4px;
  }
  .list h2 {
    margin: 0;
    color: var(--ink2);
    font: 600 13px/1.3 var(--f);
  }
  .row {
    display: grid;
    grid-template-columns: 22px 32px minmax(0, 1fr);
    gap: 10px;
    align-items: center;
    padding: 8px 10px;
    border-radius: 8px;
  }
  .row:hover,
  .row.current {
    background: var(--card2);
  }
  .row .icochip {
    width: 32px;
    height: 32px;
  }
  .row .icochip :global(svg) {
    width: 18px;
    height: 18px;
  }
  .open {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 12px;
    gap: 10px;
    align-items: center;
    min-width: 0;
    padding: 0;
    border: 0;
    border-radius: 6px;
    background: none;
    color: inherit;
    font: inherit;
    text-align: left;
    cursor: pointer;
  }
  .what b {
    display: block;
    font-weight: 600;
    line-height: 1.3;
  }
  .what small {
    display: block;
    margin-top: 2px;
    color: var(--ink2);
    font-size: 13px;
    line-height: 1.35;
  }
  .chev {
    display: grid;
    color: var(--ink3);
  }
  .pick {
    display: grid;
    place-items: center;
    width: 20px;
    height: 20px;
    margin: 0;
    border: 1.5px solid var(--ink3);
    border-radius: 6px;
    background: var(--card);
    appearance: none;
    cursor: pointer;
  }
  .pick:checked {
    border-color: var(--accent);
    background: var(--accent);
  }
  .pick:checked::after {
    content: "";
    width: 9px;
    height: 5px;
    border-bottom: 2px solid var(--on-accent);
    border-left: 2px solid var(--on-accent);
    transform: rotate(-45deg) translate(1px, -1px);
  }
  .point h2 {
    margin: 0 0 6px;
    font: 600 15px/1.3 var(--f);
  }
  .point p {
    margin: 0;
    color: var(--ink2);
    font-size: 13.5px;
    line-height: 1.45;
  }
  .detail {
    display: grid;
    gap: 14px;
  }
  .head {
    display: grid;
    grid-template-columns: 40px minmax(0, 1fr);
    gap: 12px;
    align-items: center;
  }
  .head h2 {
    margin: 0;
    font: 600 20px/1.3 var(--fd);
  }
  .meta {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    align-items: center;
    margin: 4px 0 0;
    color: var(--ink2);
    font-size: 13px;
  }
  .about,
  .plain {
    max-width: 70ch;
    margin: 0;
    line-height: 1.5;
  }
  .plain {
    color: var(--ink2);
  }
  h3 {
    margin: 0 0 8px;
    color: var(--ink2);
    font: 600 13px/1.35 var(--f);
  }
  .ev {
    display: grid;
    gap: 8px;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .ev li {
    display: grid;
    grid-template-columns: 18px minmax(0, 1fr);
    gap: 8px;
    line-height: 1.45;
  }
  .bullet {
    width: 6px;
    height: 6px;
    margin: 0.5em 0 0 6px;
    border-radius: 50%;
    background: var(--ink3);
  }
  .bullet.first {
    background: var(--accent);
  }
  .facts {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
    gap: 6px 14px;
    margin: 0;
    padding: 10px 12px;
    border-radius: 10px;
    background: var(--card2);
    font-size: 14px;
    overflow-wrap: anywhere;
  }
  .facts dt {
    color: var(--ink2);
  }
  .facts dd {
    margin: 0;
  }
  .pill2,
  .pills .pill2 {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 1px 8px;
    border-radius: 999px;
    background: var(--card);
    color: var(--ink2);
    font: 600 12px/1.6 var(--f);
  }
  .acts {
    display: flex;
    flex-wrap: wrap;
    gap: 8px 12px;
    align-items: center;
  }
  .hint {
    color: var(--ink2);
    font-size: 13px;
  }
  .hist {
    display: grid;
    gap: 4px;
    padding-top: 6px;
    border-top: 1px solid var(--stroke2);
  }
  .hrow {
    display: grid;
    gap: 8px;
    padding: 8px 0;
  }
  .hrow + .hrow {
    border-top: 1px solid var(--stroke2);
  }
  .hline {
    display: flex;
    flex-wrap: wrap;
    gap: 4px 12px;
    margin: 0;
    font-size: 14px;
  }
  .chg {
    color: var(--ink2);
  }
  .hstate {
    display: flex;
    flex-wrap: wrap;
    gap: 8px 12px;
    align-items: center;
    justify-content: space-between;
  }
  .helped {
    display: grid;
    gap: 4px;
    padding: 10px 12px;
    border-radius: 10px;
    background: var(--card2);
    font-size: 13px;
  }
  .helped div {
    display: grid;
    grid-template-columns: 56px minmax(0, 1fr) 28px;
    gap: 8px;
    align-items: center;
  }
  .helped span,
  .helped small {
    color: var(--ink2);
  }
  .helped i {
    display: block;
    height: 10px;
    border-radius: 999px;
    background: var(--ink3);
  }
  .helped i.after {
    background: var(--accent);
  }
  .helped b {
    text-align: right;
  }
  .mono {
    color: var(--ink2);
    font-family: ui-monospace, "Cascadia Mono", Consolas, monospace;
    font-size: 12.5px;
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
  .confirm {
    max-width: min(560px, calc(100vw - 32px));
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
    margin: 0 0 10px;
    font: 600 18px/1.3 var(--fd);
  }
  .confirm p {
    margin: 0 0 12px;
    color: var(--ink2);
    font-size: 14px;
    line-height: 1.5;
  }
  .confirm ul {
    display: grid;
    gap: 10px;
    max-height: 260px;
    margin: 0 0 12px;
    padding: 0;
    overflow: auto;
    list-style: none;
  }
  .confirm li {
    display: grid;
    gap: 4px;
    padding-bottom: 10px;
    border-bottom: 1px solid var(--stroke2);
    font-size: 14px;
  }
  .confirm .pills {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .confirm .pill2 {
    background: var(--card2);
  }
  .confirm .acts {
    justify-content: flex-end;
  }
  .pill2 :global(svg) {
    width: 13px;
    height: 13px;
  }
  @container content (max-width: 760px) {
    .pane2 {
      grid-template-columns: minmax(0, 1fr);
    }
  }
  @media (forced-colors: active) {
    .bullet,
    .helped i {
      forced-color-adjust: none;
      background: CanvasText;
    }
    .pick:checked {
      forced-color-adjust: none;
      background: Highlight;
      border-color: Highlight;
    }
  }
</style>
