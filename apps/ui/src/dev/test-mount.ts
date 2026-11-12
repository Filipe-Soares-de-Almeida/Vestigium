import { type Component, flushSync, mount, tick, unmount } from "svelte";

const mounted: Record<string, unknown>[] = [];

/** Mounts `component` into the test page; `unmountAll` removes it after the test. */
export function show<P extends Record<string, unknown>>(component: Component<P>, props: P = {} as P): HTMLElement {
  const target = document.createElement("div");
  document.body.appendChild(target);
  mounted.push(mount(component, { target, props }));
  flushSync();
  return target;
}

export function unmountAll(): void {
  while (mounted.length) void unmount(mounted.pop()!);
  document.body.innerHTML = "";
}

/** Waits until `check` holds, letting pending promises and renders run. */
export async function until(check: () => boolean, what: string): Promise<void> {
  for (let turn = 0; turn < 200; turn++) {
    if (check()) return;
    await new Promise((resolve) => setTimeout(resolve, 5));
    await tick();
  }
  throw new Error(`timed out waiting for ${what}`);
}
