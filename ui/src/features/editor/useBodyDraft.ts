import { useState } from "react";
import type { BodyType } from "@/api/client";
import { bodyToEditorText, inferBodyType, parseJsonBodyText, switchBodyType } from "@/domain/scenario";
import type { ScenarioTabProps } from "./types";

export interface BodyDraft {
  bodyType: BodyType;
  /** What the text editor shows; may be invalid JSON. */
  text: string;
  jsonError: string | null;
  editText: (text: string) => void;
  switchType: (next: BodyType) => void;
}

/**
 * The body as text. JSON is written back only once it parses, so the file
 * always stays valid. Lives in the editor (not the tab) so a half-typed body
 * survives switching tabs.
 */
export function useBodyDraft({ scenario, onChange }: ScenarioTabProps): BodyDraft {
  const [text, setText] = useState(() => bodyToEditorText(scenario.body));
  const [jsonError, setJsonError] = useState<string | null>(null);
  const bodyType = inferBodyType(scenario);

  const editText = (next: string) => {
    setText(next);
    if (bodyType === "text") {
      onChange({ ...scenario, body: next });
      return;
    }
    const parsed = parseJsonBodyText(next);
    if (!parsed.ok) {
      setJsonError(parsed.error);
      return;
    }
    setJsonError(null);
    onChange({ ...scenario, body: parsed.value });
  };

  const switchType = (next: BodyType) => {
    setJsonError(null);
    onChange(switchBodyType(scenario, next, text));
  };

  return { bodyType, text, jsonError, editText, switchType };
}
