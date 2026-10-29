import type { Auth, BodyType, JsonValue, Scenario } from "@/api/client";

export type JsonObject = { [key: string]: JsonValue };

export const isJsonObject = (value: JsonValue | undefined): value is JsonObject =>
  typeof value === "object" && value !== null && !Array.isArray(value);

/** Mirrors `Scenario::effective_body_type` in Rust. */
export function inferBodyType(scenario: Scenario): BodyType {
  if (scenario.bodyType) return scenario.bodyType;
  if (scenario.body === null) return "none";
  return typeof scenario.body === "string" ? "text" : "json";
}

/** How a body appears in the text editor. */
export function bodyToEditorText(body: JsonValue): string {
  if (body === null) return "";
  return typeof body === "string" ? body : JSON.stringify(body, null, 2);
}

/** Parse as JSON, falling back to the raw text. */
export function parseLooseJson(text: string): JsonValue {
  try {
    return JSON.parse(text) as JsonValue;
  } catch {
    return text;
  }
}

/** Text that isn't JSON becomes `{}`, and empty values become no body. */
function jsonBodyFromText(text: string): JsonValue {
  const parsed = parseLooseJson(text);
  if (typeof parsed === "string" && parsed) return {};
  return parsed || null;
}

/** Form fields keep only the string values of an existing object body. */
function stringFields(body: JsonValue): JsonObject {
  if (!isJsonObject(body)) return {};
  return Object.fromEntries(Object.entries(body).filter(([, value]) => typeof value === "string"));
}

/** `editorText` is what the body editor currently shows, which may not parse. */
export function switchBodyType(scenario: Scenario, next: BodyType, editorText: string): Scenario {
  const { bodyType: _previous, ...rest } = scenario;
  switch (next) {
    case "none":
      return { ...rest, body: null };
    case "text":
      return { ...rest, bodyType: "text", body: editorText };
    case "json":
      return { ...rest, bodyType: "json", body: jsonBodyFromText(editorText) };
    case "form":
    case "multipart":
      return { ...rest, bodyType: next, body: stringFields(scenario.body) };
  }
}

export function withAuth(scenario: Scenario, auth: Auth | null): Scenario {
  const { auth: _previous, ...rest } = scenario;
  return auth ? { ...rest, auth } : rest;
}

export const displayTitle = (scenario: Pick<Scenario, "title">): string => scenario.title || "Untitled";

export type JsonParseResult = { ok: true; value: JsonValue } | { ok: false; error: string };

/** Strict parse for the JSON body editor; blank text means no body. */
export function parseJsonBodyText(text: string): JsonParseResult {
  if (!text.trim()) return { ok: true, value: null };
  try {
    return { ok: true, value: JSON.parse(text) as JsonValue };
  } catch (error) {
    return { ok: false, error: error instanceof Error ? error.message : String(error) };
  }
}
