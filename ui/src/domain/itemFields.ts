/** How many items to look at for their fields: enough to see optional ones, few enough to stay quick. */
const SAMPLE = 50;

/**
 * The paths an item of a JSON list has, for checks on each item: its keys,
 * one level of nested keys (`user.email`) and `.length` of arrays and
 * strings. Empty when `json` isn't a list of objects.
 */
export function itemFields(json: string | undefined): string[] {
  let items: unknown;
  try {
    items = JSON.parse(json ?? "");
  } catch {
    return [];
  }
  if (!Array.isArray(items)) return [];
  const fields = new Set<string>();
  const add = (path: string, value: unknown) => {
    fields.add(path);
    if (Array.isArray(value) || typeof value === "string") fields.add(`${path}.length`);
  };
  for (const item of items.slice(0, SAMPLE)) {
    if (!item || typeof item !== "object" || Array.isArray(item)) continue;
    for (const [key, value] of Object.entries(item as Record<string, unknown>)) {
      add(key, value);
      if (value && typeof value === "object" && !Array.isArray(value)) {
        for (const [inner, innerValue] of Object.entries(value as Record<string, unknown>)) {
          add(`${key}.${inner}`, innerValue);
        }
      }
    }
  }
  return [...fields];
}

/** The variable a list field names when it's just `{{name}}`. */
export const listVariable = (list: string | undefined): string | null =>
  /^\{\{\s*([^{}]+?)\s*\}\}$/.exec(list?.trim() ?? "")?.[1] ?? null;
