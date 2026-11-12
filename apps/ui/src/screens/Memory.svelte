<script lang="ts">
  import { onMount, untrack } from "svelte";

  import Block from "../components/Block.svelte";
  import HistList from "../components/HistList.svelte";
  import PageHead from "../components/PageHead.svelte";
  import RowCard from "../components/RowCard.svelte";
  import RowList from "../components/RowList.svelte";
  import StatusMark from "../components/StatusMark.svelte";
  import { marks } from "../lib/api";
  import { app } from "../lib/app.svelte";
  import { health } from "../lib/health.svelte";
  import { formatTime } from "../lib/format";
  import { lang, t, tn } from "../lib/i18n.svelte";
  import { relatedLinks } from "../lib/related.svelte";
  import { here, href } from "../lib/router.svelte";
  import { hwtests } from "../lib/tests.svelte";
  import type { Episode } from "../lib/types";
  import { pollWhileShown } from "../lib/visibility";
  import { AREAS, areaState, statusKindOf } from "./health/areas";
  import { checkText, findingTitle, markWord } from "./health/words";
  import { passState } from "../lib/monitor";
  import { formatBytes, formatDayShort } from "./occurrence/format";
  import { statusOf } from "./tests/status";
  import { diagnosticWord, kindOf, memoryState, outcomeMark, quickOf } from "./tests/words";
  import { dayStart, isPressure } from "./today/day";
  import { episodeMeta, episodeTitle } from "./today/text";

  const level = $derived(app.settings?.settings.detail_level ?? "Simple");
  const dto = $derived(hwtests.dto);
  const pass = $derived(passState(app.link, Date.now()));
  const area = $derived(health.dto ? areaState(AREAS.find((a) => a.id === "memory")!, health.dto, pass) : null);
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
    return pollWhileShown(read, 60_000);
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

  /** The three memory tests, each with how it last ended. */
  const tests = $derived.by(() => {
    const diag = dto?.diagnostics[0] ?? null;
    const quick = dto?.runs.filter((r) => kindOf(r) === "MemoryQuick").sort((a, b) => b.ended_ms - a.ended_ms)[0] ?? null;
    const diagWord = diag ? diagnosticWord(diag) : null;
    const quickMark = quick ? statusOf(outcomeMark(quick)) : null;
    return [
      {
        key: "diag",
        icon: "windows" as const,
        title: t("ui.tests.diag.title"),
        sub: diag ? stamp.format(diag.ts_ms) : t("ui.tests.diag.when"),
        mark: diagWord ? { ...statusOf(diagWord.mark), word: diagWord.text } : { kind: "info" as const, word: t("ui.memory.on_demand") },
      },
      {
        key: "quick",
        icon: "memory" as const,
        title: t("ui.tests.quick.title"),
        sub: quick ? stamp.format(quick.ended_ms) : t("ui.tests.quick.when"),
        mark: quickMark ?? { kind: "info" as const, word: t("ui.memory.on_demand") },
      },
      {
        key: "memtest",
        icon: "outside" as const,
        title: t("ui.tests.memtest.title"),
        sub: t("ui.memory.memtest.when"),
        mark: { kind: "info" as const, word: t("ui.memory.outside") },
      },
    ];
  });

  relatedLinks(() => [
    { label: t("ui.related.live_now"), href: href.activity("now") },
    { label: t("ui.item.memory_tests"), href: href.tests("memory") },
  ]);
</script>

<PageHead
  title={t("ui.group.memory")}
  sub={dto
    ? t("ui.memory.sub", { total: formatBytes(dto.memory.total_bytes), free: formatBytes(dto.memory.available_bytes) })
    : t("ui.group.memory.sub")}
/>

<div class="cols2">
  <Block title={t("ui.item.memory_health")}>
    {#if area}
      {@const kind = statusKindOf(area.mark, area.why)}
      <p class="state">
        <StatusMark {kind} word={markWord(area.mark, area.why)} />
        <span>{area.open.length ? tn("ui.health.area.points", area.open.length) : tn("ui.health.area.fine", area.checks.length)}</span>
      </p>
      {#if area.open.length}
        <RowList>
          {#each area.open as f (f.id)}
            <RowCard href={to(href.health("memory"))} title={findingTitle(f, level)}>
              {#snippet right()}<StatusMark kind={f.concern === "Critical" ? "critical" : "attention"} reveal />{/snippet}
            </RowCard>
          {/each}
        </RowList>
      {:else}
        <ul class="okl">
          {#each area.checks as check (check.kind)}
            <li><StatusMark kind={statusKindOf(check.mark, check.why)} size={20} reveal /><span>{checkText(check, pass)}</span></li>
          {/each}
        </ul>
      {/if}
    {/if}
  </Block>

  <Block title={t("ui.memory.tests")}>
    <RowList>
      {#each tests as test (test.key)}
        <RowCard href={to(href.tests("memory"))} icon={test.icon} title={test.title} sub={test.sub}>
          {#snippet right()}<StatusMark kind={test.mark.kind} word={test.mark.word} reveal />{/snippet}
        </RowCard>
      {/each}
    </RowList>
  </Block>
</div>

<div class="cols2">
  <Block title={t("ui.memory.today")}>
    {#if today.length}
      <RowList>
        {#each today as e (e.start_ms)}
          <RowCard
            href={href.period(e.start_ms)}
            icon="memory"
            title={episodeTitle(e, level)}
            sub="{episodeMeta(e)} · {formatTime(e.start_ms)}"
          >
            {#snippet right()}<StatusMark kind="overload" reveal />{/snippet}
          </RowCard>
        {/each}
      </RowList>
    {:else}
      <p class="empty">{t("ui.memory.today.none")}</p>
    {/if}
  </Block>

  <Block title={t("ui.memory.history")}>
    {#if history.length}
      <HistList
        entries={history.map((h) => ({
          key: h.title + h.at,
          date: formatDayShort(h.at),
          title: h.title,
          sub: h.text,
          mark: statusOf(h.mark),
          href: h.run === null ? to(href.tests("memory")) : href.detail("test", h.run, { from: here(), label: t("ui.group.memory") }),
        }))}
      />
    {:else}
      <p class="empty">{t("ui.memory.history.none")}</p>
    {/if}
  </Block>
</div>

<style>
  .state {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px 14px;
    margin: 0 0 10px;
    color: var(--ink2);
    font-size: 13px;
  }
  .okl {
    display: grid;
    gap: 10px;
    margin: 0;
    padding: 0;
    font-size: 14px;
    list-style: none;
  }
  .okl li {
    display: flex;
    gap: 12px;
    align-items: flex-start;
    line-height: 20px;
  }
</style>
