import type { HttpResponse, JsonValue } from "@/api/client";

/** `partialJson` is a truncated JSON body: unparseable, but still worth indenting. */
export type ParsedBody = { kind: "json"; value: JsonValue } | { kind: "partialJson" } | { kind: "text" };

const LOOKS_LIKE_JSON = /^\s*[[{]/;

export const headerValue = (response: HttpResponse, name: string): string | undefined =>
  response.headers.find(([key]) => key.toLowerCase() === name.toLowerCase())?.[1];

/** JSON if the server says so or the body looks like it; truncated bodies never parse. */
export function parseBody(response: HttpResponse): ParsedBody {
  const contentType = headerValue(response, "content-type") ?? "";
  const claimsJson = contentType.includes("json") || LOOKS_LIKE_JSON.test(response.body);
  if (!claimsJson) return { kind: "text" };
  if (response.truncated) return { kind: "partialJson" };
  try {
    return { kind: "json", value: JSON.parse(response.body) as JsonValue };
  } catch {
    return { kind: "text" };
  }
}

/** The body as shown in raw view: pretty-printed when it is JSON. */
export function displayBody(response: HttpResponse, parsed: ParsedBody): string {
  if (parsed.kind === "json") return JSON.stringify(parsed.value, null, 2);
  if (parsed.kind === "partialJson") return indentJsonText(response.body);
  return response.body;
}

const INDENT = "  ";
const isSpace = (ch: string | undefined) => ch === " " || ch === "\n" || ch === "\r" || ch === "\t";

/**
 * Indent JSON text without parsing it, so a truncated body still reads well.
 * For valid JSON the output matches `JSON.stringify(value, null, 2)`.
 */
export function indentJsonText(text: string): string {
  const out: string[] = [];
  let depth = 0;
  let inString = false;
  let escaped = false;
  const newline = () => out.push("\n", INDENT.repeat(depth));

  for (let i = 0; i < text.length; i++) {
    const ch = text.charAt(i);
    if (inString) {
      out.push(ch);
      if (escaped) escaped = false;
      else if (ch === "\\") escaped = true;
      else if (ch === '"') inString = false;
      continue;
    }
    if (isSpace(ch)) continue;
    if (ch === "{" || ch === "[") {
      const close = ch === "{" ? "}" : "]";
      let next = i + 1;
      while (isSpace(text[next])) next++;
      if (text[next] === close) {
        // Empty container stays on one line, like JSON.stringify.
        out.push(ch, close);
        i = next;
      } else {
        out.push(ch);
        depth++;
        newline();
      }
    } else if (ch === "}" || ch === "]") {
      depth = Math.max(0, depth - 1);
      newline();
      out.push(ch);
    } else if (ch === ",") {
      out.push(ch);
      newline();
    } else if (ch === ":") {
      out.push(": ");
    } else {
      if (ch === '"') inString = true;
      out.push(ch);
    }
  }
  return out.join("");
}
