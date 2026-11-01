import { isTauri } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";

type Stop = () => unknown;

/**
 * Keeps `start` running only while the window is shown: stops it when the window is hidden or
 * minimized and starts it again on return. The returned function stops it for good.
 */
export function whileVisible(
  start: () => Promise<Stop>,
  onerror: (error: unknown) => void = () => {},
): () => void {
  let alive = true;
  let minimized = false;
  let stop: Stop | null = null;
  let queue = Promise.resolve();

  const sync = () => {
    queue = queue.then(async () => {
      const wanted = alive && !document.hidden && !minimized;
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

  let unlisten: (() => void) | null = null;
  if (isTauri()) {
    const window = getCurrentWindow();
    void window
      .onResized(async () => {
        minimized = await window.isMinimized();
        sync();
      })
      .then((off) => (alive ? (unlisten = off) : off()));
  }
  document.addEventListener("visibilitychange", sync);
  sync();

  return () => {
    alive = false;
    document.removeEventListener("visibilitychange", sync);
    unlisten?.();
    sync();
  };
}
