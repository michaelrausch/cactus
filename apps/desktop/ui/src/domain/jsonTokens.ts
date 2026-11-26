/** A forgiving JSON tokenizer for highlighting text that may not parse yet. */

export type JsonTokenKind = "key" | "string" | "number" | "literal" | "punctuation" | "placeholder" | "text";

export interface JsonToken {
  kind: JsonTokenKind;
  text: string;
}

const NUMBER = /-?\d+(?:\.\d+)?(?:[eE][+-]?\d+)?/y;
const WORD = /[A-Za-z_$][\w$]*/y;
const LITERALS = new Set(["true", "false", "null"]);
const PUNCTUATION = new Set(["{", "}", "[", "]", ":", ","]);

/** Index just past the string starting at `start`, which is a `"`. Unterminated strings end at the line. */
function stringEnd(text: string, start: number): number {
  for (let i = start + 1; i < text.length; i++) {
    const char = text[i];
    if (char === "\\") i++;
    else if (char === '"') return i + 1;
    else if (char === "\n") return i;
  }
  return text.length;
}

/** Whether the next non-blank character from `from` is a `:`, making a string a key. */
function followedByColon(text: string, from: number): boolean {
  for (let i = from; i < text.length; i++) {
    const char = text[i];
    if (char === ":") return true;
    if (char !== " " && char !== "\t" && char !== "\n" && char !== "\r") return false;
  }
  return false;
}

function matchAt(pattern: RegExp, text: string, at: number): string | null {
  pattern.lastIndex = at;
  return pattern.exec(text)?.[0] ?? null;
}

/** Tokens covering all of `text`, so joining their text gives it back. */
export function jsonTokens(text: string): JsonToken[] {
  const tokens: JsonToken[] = [];
  const push = (kind: JsonTokenKind, value: string) => {
    const last = tokens.at(-1);
    if (kind === "text" && last?.kind === "text") last.text += value;
    else tokens.push({ kind, text: value });
  };
  let i = 0;
  while (i < text.length) {
    const char = text[i] ?? "";
    if (char === '"') {
      const end = stringEnd(text, i);
      push(followedByColon(text, end) ? "key" : "string", text.slice(i, end));
      i = end;
      continue;
    }
    if (text.startsWith("{{", i)) {
      const close = text.indexOf("}}", i + 2);
      if (close >= 0) {
        push("placeholder", text.slice(i, close + 2));
        i = close + 2;
        continue;
      }
    }
    if (PUNCTUATION.has(char)) {
      push("punctuation", char);
      i++;
      continue;
    }
    const number = matchAt(NUMBER, text, i);
    if (number) {
      push("number", number);
      i += number.length;
      continue;
    }
    const word = matchAt(WORD, text, i);
    if (word) {
      push(LITERALS.has(word) ? "literal" : "text", word);
      i += word.length;
      continue;
    }
    push("text", char);
    i++;
  }
  return tokens;
}
