/** Shorter queries match too much to be useful. */
export const MIN_SEARCH_LENGTH = 2;
/** Caps highlighting work on huge bodies. */
export const MAX_SEARCH_MATCHES = 1000;

/** Case-insensitive, non-overlapping match offsets. */
export function findMatches(text: string, query: string): number[] {
  if (query.length < MIN_SEARCH_LENGTH) return [];
  const haystack = text.toLowerCase();
  const needle = query.toLowerCase();
  const offsets: number[] = [];
  for (let at = haystack.indexOf(needle); at >= 0 && offsets.length < MAX_SEARCH_MATCHES;) {
    offsets.push(at);
    at = haystack.indexOf(needle, at + needle.length);
  }
  return offsets;
}

/** Move through matches, wrapping at both ends. */
export const stepMatch = (current: number, delta: number, count: number): number =>
  count === 0 ? 0 : (current + delta + count) % count;

export function matchCountLabel(current: number, count: number): string {
  if (count === 0) return "0";
  return `${current + 1}/${count}${count >= MAX_SEARCH_MATCHES ? "+" : ""}`;
}
