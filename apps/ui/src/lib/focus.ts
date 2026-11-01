import { tick } from "svelte";

const STEP: Record<string, number> = { ArrowRight: 1, ArrowDown: 1, ArrowLeft: -1, ArrowUp: -1 };

/**
 * Action for a `role="radiogroup"`: arrows move and select, Home and End jump.
 * The radios themselves carry tabindex 0 when checked and -1 otherwise, so the group is one tab stop.
 */
export function radioGroup(node: HTMLElement): { destroy: () => void } {
  const onkeydown = (event: KeyboardEvent) => {
    const items = [...node.querySelectorAll<HTMLElement>('[role="radio"]')];
    const at = items.indexOf(document.activeElement as HTMLElement);
    if (at < 0) return;
    const step = STEP[event.key];
    const to = step
      ? (at + step + items.length) % items.length
      : event.key === "Home"
        ? 0
        : event.key === "End"
          ? items.length - 1
          : -1;
    if (to < 0) return;
    event.preventDefault();
    items[to].focus();
    items[to].click();
  };
  node.addEventListener("keydown", onkeydown);
  return { destroy: () => node.removeEventListener("keydown", onkeydown) };
}

/**
 * Runs a change that redraws part of `scope`, then puts focus back on the control with the same
 * `data-focus` value, or on `scope` when that control is gone.
 */
export async function keepFocus(scope: HTMLElement, change: () => unknown): Promise<void> {
  const active = document.activeElement as HTMLElement | null;
  const inside = !!active && active !== scope && scope.contains(active);
  const key = inside ? active.dataset.focus : undefined;
  await change();
  await tick();
  if (!inside || scope.contains(active)) return;
  const again = key ? scope.querySelector<HTMLElement>(`[data-focus="${CSS.escape(key)}"]`) : null;
  (again ?? scope).focus({ preventScroll: true });
}
