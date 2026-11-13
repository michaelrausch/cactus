import type { ScenarioEntry } from "@/api/client";
import { dirOf, isWithinDir } from "./paths";

export interface FolderGroup {
  /** `""` for scenarios at the project root. */
  dir: string;
  entries: ScenarioEntry[];
}

const matchesFilter = (entry: ScenarioEntry, query: string): boolean =>
  `${entry.title} ${entry.relPath} ${entry.method}`.toLowerCase().includes(query);

/**
 * Entries arrive in tree order, so grouping preserves the order within and
 * between folders. Empty `folders` follow the last group inside their parent,
 * and are left out while filtering.
 */
export function groupByFolder(entries: ScenarioEntry[], filter: string, folders: string[] = []): FolderGroup[] {
  const query = filter.toLowerCase();
  const grouped = new Map<string, ScenarioEntry[]>();
  for (const entry of entries) {
    if (query && !matchesFilter(entry, query)) continue;
    const dir = dirOf(entry.relPath);
    grouped.set(dir, [...(grouped.get(dir) ?? []), entry]);
  }
  const groups: FolderGroup[] = [...grouped].map(([dir, list]) => ({ dir, entries: list }));
  if (query) return groups;
  for (const folder of folders) {
    if (groups.some((group) => group.dir === folder)) continue;
    const parent = dirOf(folder);
    const lastInParent = groups.findLastIndex((group) => group.dir === parent || isWithinDir(group.dir, parent));
    groups.splice(lastInParent + 1, 0, { dir: folder, entries: [] });
  }
  return groups;
}
