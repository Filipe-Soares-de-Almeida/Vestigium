<script lang="ts">
  import type { DetailKind } from "../lib/router.svelte";
  import DiagnosticDetail from "./detail/DiagnosticDetail.svelte";
  import DriverDetail from "./detail/DriverDetail.svelte";
  import EventDetail from "./detail/EventDetail.svelte";
  import FindingDetail from "./detail/FindingDetail.svelte";
  import ProcessDetail from "./detail/ProcessDetail.svelte";
  import SmartDetail from "./detail/SmartDetail.svelte";
  import TestDetail from "./detail/TestDetail.svelte";

  let { kind, parts }: { kind: DetailKind; parts: string[] } = $props();

  const num = (i: number) => Number(parts[i] ?? NaN);
</script>

{#if kind === "test"}
  <TestDetail id={num(0)} />
{:else if kind === "diagnostic"}
  <DiagnosticDetail recordId={num(0)} />
{:else if kind === "finding"}
  <FindingDetail id={num(0)} />
{:else if kind === "event"}
  <EventDetail occurrenceId={num(0)} recordId={num(1)} />
{:else if kind === "driver"}
  <DriverDetail captureId={num(0)} module={parts.slice(1).join("/")} />
{:else if kind === "smart"}
  <SmartDetail disk={num(0)} attr={num(1)} />
{:else}
  <ProcessDetail name={parts.join("/")} />
{/if}
