import type { Auth, AuthMode, Body, FormField, Header, MultipartField, Param, Scenario, TypedBody } from "@/api/client";
import { AUTH_KINDS, BODY_KINDS, TEXT_BODY_KINDS, type AuthKind, type BodyKind, type TextBodyKind } from "./http";
import { parseQuery, withQuery } from "./query";
import { makeRow, type Row } from "./rows";

const PATH_PARAM_PREFIX = ":";
/** `{{name}}` placeholders, which make otherwise valid JSON fail to parse. */
const PLACEHOLDER = /\{\{[^}]*\}\}/g;

/** The request's own active variables, which may not be saved yet. */
export const ownVariableNames = (variables: NonNullable<Scenario["runtime"]>["variables"]): string[] =>
  (variables ?? []).filter((v) => !v.disabled && v.name.trim()).map((v) => v.name.trim());

export const displayTitle = (scenario: Scenario): string => scenario.info.name || "Untitled";

/** The tag that marks a request as one that logs in (`AUTH_TAG` in Rust). */
export const AUTH_TAG = "auth";

export const isAuthRequest = (tags: readonly string[] | undefined): boolean => (tags ?? []).includes(AUTH_TAG);

/**
 * Tag the request as one that logs in, or untag it. No tags means no `tags`
 * key, as the backend writes it, so a round trip doesn't look like an edit.
 */
export function withAuthTag(scenario: Scenario, auth: boolean): Scenario {
  const others = (scenario.info.tags ?? []).filter((tag) => tag !== AUTH_TAG);
  const tags = auth ? [...others, AUTH_TAG] : others;
  const { tags: _previous, ...info } = scenario.info;
  return { ...scenario, info: tags.length > 0 ? { ...info, tags } : info };
}

const isTypedBody = (body: Body | undefined): body is TypedBody =>
  typeof body === "object" && body !== null && !Array.isArray(body) && typeof body["type"] === "string";

/** The body when it is one cactus edits, else `null`. */
export function typedBody(scenario: Scenario): TypedBody | null {
  const { body } = scenario.http;
  return isTypedBody(body) ? body : null;
}

/** What the body editor shows; `other` is a body cactus keeps but doesn't edit. */
export function bodyKind(scenario: Scenario): BodyKind | "other" {
  const { body } = scenario.http;
  if (body === undefined || body === null) return "none";
  if (!isTypedBody(body)) return "other";
  return (BODY_KINDS as readonly string[]).includes(body.type) ? (body.type as BodyKind) : "other";
}

export const isTextBodyKind = (kind: string): kind is TextBodyKind =>
  (TEXT_BODY_KINDS as readonly string[]).includes(kind);

export function bodyText(scenario: Scenario): string {
  const { body } = scenario.http;
  return isTypedBody(body) && typeof body.data === "string" ? body.data : "";
}

export function withBody(scenario: Scenario, body: TypedBody | undefined): Scenario {
  const { body: _previous, ...http } = scenario.http;
  return { ...scenario, http: body ? { ...http, body } : http };
}

export const withBodyText = (scenario: Scenario, kind: TextBodyKind, data: string): Scenario =>
  withBody(scenario, { type: kind, data });

const formFields = (scenario: Scenario): FormField[] => {
  const { body } = scenario.http;
  if (!isTypedBody(body)) return [];
  if (body.type === "form-urlencoded") return body.data;
  if (body.type === "multipart-form") {
    return body.data
      .filter((field) => field.type === "text" && typeof field.value === "string")
      .map(({ name, value, disabled }): FormField =>
        disabled ? { name, value: value as string, disabled } : { name, value: value as string },
      );
  }
  return [];
};

const multipartFields = (scenario: Scenario): MultipartField[] => {
  const { body } = scenario.http;
  if (isTypedBody(body) && body.type === "multipart-form") return body.data;
  return formFields(scenario).map(({ name, value, disabled }): MultipartField =>
    disabled ? { name, value, type: "text", disabled } : { name, value, type: "text" },
  );
};

/** Switch the body to `next`, keeping what carries over: text stays text, form fields stay fields. */
export function switchBodyKind(scenario: Scenario, next: BodyKind): Scenario {
  if (next === "none") return withBody(scenario, undefined);
  if (isTextBodyKind(next)) return withBodyText(scenario, next, bodyText(scenario));
  if (next === "form-urlencoded") return withBody(scenario, { type: next, data: formFields(scenario) });
  return withBody(scenario, { type: next, data: multipartFields(scenario) });
}

/** Why JSON body text wouldn't parse, ignoring `{{placeholders}}`; `null` when fine or blank. */
export function jsonProblem(text: string): string | null {
  if (!text.trim()) return null;
  try {
    JSON.parse(text.replace(PLACEHOLDER, "0"));
    return null;
  } catch (error) {
    return error instanceof Error ? error.message : String(error);
  }
}

