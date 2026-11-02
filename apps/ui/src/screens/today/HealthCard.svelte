<script lang="ts">
  import ConcernMark from "../../components/ConcernMark.svelte";
  import { health } from "../../lib/health.svelte";
  import { t, tn } from "../../lib/i18n.svelte";
  import { href } from "../../lib/router.svelte";
  import { AREAS, areaState } from "../health/areas";

  const touched = $derived(
    health.dto ? AREAS.map((a) => areaState(a, health.dto!)).filter((s) => s.open.length) : [],
  );
</script>

{#if touched.length}
  <section class="card" aria-labelledby="today-health">
    <h3 id="today-health">{t("ui.health.title")}</h3>
    <p class="muted">
      {health.critical
        ? tn("ui.health.card.critical", health.critical)
        : tn("ui.health.card.attention", health.attention)}
    </p>
    <ul>
      {#each touched as s (s.area.id)}
        <li>
          <a href={href.health(s.area.id)}>
            <ConcernMark mark={s.mark} word={false} />{t(`ui.health.area.${s.area.id}`)}
          </a>
        </li>
      {/each}
    </ul>
  </section>
{/if}

<style>
  h3 {
    margin-bottom: 4px;
  }
  p {
    margin: 0;
    font-size: 14px;
  }
  ul {
    display: grid;
    gap: 2px;
    margin: 8px 0 0;
    padding: 0;
    list-style: none;
  }
  a {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 4px 0;
    border-radius: 6px;
    color: var(--ink);
    font-size: 14px;
    text-decoration: none;
  }
  a:hover {
    text-decoration: underline;
  }
</style>
