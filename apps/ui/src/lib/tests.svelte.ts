import { listen } from "@tauri-apps/api/event";

import {
  diskDetail,
  openMemoryDiagnostic,
  startBenchmark,
  startMemoryTest,
  startSurfaceScan,
  startVolumeCheck,
  stopTest,
  tests as readTests,
} from "./api";
import { errorKind } from "./errors";
import { helperJobs } from "./helper.svelte";
import type { DiskDetailDto, DiskDto, RunningTest, TestFinished, TestProgress, TestsDto } from "./types";

/** How a request to start a test ended. */
export type Started = { kind: "started" } | { kind: "declined" } | { kind: "failed"; error: unknown };

/** The Tests page's data: what can be tested, earlier runs, and the one test running now. */
class Tests {
  /** `null` until the first read. */
  dto = $state.raw<TestsDto | null>(null);
  error = $state<unknown>(null);
  running = $state.raw<RunningTest | null>(null);
  progress = $state.raw<TestProgress | null>(null);
  /** The last test that ended while the page listened. */
  finished = $state.raw<TestFinished | null>(null);
  /** SMART or NVMe detail read on request, by disk number. */
  details = $state.raw<Record<number, DiskDetailDto>>({});
  /** Windows is asking for permission, or a test is being set up. */
  busy = $state(false);
  /** A stop was asked for and the test has not ended yet. */
  stopping = $state(false);

  /** Listens for progress and the end of a test for the whole window; returns the function that stops it. */
  listen(): () => void {
    const progress = listen<TestProgress>("test-progress", (event) => {
      this.progress = event.payload;
    });
    const finished = listen<TestFinished>("test-finished", (event) => {
      const was = this.running;
      if (event.payload.ended === "Cancelled" && was?.kind === "VolumeCheck") {
        const root = typeof was.subject === "object" && "Volume" in was.subject ? was.subject.Volume : undefined;
        helperJobs.record("check", "cancelled", root);
      }
      this.stopping = false;
      this.finished = event.payload;
      this.running = null;
      this.progress = null;
      void this.load();
    });
    return () => {
      void progress.then((stop) => stop());
      void finished.then((stop) => stop());
    };
  }

  /** A failed read keeps what is on screen; the error shows only when there is nothing yet. */
  async load(): Promise<void> {
    try {
      this.dto = await readTests();
      this.running = this.dto.running;
      this.error = null;
    } catch (error) {
      if (this.dto === null) this.error = error;
    }
  }

  memory(minutes: number): Promise<Started> {
    return this.#start(() => startMemoryTest(minutes));
  }

  benchmark(volume: string): Promise<Started> {
    return this.#start(() => startBenchmark(volume));
  }

  surface(disk: number, fromMib: number): Promise<Started> {
    return this.#start(() => startSurfaceScan(disk, fromMib));
  }

  volume(root: string): Promise<Started> {
    helperJobs.clear("check");
    return this.#start(() => startVolumeCheck(root));
  }

  /** Opens Windows' own Memory Diagnostic; it schedules the test for the next restart. */
  async diagnostic(): Promise<Started> {
    this.busy = true;
    try {
      await openMemoryDiagnostic();
      return { kind: "started" };
    } catch (error) {
      return errorKind(error) === "declined" ? { kind: "declined" } : { kind: "failed", error };
    } finally {
      this.busy = false;
    }
  }

  /** Stops the running test, or the disk scan the helper runs for an earlier window. */
  async stop(): Promise<void> {
    this.stopping = true;
    try {
      await stopTest(this.running ? null : (this.dto?.scan?.stem ?? null));
    } catch (error) {
      this.stopping = false;
      throw error;
    }
  }

  async readDetail(disk: DiskDto): Promise<void> {
    const detail = await diskDetail(disk.number, disk.name);
    this.details = { ...this.details, [disk.number]: detail };
  }

  async #start(start: () => Promise<RunningTest>): Promise<Started> {
    if (this.busy || this.running) return { kind: "failed", error: null };
    this.busy = true;
    this.finished = null;
    try {
      this.running = await start();
      this.progress = null;
      return { kind: "started" };
    } catch (error) {
      return errorKind(error) === "declined" ? { kind: "declined" } : { kind: "failed", error };
    } finally {
      this.busy = false;
    }
  }
}

export const hwtests = new Tests();
