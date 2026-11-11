import { listen } from "@tauri-apps/api/event";

import {
  cancelCapture,
  capture as readCapture,
  captureRun,
  captures as readList,
  importCaptures,
  startCapture,
} from "./api";
import { errorKind, errorText } from "./errors";
import { helperJobs } from "./helper.svelte";
import { t } from "./i18n.svelte";
import { toast } from "./toast.svelte";
import type { CaptureDto, CaptureFinished, CaptureRun, CaptureSummary } from "./types";

/** The newest captures the history list reads. */
const KEPT = 30;
const DAY_MS = 86_400_000;
/** How often the window looks again for a capture another window started, to see it end. */
const POLL_MS = 5000;

export interface Entry {
  summary: CaptureSummary;
  dto: CaptureDto;
}

/** Why the last attempt did not produce a capture; the screen says it calmly and offers another try. */
export type Problem = "declined" | "failed" | "interrupted";

/** The deep capture under way, whoever started it, and the history of finished ones. */
class Captures {
  /** The capture under way; `null` when none is. */
  run = $state<CaptureRun | null>(null);
  /** A second capture was refused while one runs; the strip says so until it is dismissed. */
  refused = $state(false);
  /** The newest captures with what they name; `null` until the first read. */
  entries = $state.raw<Entry[] | null>(null);
  /** Windows is showing its permission prompt. */
  asking = $state(false);
  problem = $state<Problem | null>(null);
  problemError = $state<unknown>(null);
  /** A capture that finished and was not opened yet. */
  finished = $state<number | null>(null);
  /** The clock the waiting card reads; it moves once a second only while a capture runs. */
  now = $state(Date.now());
  /** The helper was told to stop and the window waits for it to end. */
  cancelling = $state(false);
  /** The length of the last capture asked for, for a new try after a timeout. */
  lastMinutes = $state(5);

  #cache = new Map<number, CaptureDto>();
  #timer: ReturnType<typeof setInterval> | undefined;
  #poller: ReturnType<typeof setInterval> | undefined;
  #polling = false;

  /** Listens for the helper's end and keeps the clock; returns the function that stops both. */
  watch(): () => void {
    const unlisten = listen<CaptureFinished>("capture-finished", (event) => void this.#ended(event.payload));
    const onvisible = () => {
      if (document.hidden) return;
      this.now = Date.now();
      void this.#poll();
    };
    document.addEventListener("visibilitychange", onvisible);
    return () => {
      void unlisten.then((stop) => stop());
      document.removeEventListener("visibilitychange", onvisible);
      this.#stopClock();
    };
  }

  /** Looks again for the capture under way: one another window started has no event to say it ended. */
  async #poll(): Promise<void> {
    if (!this.run || this.run.own || this.#polling) return;
    this.#polling = true;
    try {
      const run = await captureRun();
      if (run) {
        this.#setRun(run);
        return;
      }
      this.#setRun(null);
      const ids = await importCaptures();
      await this.#load();
      if (ids.length) {
        this.finished = ids[ids.length - 1];
        toast.show(t("ui.captures.toast.finished"));
      }
    } catch {
      // The next look tries again.
    } finally {
      this.#polling = false;
    }
  }

  /** Stores what finished while the window was closed, then reads the list and any capture still running. */
  async sync(): Promise<void> {
    let ids: number[] = [];
    try {
      ids = await importCaptures();
    } catch {
      // The watcher is not running: the list below still shows what is stored.
    }
    await this.#load();
    try {
      this.#setRun(await captureRun());
    } catch {
      // No run to restore.
    }
    if (ids.length) {
      this.finished = ids[ids.length - 1];
      toast.show(t("ui.captures.toast.finished"));
    }
  }

  /** Asks Windows for permission and starts a capture of `minutes`. */
  async begin(minutes: number): Promise<void> {
    if (this.asking || this.run) return;
    this.asking = true;
    this.problem = null;
    this.finished = null;
    this.refused = false;
    this.lastMinutes = minutes;
    helperJobs.clear("capture");
    try {
      this.#setRun(await startCapture(minutes));
    } catch (error) {
      if (errorKind(error) === "capture_running") await this.#refused();
      else {
        this.problem = errorKind(error) === "declined" ? "declined" : "failed";
        this.problemError = error;
      }
    } finally {
      this.asking = false;
    }
  }

  /** Tells the helper to stop and keep nothing; the end arrives as the finished event. */
  async cancel(): Promise<void> {
    if (!this.run || this.cancelling) return;
    this.cancelling = true;
    try {
      await cancelCapture();
    } catch (error) {
      this.cancelling = false;
      toast.show(errorText(error));
    }
  }

  /** Clears a problem, a refusal or a finished notice. */
  dismiss(): void {
    this.problem = null;
    this.finished = null;
    this.refused = false;
  }

  /** Takes the "finished" notice away once its result is open. */
  seen(id: number): void {
    if (this.finished === id) this.finished = null;
  }

  /** The stored capture, from memory when this window already read it. */
  async get(id: number): Promise<CaptureDto | null> {
    const known = this.#cache.get(id);
    if (known) return known;
    const dto = await readCapture(id);
    if (dto) this.#cache.set(id, dto);
    return dto;
  }

  /** Drops what this window read and reads the list again, after the stored captures were deleted. */
  async forget(): Promise<void> {
    this.#cache.clear();
    await this.#load();
  }

  /** A second capture was refused: the one that runs is still there, so ask which it is. */
  async #refused(): Promise<void> {
    this.refused = true;
    try {
      this.#setRun(await captureRun());
    } catch {
      // The strip still says a capture runs; the next look finds it.
    }
  }

  async #ended(done: CaptureFinished): Promise<void> {
    this.#setRun(null);
    this.cancelling = false;
    if (done.ended === "Refused") {
      await this.#refused();
      return;
    }
    // A capture the helper saved counts as finished even when a cancel or a limit came as it ended.
    const ok = done.ids.length > 0 && (done.exit_code === 0 || done.ended !== "Done");
    if (!ok) {
      if (done.ended === "Cancelled") helperJobs.record("capture", "cancelled");
      else if (done.ended === "TimedOut") helperJobs.record("capture", "timeout");
      else this.problem = "interrupted";
    }
    await this.#load();
    if (ok) {
      this.finished = done.ids[done.ids.length - 1];
      toast.show(t("ui.captures.toast.finished"));
    }
  }

  async #load(): Promise<void> {
    try {
      const now = Date.now();
      const found = (await readList(0, now + DAY_MS)).sort((a, b) => b.start_ms - a.start_ms).slice(0, KEPT);
      const read = await Promise.allSettled(found.map((s) => this.get(s.id)));
      this.entries = found.flatMap((summary, i) => {
        const result = read[i];
        return result.status === "fulfilled" && result.value ? [{ summary, dto: result.value }] : [];
      });
    } catch {
      this.entries ??= [];
    }
  }

  #setRun(run: CaptureRun | null): void {
    this.run = run;
    if (run) {
      this.now = Date.now();
      this.#timer ??= setInterval(() => {
        if (!document.hidden) this.now = Date.now();
      }, 1000);
    } else {
      this.refused = false;
      clearInterval(this.#timer);
      this.#timer = undefined;
    }
    if (run && !run.own) {
      this.#poller ??= setInterval(() => void this.#poll(), POLL_MS);
    } else {
      clearInterval(this.#poller);
      this.#poller = undefined;
    }
  }

  #stopClock(): void {
    clearInterval(this.#timer);
    this.#timer = undefined;
    clearInterval(this.#poller);
    this.#poller = undefined;
  }
}

export const captures = new Captures();
