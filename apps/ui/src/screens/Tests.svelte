<script lang="ts">
  import { onMount } from "svelte";

  import ConcernMark from "../components/ConcernMark.svelte";
  import ErrorView from "../components/ErrorView.svelte";
  import Icon, { type IconName } from "../components/Icon.svelte";
  import Loading from "../components/Loading.svelte";
  import Segmented from "../components/Segmented.svelte";
  import { app } from "../lib/app.svelte";
  import { errorText } from "../lib/errors";
  import { formatNumber, formatPercent } from "../lib/format";
  import { t, tn } from "../lib/i18n.svelte";
  import { go, href } from "../lib/router.svelte";
  import { hwtests, type Started } from "../lib/tests.svelte";
  import { toast } from "../lib/toast.svelte";
  import type { BenchMode, DetailLevel, DiskDto, MemoryDiagnostic, TestRunDto, VolumeDto } from "../lib/types";
  import { at } from "./dumps/words";
  import { formatBytes } from "./occurrence/format";
  import DiskMap from "./tests/DiskMap.svelte";
  import {
    attrName,
    benchModeName,
    benchOf,
    checkOf,
    coverage,
    type Device,
    deviceKey,
    diagnosticCovered,
    diagnosticWord,
    diskState,
    kindOf,
    lastBench,
    lastCheck,
    lastQuick,
    lastSurface,
    memoryState,
    outcomeMark,
    quickCovered,
    quickOf,
    runningDevice,
    shareOf,
    span,
    surfaceEstimate,
    surfaceOf,
    surfaceWord,
    WATCHED,
  } from "./tests/words";

  let { device }: { device: string | null } = $props();

  const LEVELS: DetailLevel[] = ["Simple", "Detailed", "Technical"];
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

  let picked = $state<DetailLevel | null>(null);
  let minutes = $state(5);
  let now = $state(Date.now());
  let report = $state<HTMLElement>();
  let dialog = $state<HTMLDialogElement>();
  let asking = $state<{ kind: "bench"; volume: VolumeDto } | { kind: "smart"; disk: DiskDto } | null>(null);
  let asked = $state<number | null>(recallAsked());

  const level = $derived(picked ?? app.settings?.settings.detail_level ?? "Simple");
  const dto = $derived(hwtests.dto);
  const chosen = $derived<Device>(device === null || device === "memory" ? "memory" : Number(device));
  const disk = $derived(chosen === "memory" ? null : (dto?.disks.find((d) => d.number === chosen) ?? null));
  const running = $derived(hwtests.running);
  const runDev = $derived(dto ? runningDevice(dto, running) : null);
  const busy = $derived(hwtests.busy || running !== null || dto?.scan != null);
  const diag = $derived(dto?.diagnostics[0] ?? null);
  // A request counts until a run of the Windows diagnostic newer than it shows up.
  const waiting = $derived(asked !== null && !(diag && diag.ts_ms > asked));

  onMount(() => {
    void hwtests.load().then(readQuickDetails);
    const timer = setInterval(() => (now = Date.now()), 1000);
    return () => clearInterval(timer);
  });

  // Another device starts at the top of its report; progress updates keep the place.
  $effect(() => {
    void chosen;
    report?.scrollTo(0, 0);
  });

  /** SSD and NVMe health reads at once; an HDD only when asked, since reading wakes it. */
  function readQuickDetails(): void {
    for (const d of hwtests.dto?.disks ?? []) {
      if (d.media !== "Hdd" && !hwtests.details[d.number]) void hwtests.readDetail(d);
    }
  }

  async function pickLevel(next: DetailLevel): Promise<void> {
    picked = next;
    const saved = app.settings?.settings;
    if (!app.editable || !saved || saved.detail_level === next) return;
    try {
      await app.saveSettings({ ...saved, detail_level: next });
    } catch (error) {
      toast.show(errorText(error));
    }
  }

  function retry(): void {
    hwtests.error = null;
    void hwtests.load();
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
      return { title: t("ui.tests.live.check", { volume: vol(root) }), note: t("ui.tests.live.check.note"), frac: null, facts: [], bad: false, stoppable: false };
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
    if (s) return { name: t("ui.tests.hist.surface"), what: s.unreadable_blocks ? tn("ui.tests.surface.unreadable", s.unreadable_blocks) : t("ui.tests.hist.read", { share: shareOf(s.to_bytes - s.from_bytes, s.disk_bytes) }) };
    if (c) return { name: t("ui.tests.hist.check", { volume: vol(c.volume) }), what: t(`ui.tests.check.${c.result}`) };
    const seq = b?.results.find((x) => x.mode === "SeqRead");
    return { name: t("ui.tests.hist.bench", { volume: vol(b?.volume ?? "") }), what: seq ? t("ui.tests.bench.seq", { value: formatNumber(seq.mb_per_s) }) : "" };
  }
</script>

