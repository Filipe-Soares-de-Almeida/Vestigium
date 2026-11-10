import { isTauri } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";

type Stop = () => unknown;

let minimized = false;
const listeners = new Set<() => void>();
let detach: (() => void) | null = null;

const notify = () => {
  for (const listener of [...listeners]) listener();
};

/** Records whether the window is minimized and tells the listeners when that changes. */
export function setMinimized(value: boolean): void {
  if (value === minimized) return;
  minimized = value;
  notify();
}

/** True while the window can be seen: the page is not hidden and the window is not minimized. */
export function isShown(): boolean {
  return !document.hidden && !minimized;
}

/** Follows the page's visibility and the window's minimized state; returns the function that stops it. */
function attach(): () => void {
  document.addEventListener("visibilitychange", notify);
  let alive = true;
  let unlisten: (() => void) | null = null;
  if (isTauri()) {
    const window = getCurrentWindow();
    void window.isMinimized().then(setMinimized, () => {});
    void window
      .onResized(async () => setMinimized(await window.isMinimized()))
      .then((off) => (alive ? (unlisten = off) : off()));
  }
  return () => {
    alive = false;
    document.removeEventListener("visibilitychange", notify);
    unlisten?.();
  };
}

/** Calls `listener` whenever the window may have been shown or hidden; returns the function that stops it. */
export function onShownChange(listener: () => void): () => void {
  if (listeners.size === 0) detach = attach();
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
    if (listeners.size === 0) {
      detach?.();
      detach = null;
      minimized = false;
    }
  };
}

/**
 * Runs `run` every `ms` while the window is shown, and once on return when a full interval has passed
 * since the last run; returns the function that stops it.
 */
export function pollWhileShown(run: () => unknown, ms: number): () => void {
  let last = Date.now();
  const tick = () => {
    last = Date.now();
    void run();
  };
  const timer = setInterval(() => {
    if (isShown()) tick();
  }, ms);
  const off = onShownChange(() => {
    if (isShown() && Date.now() - last >= ms) tick();
  });
  return () => {
    clearInterval(timer);
    off();
  };
}

/**
 * Keeps `start` running only while the window is shown: stops it when the window is hidden or
 * minimized and starts it again on return. The returned function stops it for good.
 */
export function whileVisible(
  start: () => Promise<Stop>,
  onerror: (error: unknown) => void = () => {},
): () => void {
  let alive = true;
  let stop: Stop | null = null;
  let queue = Promise.resolve();

  const sync = () => {
    queue = queue.then(async () => {
      const wanted = alive && isShown();
      try {
        if (wanted && !stop) stop = await start();
        else if (!wanted && stop) {
          const running = stop;
          stop = null;
          await running();
        }
      } catch (error) {
        if (alive) onerror(error);
      }
    });
  };

  const off = onShownChange(sync);
  sync();

  return () => {
    alive = false;
    off();
    sync();
  };
}
