import { describe, expect, it } from "vitest";
import { decodeJwt, expiredTokens, expiryText, isExpired, shortDuration } from "./jwt";

const base64url = (value: unknown) =>
  btoa(String.fromCharCode(...new TextEncoder().encode(JSON.stringify(value))))
    .replace(/\+/g, "-")
    .replace(/\//g, "_")
    .replace(/=+$/, "");

const jwt = (claims: unknown) => `${base64url({ alg: "HS256", typ: "JWT" })}.${base64url(claims)}.c2ln`;

describe("jwt", () => {
  it("decodes claims and expiry", () => {
    const decoded = decodeJwt(jwt({ sub: "bob", name: "Zoë", exp: 1_700_000_000 }));
    expect(decoded?.claims).toEqual({ sub: "bob", name: "Zoë", exp: 1_700_000_000 });
    expect(decoded?.header.alg).toBe("HS256");
    expect(decoded?.expiresAt).toBe(1_700_000_000_000);
    expect(decodeJwt(`Bearer ${jwt({ sub: "x" })}`)?.expiresAt).toBeNull();
    const unsigned = jwt({ sub: "x" }).replace(/c2ln$/, "");
    expect(decodeJwt(unsigned)?.claims).toEqual({ sub: "x" });
  });

  it("ignores values that aren't tokens", () => {
    for (const value of ["", "abc", "a.b", "a.b.c", "not a token at all", `${base64url([1])}.${base64url({})}.x`]) {
      expect(decodeJwt(value), value).toBeNull();
    }
  });

  it("describes expiry", () => {
    const now = 1_000_000_000;
    expect(expiryText(now + 4 * 60_000 + 5_000, now)).toBe("expires in 4m");
    expect(expiryText(now - 2 * 3_600_000, now)).toBe("expired 2h ago");
    expect(shortDuration(45_000)).toBe("45s");
    expect(shortDuration(3 * 86_400_000)).toBe("3d");
    expect(isExpired(now, now)).toBe(true);
    expect(isExpired(null, now)).toBe(false);
  });

  it("finds expired tokens among captured values", () => {
    const now = 2_000_000_000_000;
    const values = {
      old: { value: jwt({ exp: 1_000_000_000 }) },
      fresh: { value: jwt({ exp: 3_000_000_000 }) },
      plain: { value: "abc" },
    };
    expect(expiredTokens(values, now)).toEqual(["old"]);
  });
});