{#snippet devRow(id: Device, icon: IconName, name: string, sub: string)}
  {@const s = id === "memory" ? memoryState(dto!, runDev === "memory") : diskState(dto!, dto!.disks.find((d) => d.number === id)!, hwtests.details[id as number] ?? null, runDev === id)}
  <button type="button" class="dev" aria-current={chosen === id ? "true" : undefined} onclick={() => go(href.tests(deviceKey(id)))}>
    <span class="icochip"><Icon name={icon} /></span>
    <span><b>{name}</b><small>{sub}</small><small class="st"><ConcernMark mark={s.mark} word={false} />{s.line}</small></span>
  </button>
{/snippet}

{#snippet liveBlock()}
  {#if live}
    <div class="running" aria-live="polite">
      <div class="rhead">
        <div>
          <h3><ConcernMark mark="Running" word={false} />{live.title}</h3>
          <p>{live.note}</p>
        </div>
        {#if live.stoppable}
          <button type="button" class="btn" onclick={() => void hwtests.stop()}><Icon name="stop" />{t("ui.tests.stop")}</button>
        {/if}
      </div>
      <div class="prog" class:indeterminate={live.frac === null} role="progressbar" aria-valuemin="0" aria-valuemax="100" aria-valuenow={live.frac === null ? undefined : Math.round(live.frac * 100)}>
        <i style:width="{(live.frac ?? 0.3) * 100}%"></i>
      </div>
      {#if live.facts.length}
        <p class="facts-line" class:bad={live.bad}>{#each live.facts as f, i (i)}<span>{f}</span>{/each}</p>
      {/if}
      {#if "read" in live && live.bytes}
        <DiskMap read={live.read} diskBytes={live.bytes} name={live.title} {level} />
      {/if}
    </div>
  {/if}
{/snippet}

{#snippet pills(items: [IconName, string][])}
  {#each items as [icon, text] (text)}
    <span class="pill2"><Icon name={icon} />{text}</span>
  {/each}
{/snippet}

{#snippet history(d: Device)}
  {@const rows = historyOf(d)}
  <div class="blk">
    <h3><Icon name="clock" />{t(d === "memory" ? "ui.tests.hist.memory" : "ui.tests.hist.disk")}</h3>
    {#if rows.length}
      <div class="hlist">
        {#each rows as row, i (i)}
          {#if row.run}
            {@const line = runLine(row.run)}
            <div class="hline"><ConcernMark mark={outcomeMark(row.run)} word={false} /><span><b>{line.name}</b><small>{line.what}</small></span><span class="when">{at(row.ts)}</span></div>
          {:else if row.diag}
            {@const w = diagnosticWord(row.diag)}
            <div class="hline"><ConcernMark mark={w.mark} word={false} /><span><b>{t("ui.tests.hist.diag")}</b><small>{w.text}</small></span><span class="when">{at(row.ts)}</span></div>
          {/if}
        {/each}
      </div>
    {:else}
      <p class="muted">{t("ui.tests.hist.none")}</p>
    {/if}
  </div>
{/snippet}

{#snippet memoryReport()}
  {@const s = memoryState(dto!, runDev === "memory")}
  {@const quick = lastQuick(dto!)}
  {@const q = quick ? quickOf(quick) : null}
  <header class="rh">
    <span class="icochip"><Icon name="memory" /></span>
    <div>
      <h2 id="report-title">{t("ui.tests.memory.title")}</h2>
      <p>{t("ui.tests.memory.sub", { total: formatBytes(dto!.memory.total_bytes), free: formatBytes(dto!.memory.available_bytes) })}</p>
    </div>
  </header>
  <p class="state"><ConcernMark mark={s.mark} word={false} />{s.line}</p>

  <div class="blk">
    <h3><Icon name="windows" />{t("ui.tests.diag.title")}<span class="sub">{t("ui.tests.diag.when")}</span></h3>
    {#if diag}
      {@const w = diagnosticWord(diag)}
      {@const share = coverage(diag)}
      <p class="res"><ConcernMark mark={w.mark} word={false} /><b>{w.text}</b><span class="when">{at(diag.ts_ms)}{diag.detail ? ` · ${span(diag.detail.duration_s * 1000)}` : ""}</span></p>
      {#if share !== null}<div class="cov" role="img" aria-label={t("ui.tests.diag.coverage")}><i style:width="{share * 100}%"></i></div>{/if}
      <p class="note">{diagnosticCovered(diag, level)}</p>
    {:else}
      <p class="muted">{t("ui.tests.diag.never")}</p>
    {/if}
    {#if waiting && asked !== null}
      <p class="wait"><Icon name="restart" /><span>{t("ui.tests.diag.waiting", { when: at(asked) })}</span></p>
    {/if}
    <div class="acts">
      <button type="button" class="btn" class:accent={!diag && !waiting && quick !== null} disabled={busy} onclick={() => void diagnostic()}>
        <Icon name="restart" />{t(diag || waiting ? "ui.tests.diag.again" : "ui.tests.diag.start")}
      </button>
      <span class="pills">{@render pills([["lock", t("ui.tests.pill.permission")], ["restart", t("ui.tests.pill.restart")]])}</span>
    </div>
  </div>

  <div class="blk">
    <h3><Icon name="memory" />{t("ui.tests.quick.title")}<span class="sub">{t("ui.tests.quick.when")}</span></h3>
    {#if quick && q}
      <p class="res">
        <ConcernMark mark={outcomeMark(quick)} word={false} />
        <b>{q.error_count ? tn("ui.tests.quick.errors", q.error_count) : t(quick.outcome === "Interrupted" ? "ui.tests.result.stopped" : quick.outcome === "Failed" ? "ui.tests.result.failed" : "ui.tests.result.clean")}</b>
        <span class="when">{at(quick.ended_ms)} · {span(quick.ended_ms - quick.started_ms)} · {tn("ui.tests.quick.rounds", q.rounds)}</span>
      </p>
      {#if q.failure}
        <p class="note">{t("ui.tests.quick.failure")}{level === "Technical" ? ` (${q.failure})` : ""}</p>
      {:else}
        <div class="cov" role="img" aria-label={t("ui.tests.quick.coverage")}><i style:width="{(q.tested_bytes / Math.max(1, q.total_bytes)) * 100}%"></i></div>
        <p class="note">{quickCovered(q, level)}</p>
      {/if}
      {#if q.errors.length && level !== "Simple"}
        <table class="tab">
          <thead><tr><th>{t("ui.tests.err.where")}</th><th>{t("ui.tests.err.pattern")}</th><th>{t("ui.tests.err.expected")}</th><th>{t("ui.tests.err.read")}</th></tr></thead>
          <tbody>
            {#each q.errors as e, i (i)}
              <tr><td>{t("ui.tests.err.offset", { at: formatBytes(e.offset) })}</td><td>{t(`ui.tests.pattern.${e.pattern}`)}</td><td class="mono">0x{e.expected.toString(16).padStart(16, "0")}</td><td class="mono">0x{e.actual.toString(16).padStart(16, "0")}</td></tr>
            {/each}
          </tbody>
        </table>
      {/if}
      <p class="honest">{t(q.error_count ? "ui.tests.quick.honest.errors" : "ui.tests.quick.honest")}</p>
    {:else}
      <p class="muted">{t("ui.tests.quick.never")}</p>
    {/if}
    {#if dto!.memory.quick_bytes === null}
      <p class="note">{t("ui.tests.quick.low", { free: formatBytes(dto!.memory.available_bytes) })}</p>
    {:else}
      <p class="note">{t("ui.tests.quick.will", { tested: formatBytes(dto!.memory.quick_bytes), total: formatBytes(dto!.memory.total_bytes) })}</p>
      <div class="acts">
        <Segmented
          label={t("ui.tests.quick.length")}
          options={dto!.memory.minutes.map((m) => ({ value: String(m), label: tn("ui.tests.span.min", m), hint: "" }))}
          value={String(minutes)}
          onchange={(v: string) => (minutes = Number(v))}
        />
        <button type="button" class="btn" class:accent={!quick && !diag} disabled={busy} onclick={() => void hwtests.memory(minutes).then(started)}>
          <Icon name="start" />{t("ui.tests.quick.start")}
        </button>
      </div>
    {/if}
  </div>

  <div class="blk">
    <h3><Icon name="outside" />{t("ui.tests.memtest.title")}</h3>
    <p class="note">{t("ui.tests.memtest.body")}</p>
  </div>
  {@render history("memory")}
{/snippet}

{#snippet diskReport(d: DiskDto)}
  {@const s = diskState(dto!, d, hwtests.details[d.number] ?? null, runDev === d.number)}
  {@const detail = hwtests.details[d.number] ?? null}
  {@const scanRun = lastSurface(dto!, d)}
  {@const scan = scanRun ? surfaceOf(scanRun) : null}
  {@const resumable = scanRun?.outcome === "Interrupted" && scan !== null && scan.to_bytes < scan.disk_bytes}
  <header class="rh">
    <span class="icochip"><Icon name={diskIcon(d)} /></span>
    <div>
      <h2 id="report-title">{diskName(d)}</h2>
      <p>{t("ui.tests.disk.sub", { n: d.number, bus: t(`ui.tests.bus.${d.bus}`), volumes: d.volumes.map(vol).join(", ") || "—" })}</p>
    </div>
  </header>
  <p class="state"><ConcernMark mark={s.mark} word={false} />{s.line}</p>

  <div class="blk">
    <h3><Icon name="smart" />{t("ui.tests.smart.title")}<span class="sub">{t(d.media === "Hdd" ? "ui.tests.smart.when.hdd" : "ui.tests.smart.when")}</span></h3>
    {#if !detail}
      <p class="muted">{t(d.media === "Hdd" ? "ui.tests.smart.ask" : "ui.tests.smart.reading")}</p>
      {#if d.media === "Hdd"}
        <div class="acts"><button type="button" class="btn" onclick={() => smart(d)}><Icon name="smart" />{t("ui.tests.smart.read")}</button></div>
      {/if}
    {:else if detail.nvme}
      {@const n = detail.nvme}
      <dl class="facts">
        <div><dt>{t("ui.tests.nvme.used")}</dt><dd>{#if n.health.percentage_used >= 90}<ConcernMark mark="Attention" word={false} />{/if}<b>{formatPercent(n.health.percentage_used)}</b></dd></div>
        <div><dt>{t("ui.tests.nvme.spare")}</dt><dd>{#if n.health.available_spare_pct < n.health.spare_threshold_pct}<ConcernMark mark="Attention" word={false} />{/if}<b>{formatPercent(n.health.available_spare_pct)}</b></dd></div>
        <div><dt>{t("ui.tests.nvme.temp")}</dt><dd><b>{n.health.temperature_c} °C</b></dd></div>
        {#if level !== "Simple"}
          <div><dt>{t("ui.tests.nvme.written")}</dt><dd><b>{formatBytes(n.data_written_bytes)}</b></dd></div>
          <div><dt>{t("ui.tests.nvme.hours")}</dt><dd><b>{formatNumber(n.power_on_hours)}</b></dd></div>
          <div><dt>{t("ui.tests.nvme.unsafe")}</dt><dd><b>{formatNumber(n.unsafe_shutdowns)}</b></dd></div>
          <div><dt>{t("ui.tests.nvme.media")}</dt><dd>{#if n.health.media_errors}<ConcernMark mark="Critical" word={false} />{/if}<b>{formatNumber(n.health.media_errors)}</b></dd></div>
        {/if}
      </dl>
      {#if level === "Technical"}<p class="note mono">{t("ui.tests.nvme.tech", { warning: n.health.critical_warning.toString(16).padStart(2, "0"), errors: formatNumber(n.error_log_entries) })}</p>{/if}
    {:else if detail.health}
      {@const attrs = detail.health.attributes}
      {@const flagged = attrs.filter((a) => WATCHED.includes(a.id) && a.raw > 0)}
      <p class="res">
        {#if detail.health.predict_failure}
          <ConcernMark mark="Critical" word={false} /><b>{t("ui.tests.smart.predict")}</b>
        {:else if flagged.length}
          <ConcernMark mark="Attention" word={false} /><b>{flagged.map((a) => `${attrName(a.id)}: ${formatNumber(a.raw)}`).join(" · ")}</b>
        {:else}
          <ConcernMark mark="Ok" word={false} /><b>{t("ui.tests.smart.ok")}</b>
        {/if}
      </p>
      {#if level !== "Simple" && attrs.length}
        <table class="tab num">
          <thead><tr>{#if level === "Technical"}<th>ID</th>{/if}<th>{t("ui.tests.smart.attr")}</th><th>{t("ui.tests.smart.current")}</th><th>{t("ui.tests.smart.worst")}</th><th>{t("ui.tests.smart.raw")}</th></tr></thead>
          <tbody>
            {#each attrs as a (a.id)}
              <tr class:flag={WATCHED.includes(a.id) && a.raw > 0}>{#if level === "Technical"}<td class="mono">{a.id} (0x{a.id.toString(16).toUpperCase().padStart(2, "0")})</td>{/if}<td>{attrName(a.id)}</td><td>{a.current}</td><td>{a.worst}</td><td><b>{formatNumber(a.raw)}</b></td></tr>
            {/each}
          </tbody>
        </table>
      {/if}
    {:else}
      <p class="muted">{t("ui.tests.smart.none")}</p>
    {/if}
  </div>

  <div class="blk">
    <h3><Icon name="scan" />{t("ui.tests.surface.title")}<span class="sub">{t("ui.tests.surface.when")}</span></h3>
    {#if scanRun && scan}
      {@const w = surfaceWord(scanRun, scan, d)}
      <p class="res"><ConcernMark mark={w.mark} word={false} /><b>{w.text}</b><span class="when">{at(scanRun.ended_ms)} · {span(scanRun.ended_ms - scanRun.started_ms)}{scan.from_bytes === 0 && scan.to_bytes >= scan.disk_bytes ? ` · ${t("ui.tests.surface.whole")}` : ""}</span></p>
      <DiskMap {scan} diskBytes={scan.disk_bytes} name={diskName(d)} {level} />
      {#if level !== "Simple" && scan.worst.length}
        <h4>{t("ui.tests.surface.worst")}</h4>
        <ul class="worst">
          {#each scan.worst.slice(0, 5) as b, i (i)}
            <li><span>{t("ui.tests.err.offset", { at: formatBytes(b.offset) })}</span><b>{formatNumber(b.ms / 1000, 1)} s</b></li>
          {/each}
        </ul>
      {/if}
      {#if level === "Technical" && scan.unreadable.length}
        <p class="note mono">{t("ui.tests.surface.sectors", { list: scan.unreadable.slice(0, 20).map((o) => formatNumber(o / scan.sector_bytes)).join(", ") })}</p>
      {/if}
      {#if d.media !== "Hdd"}<p class="honest">{t("ui.tests.surface.ssd")}</p>{/if}
    {:else}
      <p class="muted">{t("ui.tests.surface.never")}</p>
    {/if}
    <div class="acts">
      {#if resumable && scan}
        <button type="button" class="btn accent" disabled={busy} onclick={() => void hwtests.surface(d.number, Math.floor(scan.to_bytes / 1048576)).then(started)}>
          <Icon name="resume" />{t("ui.tests.surface.resume", { share: shareOf(scan.to_bytes, scan.disk_bytes) })}
        </button>
      {/if}
      <button type="button" class="btn" class:accent={!scanRun} disabled={busy} onclick={() => void hwtests.surface(d.number, 0).then(started)}>
        <Icon name="scan" />{t(resumable ? "ui.tests.surface.restart" : "ui.tests.surface.start")}
      </button>
      <span class="pills">{@render pills([["clock", surfaceEstimate(d)], ["lock", t("ui.tests.pill.permission")], ["shield", t("ui.tests.pill.read_only")]])}</span>
    </div>
  </div>

  {#each volumesOf(d) as v (v.root)}
    {@const check = lastCheck(dto!, v.root)}
    {@const c = check ? checkOf(check) : null}
    {@const benchRun = lastBench(dto!, v.root)}
    {@const b = benchRun ? benchOf(benchRun) : null}
    <div class="blk">
      <h3><Icon name="fsck" />{t("ui.tests.check.title", { volume: vol(v.root) })}<span class="sub">{t("ui.tests.check.when")}</span></h3>
      {#if v.file_system !== "NTFS"}
        <p class="muted">{t("ui.tests.check.not_ntfs", { fs: v.file_system ?? "?" })}</p>
      {:else}
        {#if check && c}
          <p class="res"><ConcernMark mark={c.result === "ProblemsFound" ? "Attention" : c.result === "NoProblems" ? "Ok" : "Unknown"} word={false} /><b>{t(`ui.tests.check.${c.result}`)}</b><span class="when">{at(check.ended_ms)}{level === "Technical" && c.exit_code !== null ? ` · chkdsk ${v.root.slice(0, 2)} /scan · ${c.exit_code}` : ""}</span></p>
          {#if c.result === "ProblemsFound"}<p class="note">{t("ui.tests.check.repair", { volume: v.root.slice(0, 2) })}</p>{/if}
          {#if level === "Technical" && c.output}<pre class="out">{c.output}</pre>{/if}
        {:else}
          <p class="muted">{t("ui.tests.check.never")}</p>
        {/if}
        <div class="acts">
          <button type="button" class="btn" disabled={busy} onclick={() => void hwtests.volume(v.root).then(started)}><Icon name="fsck" />{t("ui.tests.check.start")}</button>
          <span class="pills">{@render pills([["clock", t("ui.tests.check.length")], ["lock", t("ui.tests.pill.permission")]])}</span>
        </div>
      {/if}
    </div>

    <div class="blk">
      <h3><Icon name="gauge" />{t("ui.tests.bench.title", { volume: vol(v.root) })}<span class="sub">{t("ui.tests.bench.when")}</span></h3>
      {#if benchRun && b}
        <p class="res"><ConcernMark mark={benchRun.outcome === "Clean" ? "Ok" : "Partial"} word={false} /><b>{t(benchRun.outcome === "Clean" ? "ui.tests.bench.done" : benchRun.outcome === "Failed" ? "ui.tests.result.failed" : "ui.tests.bench.stopped")}</b><span class="when">{at(benchRun.ended_ms)} · {t(`ui.tests.bus.${b.bus}`)}</span></p>
        <div class="bench">
          {#each b.results.filter((r) => level !== "Simple" || r.mode === "SeqRead" || r.mode === "RandRead1") as r (r.mode)}
            {@const rg = d.ranges.find((x) => x.mode === r.mode)}
            {@const max = Math.max(rg?.high ?? 0, r.mb_per_s) * 1.1 || 1}
            <div class="brow">
              <span class="bname">{benchModeName(r.mode, level)}</span>
              <span class="bbar" role="img" aria-label={rg ? t("ui.tests.bench.against", { value: formatNumber(r.mb_per_s, 1), low: formatNumber(rg.low, 1), high: formatNumber(rg.high, 1) }) : t("ui.tests.bench.value", { value: formatNumber(r.mb_per_s, 1) })}>
                {#if rg}<i class="rg" style:left="{(rg.low / max) * 100}%" style:width="{((rg.high - rg.low) / max) * 100}%"></i>{/if}
                <i class="val" style:width="{(r.mb_per_s / max) * 100}%"></i>
              </span>
              <span class="bnum"><b>{formatNumber(r.mb_per_s, r.mb_per_s < 10 ? 1 : 0)}</b> MB/s{#if rg && r.mb_per_s < rg.low}<ConcernMark mark="Attention" word={false} />{/if}</span>
            </div>
          {/each}
        </div>
        <p class="note">
          {d.ranges.length ? t(`ui.tests.bench.range.${d.media}`, { bus: t(`ui.tests.bus.${d.bus}`) }) : t("ui.tests.bench.no_range")}
          {t("ui.tests.bench.written", { written: formatBytes(b.written_bytes) })}
          {#if b.outside_io}<b>{t("ui.tests.bench.outside")}</b>{/if}
        </p>
      {:else}
        <p class="muted">{t("ui.tests.bench.never")}</p>
      {/if}
      <div class="acts">
        <button type="button" class="btn" disabled={busy} onclick={() => bench(v)}><Icon name="gauge" />{t(benchRun ? "ui.tests.bench.again" : "ui.tests.bench.start")}</button>
        <span class="pills">{@render pills(d.media === "Ssd" ? [["clock", t("ui.tests.bench.length")], ["write", t("ui.tests.bench.writes", { size: formatBytes(dto!.bench_file_bytes * 1.3) })]] : [["clock", t("ui.tests.bench.length")]])}</span>
      </div>
    </div>
  {/each}
  {@render history(d.number)}
{/snippet}

<div class="screen-in">
  <div class="top">
    <h1>{t("ui.tests.title")}</h1>
    <Segmented
      label={t("ui.common.detail.label")}
      options={LEVELS.map((value) => ({ value, label: t(`ui.common.detail.${value}`), hint: t(`ui.common.detail.${value}.hint`) }))}
      value={level}
      onchange={pickLevel}
    />
  </div>

  {#if dto === null && hwtests.error !== null}
    <div class="gap"><ErrorView error={hwtests.error} heading="h2" onretry={retry} /></div>
  {:else if dto === null}
    <Loading />
  {:else}
    {#if verdict}<p class="lede"><b>{verdict.strong}</b> {verdict.rest}</p>{/if}
    {#if live && runDev !== null && runDev !== chosen}
      <button type="button" class="strip" onclick={() => go(href.tests(deviceKey(runDev)))}>
        <ConcernMark mark="Running" word={false} /><b>{live.title}</b>
        <span class="prog"><i style:width="{(live.frac ?? 0.3) * 100}%"></i></span>
        <span>{live.frac === null ? "" : formatPercent(live.frac * 100)}</span>
        <span class="go">{t("ui.tests.strip.open")}<Icon name="chevron" /></span>
      </button>
    {/if}
    <div class="pane2">
      <section class="card devs" aria-label={t("ui.tests.devices")}>
        <h2>{t("ui.tests.group.memory")}</h2>
        {@render devRow("memory", "memory", t("ui.tests.memory.title"), formatBytes(dto.memory.total_bytes))}
        <h2>{t("ui.tests.group.disks")}</h2>
        {#each dto.disks as d (d.number)}
          {@render devRow(d.number, diskIcon(d), diskName(d), t("ui.tests.disk.short", { n: d.number, volumes: d.volumes.map(vol).join(", ") || "—" }))}
        {/each}
      </section>
      <!-- The report scrolls on its own, so the keyboard needs to reach it. -->
      <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
      <section class="card report" bind:this={report} tabindex="0" aria-labelledby="report-title">
        {#if runDev === chosen}{@render liveBlock()}{/if}
        {#if chosen === "memory"}
          {@render memoryReport()}
        {:else if disk}
          {@render diskReport(disk)}
        {:else}
          <p class="muted">{t("ui.tests.disk.gone")}</p>
        {/if}
      </section>
    </div>
  {/if}
</div>

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
  .screen-in {
    display: flex;
    flex: 1 1 auto;
    flex-direction: column;
    min-height: 0;
  }
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
    flex: none;
  }
  .pane2 {
    display: grid;
    flex: 1 1 auto;
    grid-template-columns: 268px minmax(0, 1fr);
    gap: 14px;
    align-items: stretch;
    min-height: 0;
  }
  .devs {
    display: grid;
    align-content: start;
    align-self: start;
    gap: 2px;
    max-height: 100%;
    padding: 6px;
    overflow: auto;
  }
  .devs h2 {
    margin: 0;
    padding: 10px 10px 4px;
    color: var(--ink2);
    font: 600 13px/1.3 var(--f);
  }
  .dev {
    display: grid;
    grid-template-columns: 36px minmax(0, 1fr);
    gap: 10px;
    align-items: center;
    width: 100%;
    padding: 9px 10px;
    border: 0;
    border-radius: 10px;
    background: none;
    color: var(--ink);
    font: inherit;
    text-align: left;
    cursor: pointer;
  }
  .dev:hover {
    background: var(--card2);
  }
  .dev[aria-current="true"] {
    background: var(--card2);
    box-shadow: inset 0 0 0 1px var(--stroke);
  }
  .dev .icochip {
    width: 36px;
    height: 36px;
  }
  .dev b {
    display: block;
    font-weight: 600;
    line-height: 1.3;
  }
  .dev small {
    display: block;
    margin-top: 2px;
    color: var(--ink2);
    font-size: 12.5px;
    line-height: 1.35;
  }
  .dev .st {
    display: flex;
    gap: 6px;
    align-items: flex-start;
  }
  .report {
    display: grid;
    align-content: start;
    gap: 16px;
    max-height: 100%;
    min-height: 0;
    overflow: auto;
    outline: none;
    scrollbar-color: var(--stroke) transparent;
    scrollbar-gutter: stable;
  }
  .report:focus-visible {
    box-shadow: 0 0 0 2px var(--ink);
  }
  .rh {
    display: grid;
    grid-template-columns: 44px minmax(0, 1fr);
    gap: 12px;
    align-items: center;
  }
  .rh h2 {
    margin: 0;
    font: 600 22px/1.3 var(--fd);
  }
  .rh p {
    margin: 3px 0 0;
    color: var(--ink2);
    font-size: 14px;
  }
  .state,
  .res {
    display: flex;
    flex-wrap: wrap;
    gap: 6px 10px;
    align-items: center;
    margin: 0;
    font-size: 14px;
  }
  .res b {
    font-weight: 600;
  }
  .when {
    color: var(--ink2);
    font-size: 13px;
  }
  .blk {
    display: grid;
    gap: 8px;
    padding-top: 14px;
    border-top: 1px solid var(--stroke2);
  }
  .blk h3 {
    display: flex;
    gap: 8px;
    align-items: center;
    margin: 0;
    font: 600 15px/1.3 var(--f);
  }
  .blk h3 :global(svg) {
    color: var(--ink2);
  }
  .blk h3 .sub {
    margin-left: auto;
    color: var(--ink2);
    font: 400 12.5px/1.35 var(--f);
  }
  h4 {
    margin: 4px 0 0;
    color: var(--ink2);
    font: 600 13px/1.3 var(--f);
  }
  .muted,
  .note {
    max-width: 70ch;
    margin: 0;
    color: var(--ink2);
    font-size: 13.5px;
    line-height: 1.45;
  }
  .note b {
    color: var(--ink);
  }
  .honest {
    max-width: 70ch;
    margin: 0;
    padding-left: 12px;
    border-left: 1px solid var(--stroke);
    color: var(--ink2);
    font-size: 13px;
    line-height: 1.45;
  }
  .wait {
    display: flex;
    gap: 10px;
    align-items: flex-start;
    margin: 0;
    padding: 10px 12px;
    border-radius: 10px;
    background: var(--card2);
    font-size: 14px;
    line-height: 1.45;
  }
  .wait :global(svg) {
    flex: none;
    margin-top: 3px;
    color: var(--ink2);
  }
  .acts {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    align-items: center;
    margin-top: 2px;
  }
  .acts.end {
    justify-content: flex-end;
  }
  .pills {
    display: inline-flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .pill2 {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 1px 8px;
    border-radius: 999px;
    background: var(--card2);
    color: var(--ink2);
    font: 600 12px/1.6 var(--f);
  }
  .pill2 :global(svg) {
    width: 13px;
    height: 13px;
  }
  .cov,
  .prog {
    height: 8px;
    overflow: hidden;
    border: 1px solid var(--stroke2);
    border-radius: 999px;
    background: var(--card2);
  }
  .cov i,
  .prog i {
    display: block;
    height: 100%;
    border-radius: 999px;
    background: var(--ink2);
  }
  .prog {
    height: 10px;
  }
  .prog i {
    background: var(--accent);
    transition: width 0.3s cubic-bezier(0.2, 0.9, 0.3, 1);
  }
  .prog.indeterminate i {
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
    .prog i {
      transition: none;
    }
    .prog.indeterminate i {
      animation: none;
    }
  }
  .running {
    display: grid;
    gap: 8px;
    padding-bottom: 14px;
    border-bottom: 1px solid var(--stroke2);
  }
  .rhead {
    display: flex;
    gap: 12px;
    align-items: flex-start;
    justify-content: space-between;
  }
  .rhead h3 {
    display: flex;
    gap: 8px;
    align-items: center;
    margin: 0;
    font: 600 18px/1.3 var(--fd);
  }
  .rhead p {
    margin: 2px 0 0;
    color: var(--ink2);
    font-size: 13px;
  }
  .facts-line {
    display: flex;
    flex-wrap: wrap;
    gap: 6px 18px;
    margin: 0;
    color: var(--ink2);
    font-size: 14px;
    font-variant-numeric: tabular-nums;
  }
  .facts-line.bad span:nth-child(3) {
    color: var(--freeze);
    font-weight: 600;
  }
  .strip {
    display: grid;
    flex: none;
    grid-template-columns: 16px auto minmax(80px, 1fr) auto auto;
    gap: 12px;
    align-items: center;
    width: 100%;
    margin: 0 0 10px;
    padding: 10px 14px;
    border: 1px solid var(--stroke);
    border-radius: 10px;
    background: var(--card);
    color: var(--ink);
    font: inherit;
    font-size: 14px;
    text-align: left;
    cursor: pointer;
  }
  .strip:hover {
    background: var(--card2);
  }
  .strip .prog {
    height: 6px;
  }
  .strip .go {
    display: inline-flex;
    gap: 4px;
    align-items: center;
    color: var(--ink2);
    font-size: 13px;
    font-weight: 600;
  }
  .facts {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
    gap: 8px;
    margin: 0;
  }
  .facts div {
    padding: 9px 12px;
    border-radius: 10px;
    background: var(--card2);
  }
  .facts dt {
    color: var(--ink2);
    font-size: 12.5px;
  }
  .facts dd {
    display: flex;
    gap: 6px;
    align-items: center;
    margin: 2px 0 0;
  }
  .facts dd b {
    font: 600 16px var(--f);
    font-variant-numeric: tabular-nums;
  }
  .tab {
    width: 100%;
    border-collapse: collapse;
    font-size: 13.5px;
    font-variant-numeric: tabular-nums;
  }
  .tab th {
    padding: 5px 8px;
    border-bottom: 1px solid var(--stroke2);
    color: var(--ink2);
    font-size: 12.5px;
    font-weight: 600;
    text-align: left;
  }
  .tab td {
    padding: 5px 8px;
    border-bottom: 1px solid var(--stroke2);
  }
  .tab.num td:nth-last-child(-n + 3),
  .tab.num th:nth-last-child(-n + 3) {
    text-align: right;
  }
  .tab tr.flag td {
    background: var(--press-t);
  }
  .mono,
  .out {
    font-family: ui-monospace, "Cascadia Mono", Consolas, monospace;
    font-size: 12.5px;
  }
  .out {
    max-height: 220px;
    margin: 0;
    padding: 10px 12px;
    overflow: auto;
    border-radius: 10px;
    background: var(--card2);
    color: var(--ink2);
    white-space: pre-wrap;
  }
  .worst {
    display: grid;
    gap: 2px;
    margin: 0;
    padding: 0;
    font-size: 13.5px;
    list-style: none;
  }
  .worst li {
    display: flex;
    justify-content: space-between;
    padding: 4px 0;
    border-bottom: 1px solid var(--stroke2);
  }
  .bench {
    display: grid;
    gap: 6px;
  }
  .brow {
    display: grid;
    grid-template-columns: minmax(150px, 1fr) minmax(0, 1.4fr) 112px;
    gap: 12px;
    align-items: center;
    font-size: 13.5px;
  }
  .bname {
    color: var(--ink2);
  }
  .bbar {
    position: relative;
    height: 12px;
    border-radius: 999px;
    background: var(--card2);
  }
  .bbar .rg {
    position: absolute;
    top: 0;
    bottom: 0;
    border-radius: 999px;
    background: var(--calm-t);
    box-shadow: inset 0 0 0 1px var(--calm);
  }
  .bbar .val {
    position: absolute;
    top: 3px;
    bottom: 3px;
    left: 0;
    border-radius: 999px;
    background: var(--ink);
  }
  .bnum {
    display: inline-flex;
    gap: 4px;
    align-items: center;
    justify-content: flex-end;
    font-variant-numeric: tabular-nums;
  }
  .hlist {
    display: grid;
    gap: 2px;
  }
  .hline {
    display: grid;
    grid-template-columns: 18px minmax(0, 1fr) auto;
    gap: 10px;
    align-items: center;
    padding: 8px 4px;
    border-bottom: 1px solid var(--stroke2);
    font-size: 14px;
  }
  .hline:last-child {
    border-bottom: 0;
  }
  .hline b {
    display: block;
    font-weight: 600;
  }
  .hline small {
    color: var(--ink2);
    font-size: 12.5px;
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
    margin: 0 0 10px;
    font: 600 18px/1.3 var(--fd);
  }
  .confirm p {
    margin: 0 0 12px;
    color: var(--ink2);
    font-size: 14px;
    line-height: 1.5;
  }
  @container content (max-width: 680px) {
    .pane2 {
      grid-template-columns: minmax(0, 1fr);
    }
  }
  @media (max-height: 520px) {
    .report {
      max-height: none;
      overflow: visible;
    }
  }
  @media (forced-colors: active) {
    .cov i,
    .prog i,
    .bbar .val {
      forced-color-adjust: none;
      background: CanvasText;
    }
  }
</style>
