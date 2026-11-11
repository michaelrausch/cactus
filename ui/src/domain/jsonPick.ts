/** Expressions for values picked out of a response body, as Bruno writes them. */

/** The expression for the whole body. */
export const BODY_EXPRESSION = "res.body";

const IDENTIFIER = /^[A-Za-z_$][\w$]*$/;

/**
 * A variable name for capturing `expression`: its last key, in letters,
 * digits and `_` (`res.body.data['access-token']` → `access_token`), or
 * `value` when there isn't one (`res.body[0]`).
 */
export function captureNameFor(expression: string): string {
  const last = /(?:\.([A-Za-z_$][\w$]*)|\['((?:[^'\\]|\\.)*)'\]|\[(\d+)\])$/.exec(expression);
  const key = last?.[1] ?? last?.[2];
  const name = key?.replace(/\W+/g, "_").replace(/^_+|_+$/g, "");
  return name || "value";
}

/** `res.body` + `token` → `res.body.token`; `a b` → `res.body['a b']`; an array index → `res.body[0]`. */
export function childExpression(parent: string, key: string, inArray: boolean): string {
  if (inArray) return `${parent}[${key}]`;
  if (IDENTIFIER.test(key)) return `${parent}.${key}`;
  return `${parent}['${key.replace(/\\/g, "\\\\").replace(/'/g, "\\'")}']`;
}
