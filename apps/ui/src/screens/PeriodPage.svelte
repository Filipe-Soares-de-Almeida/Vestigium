<script lang="ts">
  import { onMount } from "svelte";

  import BackLink from "../components/BackLink.svelte";
  import ErrorView from "../components/ErrorView.svelte";
  import Loading from "../components/Loading.svelte";
  import { marks } from "../lib/api";
  import { app } from "../lib/app.svelte";
  import { formatDuration, formatTime } from "../lib/format";
  import { t } from "../lib/i18n.svelte";
  import { href } from "../lib/router.svelte";
  import type { Episode } from "../lib/types";
  import { isPressure } from "./today/day";
  import Period from "./today/Period.svelte";
  import { episodeMeta, episodeTitle } from "./today/text";

  let { start, fromDay = null }: { start: number; fromDay?: string | null } = $props();

  /** `undefined` while it loads, `null` when the history no longer has it. */
  let found = $state<{ episode: Episode; byTest: boolean } | null | undefined>(undefined);
  let failure = $state<unknown>(null);

  const level = $derived(app.settings?.settings.detail_level ?? "Simple");
  const back = $derived(
    fromDay
      ? { href: href.history({ day: fromDay, month: fromDay.slice(0, 7) }), name: t("ui.item.history") }
      : { href: href.events(), name: t("ui.item.events") },
  );

  async function load(): Promise<void> {
    failure = null;
    found = undefined;
    try {
      const list = await marks(start - 1, start + 1);
      const at = list.episodes.findIndex((e) => e.start_ms === start && isPressure(e));
      found = at < 0 ? null : { episode: list.episodes[at], byTest: list.by_test[at] ?? false };
    } catch (error) {
      failure = error;
    }
  }

  onMount(() => {
    void load();
  });
</script>

<BackLink href={back.href} label={t("ui.common.back_to", { place: back.name })} />

{#if failure !== null}
  <ErrorView error={failure} heading="h1" onretry={load} />
{:else if found === undefined}
  <h1 class="sr">{t("ui.period.title")}</h1>
  <Loading />
{:else if found === null}
  <h1>{t("ui.period.missing.title")}</h1>
  <p class="lede">{t("ui.period.missing.body")}</p>
{:else}
  {@const e = found.episode}
  <header class="head">
    <h1>{episodeTitle(e, level)}</h1>
    <p class="sub">
      {episodeMeta(e)} · {formatTime(e.start_ms)} · {formatDuration(e.end_ms - e.start_ms)}
    </p>
  </header>
  <Period episode={e} byTest={found.byTest} {level} id="period" label={episodeTitle(e, level)} {fromDay} />
{/if}

<style>
  .head {
    margin-bottom: 4px;
  }
  .sub {
    margin: 4px 0 0;
    color: var(--ink2);
    font-size: 14px;
  }
  .lede {
    max-width: 70ch;
    margin: 6px 0 18px;
    color: var(--ink2);
    font-size: 16px;
  }
</style>
