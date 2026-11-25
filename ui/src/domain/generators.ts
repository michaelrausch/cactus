import type { Suggester } from "./suggestions";

/** Values `- tool: set` generates when written alone, e.g. `id: $uuid`. Mirrors `tools::GENERATORS` in Rust. */
export const GENERATORS = ["$uuid", "$timestamp", "$isoTime", "$randomInt", "$randomEmail"] as const;

/** Offers the generators once a value starts with `$`. */
export const generatorSuggester: Suggester = (text, cursor) => {
  const typed = text.slice(0, cursor);
  if (!typed.startsWith("$") || text.trim() !== text) return [];
  return GENERATORS.filter((name) => name.toLowerCase().startsWith(typed.toLowerCase())).map((name) => ({
    label: name,
    from: 0,
    to: text.length,
    insert: name,
  }));
};
