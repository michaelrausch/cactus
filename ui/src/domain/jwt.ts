/**
 * Reading captured tokens. Only decodes: nothing here verifies a signature,
 * so the claims are shown as information, never trusted.
 */

export interface DecodedJwt {
  header: Record<string, unknown>;
  claims: Record<string, unknown>;
  /** Epoch millis from `exp`, if it has one. */
  expiresAt: number | null;
}

const BASE64URL = /^[A-Za-z0-9_-]+$/;

const isObject = (value: unknown): value is Record<string, unknown> =>
  typeof value === "object" && value !== null && !Array.isArray(value);

function decodePart(part: string): unknown {
  const base64 = part
    .replace(/-/g, "+")
    .replace(/_/g, "/")
    .padEnd(Math.ceil(part.length / 4) * 4, "=");
  const bytes = Uint8Array.from(atob(base64), (char) => char.charCodeAt(0));
  return JSON.parse(new TextDecoder().decode(bytes));
}

/** The header and claims of `value` if it looks like a JWT, otherwise null. A `Bearer ` prefix is ignored. */
export function decodeJwt(value: string): DecodedJwt | null {
  const token = value.trim().replace(/^Bearer\s+/i, "");
  const [headerPart, claimsPart, signature, ...rest] = token.split(".");
  // An unsigned token (`alg: none`) has an empty signature.
  if (headerPart === undefined || claimsPart === undefined || signature === undefined || rest.length > 0) return null;
  if (!BASE64URL.test(headerPart) || !BASE64URL.test(claimsPart) || !/^[A-Za-z0-9_-]*$/.test(signature)) return null;
  try {
    const header = decodePart(headerPart);
    const claims = decodePart(claimsPart);
    if (!isObject(header) || !isObject(claims)) return null;
    const exp = claims.exp;
    return { header, claims, expiresAt: typeof exp === "number" ? exp * 1000 : null };
  } catch {
    return null;
  }
}

const SECOND = 1000;
const MINUTE = 60 * SECOND;
const HOUR = 60 * MINUTE;
const DAY = 24 * HOUR;

/** `"45s"`, `"4m"`, `"3h"`, `"2d"`. */
export function shortDuration(ms: number): string {
  const abs = Math.abs(ms);
  if (abs < MINUTE) return `${Math.floor(abs / SECOND)}s`;
  if (abs < HOUR) return `${Math.floor(abs / MINUTE)}m`;
  if (abs < 2 * DAY) return `${Math.floor(abs / HOUR)}h`;
  return `${Math.floor(abs / DAY)}d`;
}

/** Both in epoch millis, like `Date.now()`. */
export const isExpired = (expiresAt: number | null, now: number): boolean => expiresAt !== null && expiresAt <= now;

/** Names of the captured values that are JWTs past their `exp`. */
export const expiredTokens = (values: Record<string, { value: string }>, now: number): string[] =>
  Object.entries(values)
    .filter(([, entry]) => isExpired(decodeJwt(entry.value)?.expiresAt ?? null, now))
    .map(([name]) => name);

/** `"expires in 4m"` or `"expired 2h ago"`. */
export const expiryText = (expiresAt: number, now: number): string =>
  expiresAt > now ? `expires in ${shortDuration(expiresAt - now)}` : `expired ${shortDuration(now - expiresAt)} ago`;
