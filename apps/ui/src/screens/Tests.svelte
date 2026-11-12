<script lang="ts">
  import { onMount } from "svelte";

  import AdminPill from "../components/AdminPill.svelte";
  import Block from "../components/Block.svelte";
  import type { MarkKind } from "../components/ConcernMark.svelte";
  import ErrorView from "../components/ErrorView.svelte";
  import HelperDialogs from "../components/HelperDialogs.svelte";
  import HelperLine from "../components/HelperLine.svelte";
  import HistList from "../components/HistList.svelte";
  import Icon, { type IconName } from "../components/Icon.svelte";
  import Loading from "../components/Loading.svelte";
  import PageHead from "../components/PageHead.svelte";
  import RowCard from "../components/RowCard.svelte";
  import RowList from "../components/RowList.svelte";
  import StatusMark from "../components/StatusMark.svelte";
  import { app } from "../lib/app.svelte";
  import { health } from "../lib/health.svelte";
  import { errorText } from "../lib/errors";
  import { helperJobs } from "../lib/helper.svelte";
  import { formatNumber, formatPercent } from "../lib/format";
  import { t, tn } from "../lib/i18n.svelte";
  import { relatedLinks } from "../lib/related.svelte";
  import { go, here, href } from "../lib/router.svelte";
  import { hwtests, type Started } from "../lib/tests.svelte";
  import { toast } from "../lib/toast.svelte";
  import type { BenchMode, DiskDto, FindingDto, MemoryDiagnostic, TestRunDto, VolumeDto } from "../lib/types";
  import { at } from "./dumps/words";
  import { formatBytes, formatDayShort } from "./occurrence/format";
  import { findingTitle } from "./health/words";
  import DiskMap from "./tests/DiskMap.svelte";
  import { statusOf } from "./tests/status";
  import { mapView } from "./tests/mapView.svelte";
  import ReadHistory from "./tests/ReadHistory.svelte";
  import {
    attrName,
    benchModeName,
    benchOf,
    checkOf,
    type Device,
    deviceKey,
    diagnosticCovered,
    diagnosticWord,
    diskState,
    kindOf,
    lastBench,
    lastCheck,
    lastQuick,
    diskSurface,
    lastSurface,
    surfaceRuns,
    unreadableText,
    memoryState,
    outcomeMark,
    quickCovered,
    quickOf,
    repairMark,
    runningDevice,
    shareOf,
    span,
    surfaceEstimate,
    surfaceOf,
    smartWarns,
    surfaceWord,
    WATCHED,
  } from "./tests/words";

  let { device }: { device: string | null } = $props();

  const DIAG_ASKED = "vestigium.tests.diagnostic_asked";

  /** When the Windows diagnostic was last asked for; kept in this window's storage only. */
  function recallAsked(): number | null {
    try {
      const raw = localStorage.getItem(DIAG_ASKED);
      return raw ? Number(raw) : null;
    } catch {
      return null;
    }
  }

  let minutes = $state(5);
  let now = $state(Date.now());
  let dialog = $state<HTMLDialogElement>();
  let asking = $state<{ kind: "bench"; volume: VolumeDto } | { kind: "smart"; disk: DiskDto } | null>(null);
  /** The disk whose earlier reads are open in a dialog. */
  let reading = $state<number | null>(null);
  let asked = $state<number | null>(recallAsked());

  const level = $derived(app.settings?.settings.detail_level ?? "Simple");
  const dto = $derived(hwtests.dto);
  const overview = $derived(device === null);
  const chosen = $derived<Device>(device === null || device === "memory" ? "memory" : Number(device));
  const disk = $derived(chosen === "memory" ? null : (dto?.disks.find((d) => d.number === chosen) ?? null));
  const running = $derived(hwtests.running);
  const runDev = $derived(dto ? runningDevice(dto, running) : null);
  const busy = $derived(hwtests.busy || running !== null || dto?.scan != null);
  const diag = $derived(dto?.diagnostics[0] ?? null);
  // A request counts until a run of the Windows diagnostic newer than it shows up.
  const waiting = $derived(asked !== null && !(diag && diag.ts_ms > asked));

  onMount(() => {
    if (health.dto === null) void health.load();
    void hwtests.load().then(readQuickDetails);
    const timer = setInterval(() => (now = Date.now()), 1000);
    return () => clearInterval(timer);
  });

  /** SSD and NVMe health reads at once; an HDD only when asked, since reading wakes it. */
  function readQuickDetails(): void {
    for (const d of hwtests.dto?.disks ?? []) {
      if (d.media !== "Hdd" && !hwtests.details[d.number]) void hwtests.readDetail(d);
    }
  }


  function retry(): void {
    hwtests.error = null;
    void hwtests.load();
  }

  async function stop(): Promise<void> {
    try {
      await hwtests.stop();
    } catch (error) {
      toast.show(errorText(error));
    }
  }

  function started(outcome: Started): void {
    if (outcome.kind === "declined") toast.show(t("ui.tests.toast.declined"));
    else if (outcome.kind === "failed") toast.show(outcome.error ? errorText(outcome.error) : t("ui.tests.toast.busy"));
  }

  async function diagnostic(): Promise<void> {
    const outcome = await hwtests.diagnostic();
    if (outcome.kind === "started") {
      asked = Date.now();
      try {
        localStorage.setItem(DIAG_ASKED, String(asked));
      } catch {
        // Without storage the request is forgotten when the window closes.
      }
      toast.show(t("ui.tests.toast.diag_opened"));
    } else started(outcome);
  }

  function ask(next: NonNullable<typeof asking>): void {
    asking = next;
    if (!dialog?.open) dialog?.showModal();
  }

  function close(): void {
    dialog?.close();
    asking = null;
  }

  async function confirmed(): Promise<void> {
    const now = asking;
    close();
    if (now?.kind === "bench") started(await hwtests.benchmark(now.volume.root));
    if (now?.kind === "smart") await hwtests.readDetail(now.disk);
  }

  function bench(v: VolumeDto): void {
    if (dto && v.free_bytes < dto.bench_needed_bytes) {
      toast.show(t("ui.tests.toast.no_room", { volume: vol(v.root), needed: formatBytes(dto.bench_needed_bytes) }));
      return;
    }
    const d = dto?.disks.find((x) => x.number === v.disk);
    if (d?.media === "Ssd") ask({ kind: "bench", volume: v });
    else void hwtests.benchmark(v.root).then(started);
  }

  function smart(d: DiskDto): void {
    if (d.media === "Hdd") ask({ kind: "smart", disk: d });
    else void hwtests.readDetail(d);
  }

  const diskName = (d: DiskDto) => t(`ui.tests.disk.name.${d.media}`, { size: d.bytes ? formatBytes(d.bytes) : "" }).trim();
  const diskIcon = (d: DiskDto): IconName => (d.media === "Hdd" ? "hdd" : "ssd");
  /** "C:" for the root "C:\", as people name a volume. */
  const vol = (root: string) => root.replace(/\\$/, "");
  const volumesOf = (d: DiskDto) => (dto?.volumes ?? []).filter((v) => v.disk === d.number);

  /** Everything the page says about the test running now, or a scan the helper still runs. */
  const live = $derived.by(() => {
    if (!dto) return null;
    const p = hwtests.progress;
    if (running?.kind === "MemoryQuick") {
      const total = (running.ends_ms ?? now) - running.started_ms;
      const frac = Math.min(1, (now - running.started_ms) / Math.max(1, total));
      const facts = p?.kind === "memory"
        ? [t(`ui.tests.pattern.${p.pattern}`), t("ui.tests.live.round", { n: p.rounds + 1 }), (p.error_count ? tn("ui.tests.live.errors", p.error_count) : t("ui.tests.live.errors.zero"))]
        : [];
      return { title: t("ui.tests.live.memory"), note: t("ui.tests.live.memory.note"), frac, facts: [...facts, t("ui.tests.live.left", { time: span(Math.max(0, (running.ends_ms ?? now) - now)) })], bad: p?.kind === "memory" && p.error_count > 0, stoppable: true };
    }
    if (running?.kind === "DiskBenchmark") {
      const done = p?.kind === "benchmark" ? MODES.indexOf(p.mode) : -1;
      const root = typeof running.subject === "object" && "Volume" in running.subject ? running.subject.Volume : "";
      return { title: t("ui.tests.live.bench", { volume: vol(root) }), note: t("ui.tests.live.bench.note"), frac: (done + 1) / (MODES.length + 1), facts: [done < 0 ? t("ui.tests.live.bench.prepare") : t("ui.tests.live.bench.mode", { mode: benchModeName(MODES[done], level) })], bad: false, stoppable: true };
    }
    if (running?.kind === "VolumeCheck") {
      const root = typeof running.subject === "object" && "Volume" in running.subject ? running.subject.Volume : "";
      return { title: t("ui.tests.live.check", { volume: vol(root) }), note: t("ui.tests.live.check.note"), frac: null, facts: [], bad: false, stoppable: true, confirm: true };
    }
    const s = p?.kind === "surface" ? p : dto.scan?.progress;
    if (running?.kind === "DiskSurface" || dto.scan) {
      const target = dto.disks.find((d) => d.number === (s?.disk ?? runDev));
      const read = s ? s.at_bytes - s.from_bytes : 0;
      const rate = s && running ? read / Math.max(1, now - running.started_ms) : 0;
      const facts = s
        ? [t("ui.tests.live.read", { read: formatBytes(s.at_bytes), total: formatBytes(s.disk_bytes) }), (s.unreadable_blocks ? tn("ui.tests.live.unreadable", s.unreadable_blocks) : t("ui.tests.live.unreadable.zero"))]
        : [];
      if (s && rate > 0) facts.push(t("ui.tests.live.left", { time: span((s.disk_bytes - s.at_bytes) / rate) }));
      return { title: t("ui.tests.live.surface", { name: target ? diskName(target) : "" }), note: t("ui.tests.live.surface.note"), frac: s ? s.at_bytes / Math.max(1, s.disk_bytes) : 0, facts, bad: (s?.unreadable_blocks ?? 0) > 0, stoppable: true, read: s ? s.at_bytes / Math.max(1, s.disk_bytes) : 0, bytes: s?.disk_bytes ?? target?.bytes ?? 0 };
    }
    return null;
  });

  const MODES: BenchMode[] = ["SeqRead", "SeqWrite", "RandRead32", "RandWrite32", "RandRead1", "RandWrite1"];

  const verdict = $derived.by(() => {
    if (!dto) return null;
    if (live) return { strong: t("ui.tests.lede.running", { what: live.title }), rest: t("ui.tests.lede.running.tail") };
    const mem = memoryState(dto, false);
    const states = dto.disks.map((d) => ({ d, s: diskState(dto, d, hwtests.details[d.number] ?? null, false) }));
    const bad = [mem.mark === "Critical" ? t("ui.tests.lede.the_memory") : null, ...states.filter((x) => x.s.mark === "Critical").map((x) => t("ui.tests.lede.the_disk", { name: diskName(x.d) }))].filter((x): x is string => x !== null);
    const att = states.filter((x) => x.s.mark === "Attention").map((x) => t("ui.tests.lede.the_disk", { name: diskName(x.d) }));
    const list = (xs: string[]) => {
      const text = new Intl.ListFormat(document.documentElement.lang || undefined, { type: "conjunction" }).format(xs);
      return text.charAt(0).toLocaleUpperCase() + text.slice(1);
    };
    if (bad.length) return { strong: tn("ui.tests.lede.bad", bad.length, { what: list(bad) }), rest: t("ui.tests.lede.bad.tail") };
    if (att.length) return { strong: tn("ui.tests.lede.attention", att.length, { what: list(att) }), rest: t("ui.tests.lede.attention.tail") };
    if (!dto.runs.length && !dto.diagnostics.length) return { strong: t("ui.tests.lede.never"), rest: t("ui.tests.lede.never.tail") };
    return { strong: t("ui.tests.lede.clean"), rest: t("ui.tests.lede.clean.tail") };
  });

  const RANK: Record<string, number> = { Attention: 2, Critical: 3 };
  /** The open Health point about a device, the worst first; the tests never contradict it. */
  function healthPoint(id: Device): FindingDto | null {
    const mine = (health.dto?.open ?? []).filter((f) =>
      id === "memory"
        ? f.kind === "HardwareErrors" || f.kind === "MemoryTestErrors"
        : typeof f.subject === "object" && "Disk" in f.subject && Number(f.subject.Disk.split(" ")[0]) === id,
    );
    return mine.sort((a, b) => (RANK[b.concern] ?? 0) - (RANK[a.concern] ?? 0))[0] ?? null;
  }
  function withHealth(id: Device, s: { mark: MarkKind; line: string }): { mark: MarkKind; line: string } {
    const point = healthPoint(id);
    if (!point || s.mark === "Running" || (RANK[point.concern] ?? 0) <= (RANK[s.mark] ?? 0)) return s;
    return { mark: point.concern as MarkKind, line: t("ui.tests.health_says", { what: findingTitle(point, level) }) };
  }

  relatedLinks(() =>
    chosen === "memory" && !overview
      ? [
          { label: t("ui.item.memory_health"), href: href.health("memory") },
          { label: t("ui.group.tests"), href: href.tests() },
        ]
      : disk && !overview
        ? [
            { label: t("ui.related.disk_health"), href: href.health("disks") },
            { label: t("ui.group.disks"), href: href.disks() },
          ]
        : [
            { label: t("ui.related.disk_health"), href: href.health("disks") },
            { label: t("ui.item.memory_health"), href: href.health("memory") },
          ],
  );

  const historyOf = (d: Device) =>
    [
      ...(dto?.runs ?? [])
        .filter((r) =>
          d === "memory"
            ? kindOf(r) === "MemoryQuick"
            : typeof r.subject === "object" &&
              (("Disk" in r.subject && Number(r.subject.Disk.split(" ")[0]) === d) ||
                ("Volume" in r.subject && dto?.volumes.find((v) => v.root === (r.subject as { Volume: string }).Volume)?.disk === d)),
        )
        .map((r) => ({ ts: r.ended_ms, run: r as TestRunDto | null, diag: null as MemoryDiagnostic | null })),
      ...(d === "memory" ? (dto?.diagnostics ?? []).map((x) => ({ ts: x.ts_ms, run: null, diag: x })) : []),
    ].sort((a, b) => b.ts - a.ts);

  function runLine(r: TestRunDto): { name: string; what: string } {
    const q = quickOf(r), s = surfaceOf(r), c = checkOf(r), b = benchOf(r);
    if (q) return { name: t("ui.tests.hist.quick"), what: `${formatBytes(q.tested_bytes)} / ${formatBytes(q.total_bytes)}${q.error_count ? ` · ${tn("ui.tests.quick.errors", q.error_count)}` : ""}` };
    if (s) return { name: t("ui.tests.hist.surface"), what: s.unreadable_blocks ? unreadableText(s) : t("ui.tests.hist.read", { share: shareOf(s.to_bytes - s.from_bytes, s.disk_bytes) }) };
    if (c) return { name: t("ui.tests.hist.check", { volume: vol(c.volume) }), what: t(`ui.tests.check.${c.result}`) };
    const seq = b?.results.find((x) => x.mode === "SeqRead");
    return { name: t("ui.tests.hist.bench", { volume: vol(b?.volume ?? "") }), what: seq ? t("ui.tests.bench.seq", { value: formatNumber(seq.mb_per_s) }) : "" };
  }
