<script lang="ts">
  import { onMount, untrack } from "svelte";

  import Block from "../components/Block.svelte";
  import RowCard from "../components/RowCard.svelte";
  import RowList from "../components/RowList.svelte";
  import StatusMark from "../components/StatusMark.svelte";
  import { marks } from "../lib/api";
  import { app } from "../lib/app.svelte";
  import { health } from "../lib/health.svelte";
  import { lang, t } from "../lib/i18n.svelte";
  import { here, href } from "../lib/router.svelte";
  import { hwtests } from "../lib/tests.svelte";
  import type { Episode } from "../lib/types";
  import { AREAS, areaState } from "./health/areas";
  import { checkText, findingTitle } from "./health/words";
  import { formatBytes } from "./occurrence/format";
  import { statusOf } from "./tests/status";
  import { diagnosticWord, kindOf, memoryState, outcomeMark, quickOf } from "./tests/words";
  import { dayStart, isPressure } from "./today/day";
  import EpisodeRow from "./today/EpisodeRow.svelte";

  const level = $derived(app.settings?.settings.detail_level ?? "Simple");
  const dto = $derived(hwtests.dto);
  const area = $derived(health.dto ? areaState(AREAS.find((a) => a.id === "memory")!, health.dto) : null);
  let today = $state.raw<Episode[]>([]);

  onMount(() => {
    if (hwtests.dto === null) void hwtests.load();
  });

  // The memory overloads of the day, read once and again every minute.
  $effect(() => {
    if (app.history !== "ready") return;
    const read = async () => {
      const now = Date.now();
      try {
        const found = await marks(dayStart(now), now);
        today = found.episodes.filter((e) => isPressure(e) && e.kind === "MemoryHigh");
      } catch {
        // A failed read keeps what is on screen.
      }
    };
    untrack(() => void read());
    const timer = setInterval(() => {
      if (!document.hidden) void read();
    }, 60_000);
    return () => clearInterval(timer);
  });

  const standing = $derived(dto ? memoryState(dto, hwtests.running !== null) : null);
  const st = $derived(standing ? statusOf(standing.mark) : null);

  const stamp = $derived(
    new Intl.DateTimeFormat(lang(), { day: "numeric", month: "long", hour: "numeric", minute: "2-digit" }),
  );

  /** Every memory test, the newest first: the Windows diagnostic and the quick tests. */
  const history = $derived.by(() => {
    if (!dto) return [];
    const diag = dto.diagnostics.map((d) => {
      const word = diagnosticWord(d);
      return { at: d.ts_ms, title: t("ui.memory.test.diag"), mark: word.mark, text: word.text, run: null as number | null };
    });
    const quick = dto.runs
      .filter((r) => kindOf(r) === "MemoryQuick")
      .map((r) => {
        const q = quickOf(r);
        return {
          at: r.ended_ms,
          title: t("ui.memory.test.quick"),
          mark: outcomeMark(r),
          text: q ? formatBytes(q.tested_bytes) : "",
          run: r.id as number | null,
        };
      });
    return [...diag, ...quick].sort((a, b) => b.at - a.at);
  });

  const to = (path: string) => `${path}${path.includes("?") ? "&" : "?"}g=memory`;
</script>

<header class="head">
  <h1>{t("ui.group.memory")}</h1>
  <p class="sub">
    {dto
      ? t("ui.memory.sub", { total: formatBytes(dto.memory.total_bytes), free: formatBytes(dto.memory.available_bytes) })
      : t("ui.group.memory.sub")}
  </p>
</header>

<div class="cols">
  <Block title={t("ui.item.memory_health")} first>
    {#if area}
      {#if area.open.length}
        <RowList>
          {#each area.open as f (f.id)}
            <RowCard href={to(href.health("memory"))} icon="memory" title={findingTitle(f, level)}>
              {#snippet right()}<StatusMark kind={f.concern === "Critical" ? "critical" : "attention"} reveal />{/snippet}
            </RowCard>
          {/each}
        </RowList>
      {:else}
        <ul class="oks">
          {#each area.checks as check (check.kind)}
            <li>
              <StatusMark kind={check.mark === "Ok" ? "ok" : "info"} size={20} reveal />
              <span>{checkText(check)}</span>
            </li>
          {/each}
        </ul>
      {/if}
    {/if}
    <p class="more"><a href={to(href.health("memory"))}>{t("ui.memory.open_health")}</a></p>
  </Block>

  <Block title={t("ui.memory.tests")} first>
    {#if standing && st}
      <p class="state"><StatusMark kind={st.kind} word={st.word} /><span>{standing.line}</span></p>
    {/if}
    <p class="more"><a href={to(href.tests("memory"))}>{t("ui.memory.open_tests")}</a></p>
  </Block>
</div>

<Block title={t("ui.memory.today")}>
  {#if today.length}
    <ul class="list">
      {#each today as e (e.start_ms)}
        <li><EpisodeRow {e} {level} /></li>
      {/each}
    </ul>
  {:else}
    <p class="muted">{t("ui.memory.today.none")}</p>
  {/if}
</Block>

<Block title={t("ui.memory.history")}>
  {#if history.length}
    <RowList>
      {#each history as h (h.title + h.at)}
        {@const s = statusOf(h.mark)}
        <RowCard
          href={h.run === null ? to(href.tests("memory")) : href.detail("test", h.run, { from: here(), label: t("ui.group.memory") })}
          icon="tests"
          title={h.title}
          sub={[stamp.format(h.at), h.text].filter(Boolean).join(" · ")}
        >
          {#snippet right()}<StatusMark kind={s.kind} word={s.word} reveal />{/snippet}
        </RowCard>
      {/each}
    </RowList>
  {:else}
    <p class="muted">{t("ui.memory.history.none")}</p>
  {/if}
</Block>

<style>
  .head {
    margin-bottom: 4px;
  }
  .sub {
    margin: 4px 0 0;
    color: var(--ink2);
    font-size: 14px;
  }
  .cols {
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    gap: 0 56px;
  }
  @container content (min-width: 1000px) {
    .cols {
      grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
      align-items: start;
    }
  }
  .state {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px 14px;
    margin: 0 0 14px;
    font-size: 14px;
  }
  .oks {
    display: grid;
    gap: 4px;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .oks li {
    display: grid;
    grid-template-columns: 20px minmax(0, 1fr);
    gap: 12px;
    align-items: start;
    padding: 6px 0;
    font-size: 14px;
    line-height: 1.45;
  }
  .list {
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
  .more {
    margin: 14px 0 0;
    font-size: 14px;
  }
  .more a {
    font-weight: 600;
  }
</style>
