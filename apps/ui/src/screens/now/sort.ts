import type { ProcessSample } from "../../lib/types";

export type ProgramSort = "cpu" | "memory" | "disk";

const VALUE: Record<ProgramSort, (p: ProcessSample) => number> = {
  cpu: (p) => p.cpu_pct,
  memory: (p) => p.private_bytes,
  disk: (p) => p.read_bps + p.write_bps,
};

/** Programs from the one that uses the most of `key` down, a copy of the list. */
export function sortPrograms(list: ProcessSample[], key: ProgramSort): ProcessSample[] {
  const value = VALUE[key];
  return list.slice().sort((a, b) => value(b) - value(a) || a.pid - b.pid);
}
