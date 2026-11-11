<script lang="ts">
  import Block from "../../components/Block.svelte";
  import SelectField from "../../components/SelectField.svelte";
  import { formatPercent } from "../../lib/format";
  import { t } from "../../lib/i18n.svelte";
  import { here, href } from "../../lib/router.svelte";
  import type { ProcessSample, ProcessTop } from "../../lib/types";
  import { formatBytes, formatRate } from "./lanes";
  import { type ProgramSort, sortPrograms } from "./sort";

  let {
    top,
    programs,
    notice,
  }: {
    top: ProcessTop | null;
    programs: ProcessTop | null;
    /** Set while the monitor is silent: the list is old. */
    notice?: string;
  } = $props();

  let sort = $state<ProgramSort>("cpu");
  /** Every program when the watcher sends them all, else its busiest by processor; five show. */
  const list = $derived<ProcessSample[]>(programs?.entries ?? top?.entries ?? []);
  const rows = $derived(sortPrograms(list, sort).slice(0, 5));
  const options = $derived([
    { value: "cpu", text: t("ui.now.top.cpu") },
    { value: "memory", text: t("ui.now.top.memory") },
    { value: "disk", text: t("ui.now.top.disk") },
  ]);
</script>

{#snippet sorter()}
  <label class="inl">
    <span>{t("ui.now.sort")}</span>
    <SelectField value={sort} {options} label={t("ui.now.sort")} onchange={(value) => (sort = value as ProgramSort)} />
  </label>
{/snippet}

<Block title={t("ui.now.top.title")} actions={sorter} {notice}>
  {#if rows.length === 0 && notice !== undefined}
    <!-- Nothing to list: the notice says nothing arrived. -->
  {:else if rows.length === 0}
    <p class="empty">{t("ui.now.top.waiting")}</p>
  {:else}
    <table class="tbl">
      <thead>
        <tr>
          <th scope="col">{t("ui.now.top.program")}</th>
          <th scope="col" class="num">{t("ui.now.top.cpu")}</th>
          <th scope="col" class="num">{t("ui.now.top.memory")}</th>
          <th scope="col" class="num">{t("ui.now.top.disk")}</th>
        </tr>
      </thead>
      <tbody>
        {#each rows as row (row.pid)}
          <tr>
            <td class="nm" title={row.name}>
              <a class="row-a" href={href.detail("process", row.name, { from: here(), label: t("ui.now.title") })}>{row.name}</a>
            </td>
            <td class="num">{formatPercent(row.cpu_pct, 1)}</td>
            <td class="num">{formatBytes(row.private_bytes)}</td>
            <td class="num">{formatRate(row.read_bps + row.write_bps)}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
  {#if list.length > 0}
    <p class="more"><a href={href.programs()}>{t("ui.now.top.all", { count: programs?.entries.length ?? list.length })}</a></p>
  {/if}
</Block>
