/** The elevated jobs a person can cancel or see give up: each is a card on its own screen. */
export type JobKey = "capture" | "check" | "dumps" | "health";

/** How the last attempt of a job ended without a result. */
export interface Attempt {
  kind: "cancelled" | "timeout";
  /** A timeout waits to be told once, on the job's own screen. */
  seen: boolean;
  /** The volume a check was about. */
  volume?: string;
}

/** The last attempt of each helper job that ended without a result, and the cancel question open. */
class HelperJobs {
  /** The job whose cancel confirmation is open. */
  asking = $state<JobKey | null>(null);
  last = $state<Partial<Record<JobKey, Attempt>>>({});

  ask(job: JobKey): void {
    this.asking = job;
  }

  dismiss(): void {
    this.asking = null;
  }

  /** Notes how an attempt ended; a cancel needs no telling, a timeout does. */
  record(job: JobKey, kind: Attempt["kind"], volume?: string): void {
    this.last = { ...this.last, [job]: { kind, seen: kind === "cancelled", volume } };
  }

  /** Forgets the last attempt, when a new one starts. */
  clear(job: JobKey): void {
    const { [job]: _gone, ...rest } = this.last;
    this.last = rest;
  }

  /** Marks the timeout of a job as told. */
  see(job: JobKey): void {
    const attempt = this.last[job];
    if (attempt) this.last = { ...this.last, [job]: { ...attempt, seen: true } };
  }

  /** A timeout nobody was told about yet. */
  unseen(job: JobKey): boolean {
    const attempt = this.last[job];
    return attempt !== undefined && attempt.kind === "timeout" && !attempt.seen;
  }
}

export const helperJobs = new HelperJobs();
