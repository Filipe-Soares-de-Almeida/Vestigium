<script lang="ts">
  import { onMount } from "svelte";

  import BackLink from "../components/BackLink.svelte";
  import ErrorView from "../components/ErrorView.svelte";
  import Loading from "../components/Loading.svelte";
  import PageHead from "../components/PageHead.svelte";
  import { marks } from "../lib/api";
  import { app } from "../lib/app.svelte";
  import { formatTime } from "../lib/format";
  import { lang, t } from "../lib/i18n.svelte";
  import { relatedLinks } from "../lib/related.svelte";
  import { href } from "../lib/router.svelte";
  import type { Episode } from "../lib/types";
  import { dayStart, isPressure } from "./today/day";
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

  /** "today at 10:02", or the day and time for an earlier day. */
  function when(ms: number): string {
    const time = formatTime(ms);
    if (dayStart(ms) === dayStart(Date.now())) return t("ui.period.when.today", { time });
    const date = new Intl.DateTimeFormat(lang(), { day: "numeric", month: "long" }).format(ms);
    return t("ui.period.when.day", { date, time });
  }

  relatedLinks(() => [
    { label: t("ui.related.live_now"), href: href.activity("now") },
    { label: t("ui.related.see_day"), href: back.href },
  ]);

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

<BackLink href={back.href} label={back.name} />

{#if failure !== null}
  <ErrorView error={failure} heading="h1" onretry={load} />
{:else if found === undefined}
  <h1 class="sr">{t("ui.period.title")}</h1>
  <Loading />
{:else if found === null}
  <PageHead title={t("ui.period.missing.title")} sub={t("ui.period.missing.body")} />
{:else}
  {@const e = found.episode}
  <PageHead title={episodeTitle(e, level)} sub="{episodeMeta(e)} · {when(e.start_ms)}" />
  <Period episode={e} byTest={found.byTest} {level} id="period" label={episodeTitle(e, level)} {fromDay} />
{/if}