const isAuthMode = (auth: Auth): auth is AuthMode =>
  typeof auth === "object" && auth !== null && !Array.isArray(auth) && typeof auth.type === "string";

export function authKind(scenario: Scenario): AuthKind | "other" {
  const { auth } = scenario.http;
  if (auth === undefined || auth === null) return "none";
  if (auth === "inherit") return "inherit";
  if (isAuthMode(auth) && (AUTH_KINDS as readonly string[]).includes(auth.type)) return auth.type;
  return "other";
}

/** The `type` of an auth mode cactus keeps but doesn't send, for display. */
export function otherAuthType(scenario: Scenario): string {
  const { auth } = scenario.http;
  const type = typeof auth === "object" && auth !== null && !Array.isArray(auth) ? auth["type"] : undefined;
  return typeof type === "string" ? type : "custom";
}

export function withAuth(scenario: Scenario, auth: Auth | undefined): Scenario {
  const { auth: _previous, ...http } = scenario.http;
  return { ...scenario, http: auth === undefined ? http : { ...http, auth } };
}

export const authMode = (scenario: Scenario): AuthMode | null => {
  const { auth } = scenario.http;
  return auth !== undefined && auth !== null && isAuthMode(auth) ? auth : null;
};

/** `:name` segments of the URL's path, in order. */
export function pathParamNames(url: string): string[] {
  const path = url.split(/[?#]/)[0] ?? "";
  return path
    .split("/")
    .filter((segment) => segment.startsWith(PATH_PARAM_PREFIX) && segment.length > 1)
    .map((segment) => segment.slice(PATH_PARAM_PREFIX.length));
}

export const pathParams = (scenario: Scenario): Param[] =>
  (scenario.http.params ?? []).filter((param) => param.type === "path");

/**
 * Set the URL and keep params in step with it, as the backend does on save
 * (`Scenario::sync_query_params`), so what is saved reads back unchanged:
 * enabled query params take the URL's pairs in order, disabled ones stay, and
 * path params follow the URL's `:name` segments, keeping their values.
 */
export function withUrl(scenario: Scenario, url: string): Scenario {
  const pairs = parseQuery(url).map((row) => ({ name: row.key, value: row.value }));
  const names = pathParamNames(url);
  const params: Param[] = [];
  for (const param of scenario.http.params ?? []) {
    if (param.type === "path") {
      if (names.includes(param.name) && !params.some((p) => p.type === "path" && p.name === param.name))
        params.push(param);
    } else if (param.disabled) {
      params.push(param);
    } else {
      const pair = pairs.shift();
      if (pair) params.push({ name: pair.name, value: pair.value, type: "query" });
    }
  }
  params.push(...pairs.map(({ name, value }): Param => ({ name, value, type: "query" })));
  for (const name of names) {
    if (!params.some((p) => p.type === "path" && p.name === name)) params.push({ name, value: "", type: "path" });
  }
  const { params: _previous, ...http } = scenario.http;
  return { ...scenario, http: params.length > 0 ? { ...http, url, params } : { ...http, url } };
}

/** The query table: the URL's pairs, which are sent, then the disabled query params, which aren't. */
export const queryRows = (scenario: Scenario): Row[] => [
  ...parseQuery(scenario.http.url),
  ...(scenario.http.params ?? [])
    .filter((param) => param.type === "query" && param.disabled)
    .map((param) => makeRow(param.name, param.value, false)),
];

/** Enabled rows go into the URL; disabled ones are kept as disabled query params. */
export function withQueryRows(scenario: Scenario, rows: Row[]): Scenario {
  const url = withQuery(
    scenario.http.url,
    rows.filter((row) => row.enabled),
  );
  const disabled = rows
    .filter((row) => !row.enabled)
    .map((row): Param => ({ name: row.key, value: row.value, type: "query", disabled: true }));
  const kept = (scenario.http.params ?? []).filter((param) => !(param.type === "query" && param.disabled));
  return withUrl({ ...scenario, http: { ...scenario.http, params: [...kept, ...disabled] } }, url);
}

/** Empty lists are left out, as the backend writes them. */
export function withHeaders(scenario: Scenario, headers: Header[]): Scenario {
  const { headers: _previous, ...http } = scenario.http;
  return { ...scenario, http: headers.length > 0 ? { ...http, headers } : http };
}

export function withPathParamValues(scenario: Scenario, values: Record<string, string>): Scenario {
  const params = (scenario.http.params ?? []).map((param): Param => {
    if (param.type !== "path" || !(param.name in values)) return param;
    const filled: Param = { name: param.name, value: values[param.name] ?? "", type: param.type };
    if (param.disabled) filled.disabled = true;
    return filled;
  });
  return { ...scenario, http: { ...scenario.http, params } };
}
