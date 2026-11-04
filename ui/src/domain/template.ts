/**
 * `{{name}}` placeholders, found the way the backend's substitution finds
 * them: from `{{` to the first `}}` after it, the name trimmed.
 */

const OPEN = "{{";
const CLOSE = "}}";

export interface TemplatePart {
  text: string;
  /** The trimmed variable name when this part is a placeholder. */
  placeholder?: string;
}

/** Split `text` into plain runs and placeholders, in order. */
export function templateParts(text: string): TemplatePart[] {
  const parts: TemplatePart[] = [];
  let rest = 0;
  for (;;) {
    const start = text.indexOf(OPEN, rest);
    const close = start < 0 ? -1 : text.indexOf(CLOSE, start + OPEN.length);
    if (close < 0) break;
    const end = close + CLOSE.length;
    if (start > rest) parts.push({ text: text.slice(rest, start) });
    parts.push({ text: text.slice(start, end), placeholder: text.slice(start + OPEN.length, close).trim() });
    rest = end;
  }
  if (rest < text.length) parts.push({ text: text.slice(rest) });
  return parts;
}

/** Characters a placeholder name can be completed through. */
const NAME_CHARS = /^[\w.$-]*$/;

/**
 * The placeholder being typed at `cursor`: `start` is where its `{{` is and
 * `query` what has been typed of the name so far.
 */
export function openPlaceholder(text: string, cursor: number): { start: number; query: string } | null {
  const before = text.slice(0, cursor);
  const start = before.lastIndexOf(OPEN);
  if (start < 0) return null;
  const typed = before.slice(start + OPEN.length);
  if (typed.includes(CLOSE) || !NAME_CHARS.test(typed.trimStart())) return null;
  return { start, query: typed.trim() };
}

/** How far past `cursor` a completed placeholder replaces: the rest of the name and its `}}`. */
export function placeholderTail(text: string, cursor: number): number {
  const match = /^[\w.$-]*\s*\}\}/.exec(text.slice(cursor));
  return match ? match[0].length : 0;
}
