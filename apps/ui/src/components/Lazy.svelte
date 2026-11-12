<script lang="ts" generics="P extends Record<string, any>">
  import { onMount, type Component } from "svelte";

  import ErrorView from "./ErrorView.svelte";
  import Loading from "./Loading.svelte";

  let { load, props }: { load: () => Promise<{ default: Component<P> }>; props: P } = $props();

  let screen = $state.raw<Component<P> | null>(null);
  let failure = $state<unknown>(null);

  function fetchScreen(): void {
    failure = null;
    load().then(
      (loaded) => (screen = loaded.default),
      (error: unknown) => (failure = error),
    );
  }

  onMount(fetchScreen);
</script>

{#if failure !== null}
  <ErrorView error={failure} heading="h1" onretry={fetchScreen} />
{:else if screen}
  {@const Screen = screen}
  <Screen {...props} />
{:else}
  <Loading />
{/if}
