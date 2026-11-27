/**
 * Critter ids name the folder `critters/<id>/`, so, like env names, they are
 * limited to `[A-Za-z0-9_-]+` (`validate_critter_id` in Rust).
 */

const VALID_ID = /^[A-Za-z0-9_-]+$/;

/** Used when a name has nothing an id can keep, e.g. "🦎". */
const FALLBACK_ID = "critter";

export const isValidCritterId = (id: string): boolean => VALID_ID.test(id);

/** Whether a change to `path` (project-relative) can change the critters or their flows. Mirrors `layout::critter_from_path`. */
export const isCritterPath = (path: string): boolean => path.startsWith("critters/");

/** `critters/<critter>/flows/<flow>.yml`, as `layout::flow_file` writes it. */
export const flowFilePath = (critter: string, flow: string): string => `critters/${critter}/flows/${flow}.yml`;

/** The critter and flow a path is the flow file of, or null. */
export function flowOfPath(path: string): { critter: string; flow: string } | null {
  const match = /^critters\/([^/]+)\/flows\/([^/]+)\.yml$/.exec(path);
  return match?.[1] && match[2] ? { critter: match[1], flow: match[2] } : null;
}

/** `"critters/bob/flows/login.yml"` → `"bob"`; null for any other path. */
export function critterIdOf(path: string): string | null {
  if (!isCritterPath(path)) return null;
  const id = path.split("/")[1];
  return id ? id : null;
}

/** `"Bob (staging)"` → `"bob-staging"`. */
export function critterIdFrom(name: string): string {
  const slug = name
    .normalize("NFKD")
    .replace(/[̀-ͯ]/g, "")
    .toLowerCase()
    .replace(/[^a-z0-9_]+/g, "-")
    .replace(/^-+|-+$/g, "");
  return slug || FALLBACK_ID;
}

/** {@link critterIdFrom}, made free among `taken` by adding `-2`, `-3`, … */
export function uniqueCritterId(name: string, taken: Iterable<string>): string {
  const used = new Set([...taken].map((id) => id.toLowerCase()));
  const base = critterIdFrom(name);
  if (!used.has(base)) return base;
  for (let n = 2; ; n++) {
    const candidate = `${base}-${n}`;
    if (!used.has(candidate)) return candidate;
  }
}

/** Flow file names follow the same rules as critter ids. */
export const uniqueFlowId = uniqueCritterId;

/** Why a critter can't be called `name`, or `null` if it can. */
export function critterNameProblem(name: string, others: readonly { name: string }[]): string | null {
  const trimmed = name.trim();
  if (!trimmed) return "Give it a name.";
  if (others.some((other) => other.name.trim().toLowerCase() === trimmed.toLowerCase())) {
    return `There's already a bot called ${trimmed}.`;
  }
  return null;
}
