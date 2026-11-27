import type { VarSource } from "@/api/client";
import { shortDuration } from "./jwt";

/**
 * Where a variable's value comes from, in a few words: `"staging"`,
 * `"Bob's context · auth/login.yml"`. `critter` names the critter the
 * critter layers belong to.
 */
export function varSourceLabel(source: VarSource, critter?: string): string {
  const owner = critter ? `${critter}'s` : "bot";
  switch (source.kind) {
    case "collection":
      return "shared";
    case "environment":
      return source.name;
    case "folder":
      return `folder ${source.dir}`;
    case "request":
      return "this request";
    case "secret":
      return "project secrets";
    case "captured":
      return "captured";
    case "critter":
      return `${owner} variables`;
    case "critterSecret":
      return `${owner} secrets`;
    case "critterContext":
      return `${owner} context · ${source.from}`;
    case "step":
      return "flow step";
  }
}

/**
 * {@link varSourceLabel} plus, for a captured value, how long ago it was
 * captured: `"Bob's context · auth/login.yml, 2m ago"`.
 */
export function varSourceDetail(source: VarSource, critter: string | undefined, now: number): string {
  const label = varSourceLabel(source, critter);
  return source.kind === "critterContext" ? `${label}, ${shortDuration(now - source.at)} ago` : label;
}

/** Each variable's source detail, by name. */
export const sourceDetails = (
  sources: Record<string, VarSource>,
  critter: string | undefined,
  now: number,
): Record<string, string> =>
  Object.fromEntries(Object.entries(sources).map(([name, source]) => [name, varSourceDetail(source, critter, now)]));

/**
 * The names `own` sets that a lower layer also sets, with that layer's
 * label: what a critter's variables override.
 */
export function overriddenNames(own: Iterable<string>, lower: Record<string, VarSource>): [string, string][] {
  return [...own]
    .filter((name) => name in lower)
    .toSorted()
    .map((name) => [name, varSourceLabel(lower[name] as VarSource)]);
}
