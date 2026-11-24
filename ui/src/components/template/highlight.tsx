import type { ReactNode } from "react";
import { jsonTokens, type JsonTokenKind } from "@/domain/jsonTokens";
import { templateParts } from "@/domain/template";

export type Syntax = "template" | "json";

/** Variable names in scope; null while unknown, when nothing is flagged as unresolved. */
export type KnownNames = ReadonlySet<string> | null;

const RESOLVED = "rounded-sm bg-teal-400/15 text-teal-200";
const UNRESOLVED =
  "rounded-sm bg-amber-400/15 text-amber-300 underline decoration-amber-400/70 decoration-wavy underline-offset-2";

/** The same colours as the response's JSON viewer. */
const JSON_CLASSES: Record<JsonTokenKind, string | undefined> = {
  key: "text-sky-300",
  string: "text-emerald-300",
  number: "text-amber-300",
  literal: "text-violet-300",
  punctuation: "text-muted-foreground",
  placeholder: undefined,
  text: undefined,
};

const isResolved = (name: string, known: KnownNames) => known === null || known.has(name);

function placeholders(text: string, known: KnownNames, plainClass: string | undefined, key: string): ReactNode[] {
  return templateParts(text).map((part, index) => {
    const id = `${key}.${index}`;
    if (part.placeholder === undefined) {
      return plainClass ? (
        <span key={id} className={plainClass}>
          {part.text}
        </span>
      ) : (
        part.text
      );
    }
    const resolved = isResolved(part.placeholder, known);
    return (
      <span
        key={id}
        className={resolved ? RESOLVED : UNRESOLVED}
        title={resolved ? undefined : `${part.placeholder} isn't defined in any env layer`}
      >
        {part.text}
      </span>
    );
  });
}

/**
 * `text` as coloured spans. The text is unchanged, so it lines up exactly
 * with the input drawn over it.
 */
export function highlight(text: string, syntax: Syntax, known: KnownNames): ReactNode[] {
  if (syntax === "template") return placeholders(text, known, undefined, "t");
  return jsonTokens(text).flatMap((token, index) =>
    placeholders(token.text, known, JSON_CLASSES[token.kind], `j${index}`),
  );
}
