import { listen } from "@tauri-apps/api/event";

import {
  capture as readCapture,
  captureRun,
  captures as readList,
  importCaptures,
  startCapture,
} from "./api";
import { errorKind } from "./errors";
import { t } from "./i18n.svelte";
import { toast } from "./toast.svelte";
import type { CaptureDto, CaptureFinished, CaptureRun, CaptureSummary } from "./types";

/** The newest captures the history list reads. */
const KEPT = 30;
const DAY_MS = 86_400_000;

export interface Entry {
  summary: CaptureSummary;
  dto: CaptureDto;
}

/** Why the last attempt did not produce a capture; the screen says it calmly and offers another try. */
export type Problem = "declined" | "failed" | "interrupted";

/** The deep capture this window started or found finished: its run, its history and what to say next. */
class Captures {
  /** The capture under way; `null` when none is. */
  run = $state<CaptureRun | null>(null);
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

  #cache = new Map<number, CaptureDto>();
  #timer: ReturnType<typeof setInterval> | undefined;

  /** Listens for the helper's end and keeps the clock; returns the function that stops both. */
  watch(): () => void {
    const unlisten = listen<CaptureFinished>("capture-finished", (event) => void this.#ended(event.payload));
    const onvisible = () => {
      if (!document.hidden) this.now = Date.now();
    };
    document.addEventListener("visibilitychange", onvisible);
    return () => {
      void unlisten.then((stop) => stop());
      document.removeEventListener("visibilitychange", onvisible);
      this.#stopClock();
    };
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
    try {
      this.#setRun(await startCapture(minutes));
    } catch (error) {
      this.problem = errorKind(error) === "declined" ? "declined" : "failed";
      this.problemError = error;
    } finally {
      this.asking = false;
    }
  }

  /** Clears a problem or a finished notice so the start card shows again. */
  dismiss(): void {
    this.problem = null;
    this.finished = null;
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

  async #ended(done: CaptureFinished): Promise<void> {
    this.#setRun(null);
    const ok = done.exit_code === 0 && done.ids.length > 0;
    if (!ok) this.problem = "interrupted";
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
      this.#stopClock();
    }
  }

  #stopClock(): void {
    clearInterval(this.#timer);
    this.#timer = undefined;
  }
}

export const captures = new Captures();
