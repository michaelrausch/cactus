import type { ScenarioEntry } from "@/api/client";
import { dirOf } from "./paths";

export interface FolderGroup {
  /** `""` for scenarios at the project root. */
  dir: string;
  entries: ScenarioEntry[];
}

const matchesFilter = (entry: ScenarioEntry, query: string): boolean =>
  `${entry.title} ${entry.relPath} ${entry.method}`.toLowerCase().includes(query);

/** Entries arrive sorted by path, so grouping preserves the order within and between folders. */
export function groupByFolder(entries: ScenarioEntry[], filter: string): FolderGroup[] {
  const query = filter.toLowerCase();
  const groups = new Map<string, ScenarioEntry[]>();
  for (const entry of entries) {
    if (query && !matchesFilter(entry, query)) continue;
    const dir = dirOf(entry.relPath);
    groups.set(dir, [...(groups.get(dir) ?? []), entry]);
  }
  return [...groups].map(([dir, grouped]) => ({ dir, entries: grouped }));
}