</script>

{#snippet devRow(id: Device, icon: IconName, name: string, sub: string)}
  {@const s = withHealth(id, id === "memory" ? memoryState(dto!, runDev === "memory") : diskState(dto!, dto!.disks.find((d) => d.number === id)!, hwtests.details[id as number] ?? null, runDev === id))}
  {@const st = statusOf(s.mark)}
  <RowCard href={href.tests(deviceKey(id))} {icon} title={name} sub="{sub} · {s.line}">
    {#snippet right()}<StatusMark kind={st.kind} word={st.word} reveal />{/snippet}
  </RowCard>
{/snippet}

{#snippet liveBlock()}
  {#if live}
    <div class="running" aria-live="polite">
      <p class="state"><StatusMark kind="info" word={t("ui.health.mark.Running")} /><span>{live.title}</span></p>
      <div class="run">
        <span class="run-l"><span>{live.note}</span>{#if live.frac !== null}<b>{formatPercent(live.frac * 100)}</b>{/if}</span>
        <span class="pbar" class:indeterminate={live.frac === null} role="progressbar" aria-valuemin="0" aria-valuemax="100" aria-valuenow={live.frac === null ? undefined : Math.round(live.frac * 100)}>
          <i style:width={live.frac === null ? undefined : `${live.frac * 100}%`}></i>
        </span>
        {#if live.stoppable && "confirm" in live}
          {#if hwtests.stopping}
            <button type="button" class="btn" disabled aria-disabled="true">{t("ui.helper.cancelling")}</button>
          {:else}
            <button type="button" class="btn" onclick={() => helperJobs.ask("check")}>{t("ui.helper.cancel.check")}</button>
          {/if}
        {:else if live.stoppable}
          <button type="button" class="btn" onclick={() => void stop()}>{t("ui.tests.cancel")}</button>
        {/if}
      </div>
      {#if "confirm" in live && hwtests.stopping}
        <p class="note" role="status">{t("ui.helper.cancelling.note")}</p>
      {/if}
      {#if live.facts.length}
        <p class="note facts-line" class:bad={live.bad}>{#each live.facts as f, i (i)}<span>{f}</span>{/each}</p>
      {/if}
      {#if "read" in live && live.bytes}
        <DiskMap read={live.read} diskBytes={live.bytes} name={live.title} {level} />
      {/if}
    </div>
  {/if}
{/snippet}

{#snippet history(d: Device)}
  {@const rows = historyOf(d)}
  <Block title={t("ui.tests.hist.title")}>
    {#if rows.length}
      <HistList
        entries={rows.map((row, i) => {
          if (row.run) {
            const line = runLine(row.run);
            return {
              key: `r${row.run.id}`,
              date: formatDayShort(row.ts),
              title: line.name,
              sub: line.what,
              mark: statusOf(outcomeMark(row.run)),
              href: href.detail("test", row.run.id, { from: here(), label: d === "memory" ? t("ui.tests.memory.title") : disk ? diskName(disk) : t("ui.tests.title") }),
            };
          }
          const w = diagnosticWord(row.diag!);
          return {
            key: `d${row.diag!.record_id}`,
            date: formatDayShort(row.ts),
            title: t("ui.tests.hist.diag"),
            sub: w.text,
            mark: { ...statusOf(w.mark), word: w.text },
            href: href.detail("diagnostic", row.diag!.record_id, { from: here(), label: t("ui.tests.memory.title") }),
          };
        })}
      />
    {:else}
      <p class="empty">{t("ui.tests.hist.none")}</p>
    {/if}
  </Block>
{/snippet}

{#snippet memoryReport()}
  {@const quick = lastQuick(dto!)}
  {@const q = quick ? quickOf(quick) : null}
  <PageHead title={t("ui.tests.memory.title")} sub={t("ui.tests.memory.sub", { total: formatBytes(dto!.memory.total_bytes), free: formatBytes(dto!.memory.available_bytes) })} />
  {#if runDev === "memory"}{@render liveBlock()}{/if}

  <div class="cols2">
    <Block title={t("ui.tests.diag.title")} icon="windows">
      {#if diag}
        {@const w = diagnosticWord(diag)}
        {@const st = statusOf(w.mark)}
        <p class="state"><StatusMark kind={st.kind} word={w.text} /><span>{t("ui.tests.diag.when")}</span></p>
        <dl class="kv"><div><dt>{t("ui.tests.last")}</dt><dd>{at(diag.ts_ms)}{diag.detail ? ` · ${span(diag.detail.duration_s * 1000)}` : ""}</dd></div></dl>
        <p class="p">{diagnosticCovered(diag, level)}</p>
      {:else}
        <p class="state"><StatusMark kind="info" word={t("ui.memory.on_demand")} /><span>{t("ui.tests.diag.when")}</span></p>
        <p class="p">{t("ui.tests.diag.never")}</p>
      {/if}
      {#if waiting && asked !== null}
        <p class="note">{t("ui.tests.diag.waiting", { when: at(asked) })}</p>
      {/if}
      <div class="acts">
        <button type="button" class="btn" class:accent={!diag && !waiting && quick !== null} disabled={busy} onclick={() => void diagnostic()}>
          <Icon name="restart" />{t(diag || waiting ? "ui.tests.diag.again" : "ui.tests.diag.start")}
        </button>
      </div>
      <p class="tags"><AdminPill /><span class="tag">{t("ui.tests.pill.restart")}</span></p>
    </Block>

    <Block title={t("ui.tests.quick.title")} icon="memory">
      {#if quick && q}
        {@const st = statusOf(outcomeMark(quick))}
        <p class="state">
          <StatusMark kind={st.kind} word={q.error_count ? tn("ui.tests.quick.errors", q.error_count) : t(quick.outcome === "Interrupted" ? "ui.tests.result.stopped" : quick.outcome === "Failed" ? "ui.tests.result.failed" : "ui.tests.result.clean")} />
          <span>{t("ui.tests.quick.when")}</span>
        </p>
        <dl class="kv"><div><dt>{t("ui.tests.last")}</dt><dd>{at(quick.ended_ms)} · {span(quick.ended_ms - quick.started_ms)} · {tn("ui.tests.quick.rounds", q.rounds)}</dd></div></dl>
        {#if q.failure}
          <p class="p">{t("ui.tests.quick.failure")}{level === "Technical" ? ` (${q.failure})` : ""}</p>
        {:else}
          <p class="p">{quickCovered(q, level)}</p>
        {/if}
        {#if q.errors.length && level !== "Simple"}
          <table class="tbl">
            <thead><tr><th>{t("ui.tests.err.where")}</th><th>{t("ui.tests.err.pattern")}</th><th>{t("ui.tests.err.expected")}</th><th>{t("ui.tests.err.read")}</th></tr></thead>
            <tbody>
              {#each q.errors as e, i (i)}
                <tr><td>{t("ui.tests.err.offset", { at: formatBytes(e.offset) })}</td><td>{t(`ui.tests.pattern.${e.pattern}`)}</td><td class="mono">0x{e.expected.toString(16).padStart(16, "0")}</td><td class="mono">0x{e.actual.toString(16).padStart(16, "0")}</td></tr>
              {/each}
            </tbody>
          </table>
        {/if}
      {:else}
        <p class="state"><StatusMark kind="info" word={t("ui.memory.on_demand")} /><span>{t("ui.tests.quick.when")}</span></p>
        <p class="p">{t("ui.tests.quick.never")}</p>
      {/if}
      <p class="caveat"><Icon name="info" /><span>{t(q?.error_count ? "ui.tests.quick.honest.errors" : "ui.tests.quick.honest")}</span></p>
      {#if dto!.memory.quick_bytes === null}
        <p class="note">{t("ui.tests.quick.low", { free: formatBytes(dto!.memory.available_bytes) })}</p>
      {:else}
        <div class="fld">
          <p class="fld-l">{t("ui.tests.quick.length")}</p>
          <div class="rds">
            {#each dto!.memory.minutes as m (m)}
              <label class="rd">
                <input type="radio" name="quick-minutes" value={m} checked={minutes === m} onchange={() => (minutes = m)} />
                <span class="rd-c" aria-hidden="true"></span>
                <span>{tn("ui.tests.span.min", m)}</span>
              </label>
            {/each}
          </div>
          <p class="fld-n">{t("ui.tests.quick.will", { tested: formatBytes(dto!.memory.quick_bytes), total: formatBytes(dto!.memory.total_bytes) })}</p>
        </div>
        <div class="acts">
          <button type="button" class="btn" class:accent={!quick && !diag} disabled={busy} onclick={() => void hwtests.memory(minutes).then(started)}>
            <Icon name="start" />{t("ui.tests.quick.start")}
          </button>
        </div>
      {/if}
    </Block>
  </div>

  <div class="cols2">
    <Block title={t("ui.tests.memtest.title")} icon="outside">
      <p class="state"><StatusMark kind="info" word={t("ui.memory.outside")} /><span>{t("ui.memory.memtest.when")}</span></p>
      <p class="p">{t("ui.tests.memtest.body")}</p>
    </Block>
    {@render history("memory")}
  </div>
{/snippet}

{#snippet diskReport(d: DiskDto)}
  {@const s = withHealth(d.number, diskState(dto!, d, hwtests.details[d.number] ?? null, runDev === d.number))}
  {@const st = statusOf(s.mark)}
  {@const detail = hwtests.details[d.number] ?? null}
  {@const scanRun = lastSurface(dto!, d)}
  {@const reads = surfaceRuns(dto!, d)}
  {@const scan = diskSurface(dto!, d)}
  {@const resumable = scanRun?.outcome === "Interrupted" && scan !== null && scan.to_bytes < scan.disk_bytes}
  {#snippet diskMeta()}
    <StatusMark kind={st.kind} word={st.word} />
    <span>{t("ui.tests.disk.sub", { n: d.number, bus: t(`ui.tests.bus.${d.bus}`), volumes: d.volumes.map(vol).join(", ") || "—" })}</span>
  {/snippet}
  <PageHead title={diskName(d)} meta={diskMeta} />
  <p class="lede">{s.line}</p>
  {#if runDev === d.number}{@render liveBlock()}{/if}

  <Block title={t("ui.tests.surface.title")}>
    {#if scanRun && scan}
      {@const w = surfaceWord(scanRun, scan, d)}
      {@const sw = statusOf(w.mark)}
      <p class="state">
        <StatusMark kind={sw.kind} word={sw.word} />
        <span>{at(scanRun.ended_ms)} · {span(scanRun.ended_ms - scanRun.started_ms)}{scan.from_bytes === 0 && scan.to_bytes >= scan.disk_bytes ? ` · ${t("ui.tests.surface.whole")}` : ""}</span>
      </p>
      <div class="map"><DiskMap {scan} diskBytes={scan.disk_bytes} name={diskName(d)} {level} /></div>
      <p class="p">{w.text}</p>
      {#if mapView.speed && level !== "Simple" && scan.worst.length}
        <h3 class="h3">{t("ui.tests.surface.worst")}</h3>
        <table class="tbl">
          <tbody>
            {#each scan.worst.slice(0, 5) as b, i (i)}
              <tr><td class="nm">{t("ui.tests.err.offset", { at: formatBytes(b.offset) })}</td><td class="num">{formatNumber(b.ms / 1000, 1)} s</td></tr>
            {/each}
          </tbody>
        </table>
      {/if}
      {#if level === "Technical" && scan.unreadable.length}
        <p class="note mono">{t("ui.tests.surface.sectors", { list: scan.unreadable.slice(0, 20).map((o) => formatNumber(o / scan.sector_bytes)).join(", ") })}</p>
      {/if}
      {#if mapView.speed && d.media !== "Hdd"}<p class="note">{t("ui.tests.surface.ssd")}</p>{/if}
    {:else}
      <p class="p">{t("ui.tests.surface.never")}</p>
    {/if}
    <div class="acts">
      {#if resumable && scan}
        <button type="button" class="btn accent" disabled={busy} onclick={() => void hwtests.surface(d.number, Math.floor(scan.to_bytes / 1048576)).then(started)}>
          <Icon name="scan" />{t("ui.tests.surface.resume", { share: shareOf(scan.to_bytes, scan.disk_bytes) })}
        </button>
      {/if}
      <button type="button" class="btn" class:accent={!resumable} disabled={busy} onclick={() => void hwtests.surface(d.number, 0).then(started)}>
        <Icon name="scan" />{t(resumable ? "ui.tests.surface.restart" : scanRun ? "ui.tests.surface.again" : "ui.tests.surface.start")}
      </button>
      <span class="muted">{surfaceEstimate(d)}</span>
      {#if reads.length > 1}
        <button type="button" class="btn earlier" onclick={() => (reading = d.number)}>
          <Icon name="clock" />{t("ui.tests.reads.open", { count: reads.length })}
        </button>
      {/if}
    </div>
  </Block>

  <div class="cols2">
    <Block title={t("ui.tests.smart.title")}>
      {#if !detail}
        <p class="p">{t(d.media === "Hdd" ? "ui.tests.smart.ask" : "ui.tests.smart.reading")}</p>
        {#if d.media === "Hdd"}
          <div class="acts"><button type="button" class="btn" onclick={() => smart(d)}><Icon name="smart" />{t("ui.tests.smart.read")}</button></div>
        {/if}
      {:else if detail.nvme}
        {@const n = detail.nvme}
        <RowList>
          {@render smartRow(t("ui.tests.nvme.used"), formatPercent(n.health.percentage_used), n.health.percentage_used >= 90 ? "attention" : null, null)}
          {@render smartRow(t("ui.tests.nvme.spare"), formatPercent(n.health.available_spare_pct), n.health.available_spare_pct < n.health.spare_threshold_pct ? "attention" : null, null)}
          {@render smartRow(t("ui.tests.nvme.temp"), `${n.health.temperature_c} °C`, null, null)}
          {#if level !== "Simple"}
            {@render smartRow(t("ui.tests.nvme.written"), formatBytes(n.data_written_bytes), null, null)}
            {@render smartRow(t("ui.tests.nvme.hours"), formatNumber(n.power_on_hours), null, null)}
            {@render smartRow(t("ui.tests.nvme.unsafe"), formatNumber(n.unsafe_shutdowns), null, null)}
            {@render smartRow(t("ui.tests.nvme.media"), formatNumber(n.health.media_errors), n.health.media_errors ? "critical" : null, null)}
          {/if}
        </RowList>
        {#if level === "Technical"}<p class="note mono">{t("ui.tests.nvme.tech", { warning: n.health.critical_warning.toString(16).padStart(2, "0"), errors: formatNumber(n.error_log_entries) })}</p>{/if}
      {:else if detail.health}
        {@const attrs = detail.health.attributes}
        {#if detail.health.predict_failure}
          <p class="state"><StatusMark kind="critical" word={t("ui.health.mark.Critical")} /><span>{t("ui.tests.smart.predict")}</span></p>
        {/if}
        <RowList>
          {#each attrs.filter((a) => level !== "Simple" || WATCHED.includes(a.id)) as a (a.id)}
            {@render smartRow(attrName(a.id), formatNumber(a.raw), smartWarns(a, d.media) ? "attention" : null, href.detail("smart", [d.number, a.id], { from: here(), label: diskName(d) }))}
          {/each}
        </RowList>
      {:else}
        <p class="p">{t("ui.tests.smart.none")}</p>
      {/if}
    </Block>

    <Block title={t("ui.tests.fs.title")}>
      {#each volumesOf(d) as v (v.root)}
        {@const check = lastCheck(dto!, v.root)}
        {@const c = check ? checkOf(check) : null}
        {@const mark = repairMark(dto!, v.root)}
        <div class="vol">
          {#if v.file_system !== "NTFS"}
            <p class="p">{t("ui.tests.check.not_ntfs", { fs: v.file_system ?? "?" })}</p>
          {:else}
            {#if check && c}
              {@const cs = statusOf(c.result === "ProblemsFound" ? "Attention" : c.result === "NoProblems" ? "Ok" : "Unknown")}
              <p class="state"><StatusMark kind={cs.kind} word={t(`ui.tests.check.${c.result}`)} /></p>
            {/if}
            <HelperLine job="check" volume={v.root} />
            <dl class="kv">
              <div><dt>{t("ui.tests.fs.volume")}</dt><dd>{vol(v.root)}</dd></div>
              <div><dt>{t("ui.tests.fs.last")}</dt><dd>{check ? at(check.ended_ms) : t("ui.tests.check.never")}</dd></div>
            </dl>
            {#if mark}
              <div class="panel" role="status">
                <span class="pi"><StatusMark kind="attention" word="" /></span>
                <div>
                  <h3>{t(`ui.tests.check.panel.${mark}`, { volume: vol(v.root) })}</h3>
                  <p>{t("ui.tests.check.panel.body")}</p>
                </div>
              </div>
            {/if}
            {#if level === "Technical" && c?.output}<pre class="out">{c.output}</pre>{/if}
            <div class="acts">
              <button type="button" class="btn" disabled={busy} onclick={() => void hwtests.volume(v.root).then(started)}><Icon name="fsck" />{t("ui.tests.check.start")}</button>
            </div>
          {/if}
        </div>
      {:else}
        <p class="empty">{t("ui.tests.fs.none")}</p>
      {/each}
    </Block>
  </div>

  <div class="cols2">
    <Block title={t("ui.tests.speed.title")}>
      {#each volumesOf(d) as v (v.root)}
        {@const benchRun = lastBench(dto!, v.root)}
        {@const b = benchRun ? benchOf(benchRun) : null}
        <div class="vol">
          {#if benchRun && b}
            {@const bs = statusOf(benchRun.outcome === "Clean" ? "Ok" : "Partial")}
            <p class="state"><StatusMark kind={bs.kind} word={t(benchRun.outcome === "Clean" ? "ui.tests.bench.done" : benchRun.outcome === "Failed" ? "ui.tests.result.failed" : "ui.tests.bench.stopped")} /><span>{at(benchRun.ended_ms)} · {vol(v.root)} · {t(`ui.tests.bus.${b.bus}`)}</span></p>
            <div class="speed">
              {#each b.results.filter((r) => level !== "Simple" || r.mode === "SeqRead" || r.mode === "RandRead1") as r (r.mode)}
                {@const rg = d.ranges.find((x) => x.mode === r.mode)}
                {@const max = Math.max(rg?.high ?? 0, r.mb_per_s) * 1.1 || 1}
                <div class="sp">
                  <div class="sp-head">
                    <span>{benchModeName(r.mode, level)}</span>
                    <span><b>{formatNumber(r.mb_per_s, r.mb_per_s < 10 ? 1 : 0)}</b> MB/s{#if rg}<small> · {t("ui.tests.speed.typical", { low: formatNumber(rg.low, 0), high: formatNumber(rg.high, 0) })}</small>{/if}</span>
                  </div>
                  <span class="sp-track" role="img" aria-label={rg ? t("ui.tests.bench.against", { value: formatNumber(r.mb_per_s, 1), low: formatNumber(rg.low, 1), high: formatNumber(rg.high, 1) }) : t("ui.tests.bench.value", { value: formatNumber(r.mb_per_s, 1) })}>
                    {#if rg}<i class="sp-typ" style:left="{(rg.low / max) * 100}%" style:width="{((rg.high - rg.low) / max) * 100}%"></i>{/if}
                    <i class="sp-val" class:low={rg && r.mb_per_s < rg.low} style:width="{(r.mb_per_s / max) * 100}%"></i>
                  </span>
                </div>
              {/each}
            </div>
            <p class="note">
              {d.ranges.length ? t(`ui.tests.bench.range.${d.media}`, { bus: t(`ui.tests.bus.${d.bus}`) }) : t("ui.tests.bench.no_range")}
              {t("ui.tests.bench.written", { written: formatBytes(b.written_bytes) })}
              {#if b.outside_io}<b>{t("ui.tests.bench.outside")}</b>{/if}
            </p>
          {:else}
            <p class="p">{t("ui.tests.bench.never")}</p>
          {/if}
          <div class="acts">
            <button type="button" class="btn" disabled={busy} onclick={() => bench(v)}><Icon name="gauge" />{t(benchRun ? "ui.tests.bench.again" : "ui.tests.bench.start")}</button>
          </div>
        </div>
      {:else}
        <p class="empty">{t("ui.tests.fs.none")}</p>
      {/each}
    </Block>
    {@render history(d.number)}
  </div>
{/snippet}

{#snippet smartRow(name: string, value: string, warn: "attention" | "critical" | null, link: string | null)}
  <RowCard href={link ?? undefined} title={name} chevron={link !== null} static={link === null}>
    {#snippet right()}
      <b class="smart-v">{value}</b>
      {#if warn}<StatusMark kind={warn} reveal />{/if}
    {/snippet}
  </RowCard>
{/snippet}

{#if dto === null && hwtests.error !== null}
  <PageHead title={t("ui.tests.title")} />
  <div class="gap"><ErrorView error={hwtests.error} heading="h2" onretry={retry} /></div>
{:else if dto === null}
  <PageHead title={t("ui.tests.title")} />
  <Loading />
{:else if overview}
  <PageHead title={t("ui.tests.title")} sub={t("ui.tests.coverage")} />
  {#if verdict}<p class="lede"><b>{verdict.strong}</b> {verdict.rest}</p>{/if}
  {#if live && runDev !== null}
    <button type="button" class="strip" onclick={() => go(href.tests(deviceKey(runDev)))}>
      <StatusMark kind="info" word={t("ui.health.mark.Running")} /><b>{live.title}</b>
      <span class="pbar"><i style:width="{(live.frac ?? 0.3) * 100}%"></i></span>
      <span>{live.frac === null ? "" : formatPercent(live.frac * 100)}</span>
      <span class="go">{t("ui.tests.strip.open")}<Icon name="chevron" /></span>
    </button>
  {/if}
  <Block title={t("ui.tests.devices")}>
    <RowList>
      {@render devRow("memory", "memory", t("ui.tests.memory.title"), formatBytes(dto.memory.total_bytes))}
      {#each dto.disks as d (d.number)}
        {@render devRow(d.number, diskIcon(d), diskName(d), t("ui.tests.disk.short", { n: d.number, volumes: d.volumes.map(vol).join(", ") || "—" }))}
      {/each}
    </RowList>
  </Block>
{:else if chosen === "memory"}
  {@render memoryReport()}
{:else if disk}
  {@render diskReport(disk)}
{:else}
  <PageHead title={t("ui.tests.title")} />
  <p class="empty">{t("ui.tests.disk.gone")}</p>
{/if}
{#if dto}
  {@const readDisk = reading === null ? null : dto.disks.find((d) => d.number === reading)}
  {#if readDisk}
    <ReadHistory runs={surfaceRuns(dto, readDisk)} disk={readDisk} name={diskName(readDisk)} {level} onclose={() => (reading = null)} />
  {/if}
{/if}

<HelperDialogs job="check" onconfirm={() => void stop()} />

<dialog class="confirm" bind:this={dialog} aria-labelledby="ask-title" onclose={() => (asking = null)}>
  {#if asking?.kind === "bench"}
    <h2 id="ask-title">{t("ui.tests.bench.ask", { volume: vol(asking.volume.root) })}</h2>
    <p>{t("ui.tests.bench.ask.body", { size: formatBytes((dto?.bench_file_bytes ?? 0) * 1.3), volume: vol(asking.volume.root) })}</p>
    <p>{t("ui.tests.bench.ask.time")}</p>
    <div class="acts end">
      <button type="button" class="btn" onclick={close}>{t("ui.tests.cancel")}</button>
      <button type="button" class="btn accent" onclick={() => void confirmed()}><Icon name="gauge" />{t("ui.tests.bench.start")}</button>
    </div>
  {:else if asking?.kind === "smart"}
    <h2 id="ask-title">{t("ui.tests.smart.ask.title", { name: diskName(asking.disk) })}</h2>
    <p>{t("ui.tests.smart.ask.body")}</p>
    <div class="acts end">
      <button type="button" class="btn" onclick={close}>{t("ui.tests.smart.ask.later")}</button>
      <button type="button" class="btn accent" onclick={() => void confirmed()}>{t("ui.tests.smart.read")}</button>
    </div>
  {/if}
</dialog>

<style>
  .gap {
    margin-top: 14px;
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
  .map {
    max-width: 760px;
    margin-bottom: 12px;
  }
  .h3 {
    margin: 20px 0 8px;
  }
  .muted {
    font-size: 13px;
  }
  .acts .earlier {
    margin-left: auto;
  }
  .kv {
    display: grid;
    grid-template-columns: max-content minmax(0, 1fr);
    gap: 8px 32px;
    margin: 0 0 10px;
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
  .vol + .vol {
    margin-top: 20px;
    padding-top: 16px;
    border-top: 1px solid var(--stroke2);
  }
  .smart-v {
    color: var(--ink);
    font-size: 14px;
    font-weight: 600;
  }
  .caveat {
    display: flex;
    gap: 10px;
    max-width: 72ch;
    margin: 12px 0 0;
    color: var(--ink2);
    font-size: 13px;
    line-height: 1.5;
  }
  .caveat :global(svg) {
    flex: none;
    width: 16px;
    height: 16px;
    margin-top: 1px;
  }
  .tags {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin: 14px 0 0;
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
  .fld {
    margin: 14px 0 0;
  }
  .running {
    margin-top: 24px;
  }
  .run {
    display: grid;
    grid-template-columns: minmax(200px, 360px) auto;
    gap: 6px 16px;
    align-items: center;
  }
  .run-l {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    color: var(--ink2);
    font-size: 13px;
  }
  .run-l b {
    color: var(--ink);
    font-weight: 600;
  }
  .run .btn {
    grid-column: 2;
    grid-row: 1 / 3;
  }
  .pbar {
    display: block;
    height: 4px;
    overflow: hidden;
    border-radius: 2px;
    background: var(--stroke);
  }
  .pbar i {
    display: block;
    height: 100%;
    border-radius: 2px;
    background: var(--accent);
    transition: width 0.3s cubic-bezier(0.2, 0.9, 0.3, 1);
  }
  .pbar.indeterminate i {
    width: 30%;
    animation: slide 1.4s ease-in-out infinite;
  }
  @keyframes slide {
    from {
      transform: translateX(-100%);
    }
    to {
      transform: translateX(340%);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .pbar i {
      transition: none;
    }
    .pbar.indeterminate i {
      animation: none;
    }
  }
  .facts-line {
    display: flex;
    flex-wrap: wrap;
    gap: 6px 18px;
  }
  .facts-line.bad span:nth-child(3) {
    color: var(--freeze);
    font-weight: 600;
  }
  .strip {
    display: grid;
    grid-template-columns: auto auto minmax(80px, 1fr) auto auto;
    gap: 12px;
    align-items: center;
    width: 100%;
    margin: 16px 0 0;
    padding: 10px 12px;
    border: 0;
    border-radius: 6px;
    background: var(--row);
    color: var(--ink);
    font: inherit;
    font-size: 14px;
    text-align: left;
    cursor: pointer;
  }
  .strip:hover {
    background: var(--row-hover);
  }
  .strip .go {
    display: inline-flex;
    gap: 4px;
    align-items: center;
    color: var(--ink2);
    font-size: 13px;
  }
  .speed {
    display: grid;
    gap: 14px;
    max-width: 760px;
  }
  .sp-head {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    margin-bottom: 6px;
    color: var(--ink2);
    font-size: 13px;
  }
  .sp-head b {
    color: var(--ink);
    font: 600 20px var(--fd);
  }
  .sp-head small {
    color: var(--ink2);
    font: 400 12.5px var(--f);
  }
  .sp-track {
    position: relative;
    display: block;
    height: 10px;
    overflow: hidden;
    border-radius: 6px;
    background: var(--card2);
  }
  .sp-typ {
    position: absolute;
    top: 0;
    bottom: 0;
    background: var(--stroke);
  }
  .sp-val {
    position: absolute;
    top: 3px;
    bottom: 3px;
    left: 0;
    border-radius: 3px;
    background: var(--press);
  }
  .mono,
  .out {
    font-family: var(--mono);
    font-size: 12.5px;
  }
  .out {
    max-height: 220px;
    margin: 10px 0 0;
    padding: 10px 12px;
    overflow: auto;
    border-radius: 6px;
    background: var(--hover);
    color: var(--ink2);
    white-space: pre-wrap;
  }
  .confirm {
    max-width: min(520px, calc(100vw - 32px));
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
  .confirm p {
    margin: 0 0 12px;
    color: var(--ink2);
    font-size: 14px;
    line-height: 1.5;
  }
  .acts.end {
    justify-content: flex-end;
  }
  .panel {
    display: flex;
    gap: 14px;
    align-items: flex-start;
    max-width: 72ch;
    margin: 14px 0 0;
    padding: 14px 16px;
    border: 1px solid color-mix(in srgb, var(--busy) 40%, var(--stroke));
    border-radius: 12px;
    background: var(--row);
  }
  .panel .pi {
    display: flex;
    margin-top: 2px;
  }
  .panel .pi :global(.word) {
    display: none;
  }
  .panel h3 {
    margin: 0 0 4px;
    font: 600 15px/1.4 var(--f);
  }
  .panel p {
    margin: 0;
    color: var(--ink2);
    font-size: 15px;
    line-height: 1.5;
  }
  @media (forced-colors: active) {
    .pbar i,
    .sp-val {
      forced-color-adjust: none;
      background: CanvasText;
    }
  }
</style>
