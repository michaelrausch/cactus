/** Scenario paths are project-relative and always use `/`. */

const JSON_EXTENSION = ".json";

/** `"a/b/c.json"` → `"a/b"`; `""` at the project root. */
export const dirOf = (path: string): string => path.split("/").slice(0, -1).join("/");

/** Last segment of a path with either separator, e.g. a project folder's name. */
export const baseName = (path: string): string => path.split(/[\\/]/).findLast(Boolean) ?? path;

/** Mirrors `ScenarioPath::is_within` in Rust: every path is within `""`. */
export function isWithinDir(path: string, dir: string): boolean {
  const folder = dir.replace(/\/+$/, "");
  return folder === "" || path.startsWith(`${folder}/`);
}

export const withJsonExtension = (path: string): string =>
  path.endsWith(JSON_EXTENSION) ? path : `${path}${JSON_EXTENSION}`;
