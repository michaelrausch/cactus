export const HTTP_METHODS = ["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"] as const;

/** Body kinds the editor offers, by their OpenCollection `type`. */
export const BODY_KINDS = ["none", "json", "text", "xml", "form-urlencoded", "multipart-form"] as const;
export type BodyKind = (typeof BODY_KINDS)[number];

export const BODY_KIND_LABELS: Record<BodyKind, string> = {
  none: "None",
  json: "JSON",
  text: "Text",
  xml: "XML",
  "form-urlencoded": "Form",
  "multipart-form": "Multipart",
};

/** Body kinds edited as text. */
export const TEXT_BODY_KINDS = ["json", "text", "xml"] as const;
export type TextBodyKind = (typeof TEXT_BODY_KINDS)[number];

export const AUTH_KINDS = ["none", "inherit", "bearer", "basic", "apikey"] as const;
export type AuthKind = (typeof AUTH_KINDS)[number];

export const AUTH_KIND_LABELS: Record<AuthKind, string> = {
  none: "No auth",
  inherit: "Inherit from folder",
  bearer: "Bearer token",
  basic: "Basic auth",
  apikey: "API key",
};

export const isAuthKind = (value: string): value is AuthKind => (AUTH_KINDS as readonly string[]).includes(value);

/** Operators cactus checks, as Bruno names them. Mirrors `ASSERTION_OPERATORS` in core (checked by core/tests/ui_contract.rs). */
export const ASSERTION_OPERATORS = [
  "eq",
  "neq",
  "gt",
  "gte",
  "lt",
  "lte",
  "in",
  "notIn",
  "contains",
  "notContains",
  "startsWith",
  "endsWith",
  "matches",
  "notMatches",
  "length",
  "isDefined",
  "isUndefined",
  "isNull",
  "isNotNull",
  "isEmpty",
  "isNotEmpty",
  "isString",
  "isNumber",
  "isBoolean",
  "isArray",
  "isJson",
  "isTruthy",
  "isFalsy",
] as const;
