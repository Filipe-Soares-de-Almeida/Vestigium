import { untrack } from "svelte";

import { liveStart } from "../../lib/api";
import { app } from "../../lib/app.svelte";
import { keepFocus } from "../../lib/focus";
import { CtrlHold } from "../../lib/hold.svelte";
import type { LivePoint, NowFrame, UiError } from "../../lib/types";
import { whileVisible } from "../../lib/visibility";
import { type Hung, Live } from "./live.svelte";

const STOPPED: UiError = { kind: "agent_stopped", message: "" };

/** What the screen shows while Ctrl is held. */
interface Frozen {
  points: LivePoint[];
  point: LivePoint | null;
  frame: NowFrame | null;
  hung: Hung[];
  nowMs: number;
}

/**
 * The live stream of one screen: it runs only while the screen is shown, the window is visible and
 * watching runs, and holding Ctrl keeps the second it was pressed. Create it while a component starts.
 */
export class NowStream {
  readonly live = new Live();
  #hold = new CtrlHold();
  #frozen = $state.raw<Frozen | null>(null);
  failure = $state<unknown>(null);
  #attempt = $state(0);

  /** The live data, or the frozen second while Ctrl is held. */
  readonly view = $derived(this.#frozen ?? this.live);
  readonly paused = $derived(this.#frozen !== null);
  readonly watching = $derived(app.running && !app.paused);
  readonly error = $derived(this.failure ?? (this.live.ended ? STOPPED : null));

  constructor() {
    $effect(() => {
      if (!this.watching) return;
      void this.#attempt;
      untrack(() => {
        this.failure = null;
        this.live.reset();
      });
      return whileVisible(
        () => liveStart((message) => this.live.take(message)),
        (error) => (this.failure = error),
      );
    });

    $effect(() => this.#hold.listen());

    $effect(() => {
      const held = this.#hold.held;
      untrack(() => {
        const live = this.live;
        this.#frozen =
          held && live.started
            ? { points: live.points, point: live.point, frame: live.frame, hung: live.hung, nowMs: live.nowMs }
            : null;
      });
    });

    // The watcher stopped answering: ask the shell again so its banner shows at once.
    $effect(() => {
      if (this.live.ended) void untrack(() => app.refresh());
    });
  }

  /** Starts the stream again; focus stays inside the screen while the retry button goes away. */
  retry(): void {
    const main = document.getElementById("main");
    if (main) void keepFocus(main, () => this.#attempt++);
    else this.#attempt++;
  }
}
