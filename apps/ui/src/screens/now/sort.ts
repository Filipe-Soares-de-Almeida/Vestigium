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

/** Programs whose name holds `query`, ranked by `key`; `rows` is cut to `limit`, `matched` is not. */
export function visiblePrograms(
  list: ProcessSample[],
  key: ProgramSort,
  query: string,
  limit: number,
): { rows: ProcessSample[]; matched: number } {
  const needle = query.trim().toLowerCase();
  const found = needle ? list.filter((p) => p.name.toLowerCase().includes(needle)) : list;
  const ranked = sortPrograms(found, key);
  return { rows: ranked.slice(0, limit), matched: ranked.length };
}
