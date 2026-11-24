import { describe, expect, it } from "vitest";
import type { HttpResponse } from "@/api/client";
import { formatBytes, stableStringify, timeAgo } from "@/lib/format";
import { methodColor, statusColor } from "@/theme/httpColors";
import { envFileKey, envLayerInfo, envLayers, isRootEnvFile } from "./envFiles";
import { baseName, dirOf, isWithinDir, withJsonExtension } from "./paths";
import { displayBody, indentJsonText, parseBody } from "./responseBody";
import { findMatches, matchCountLabel, stepMatch } from "./search";

describe("paths", () => {
  it("splits and matches folders", () => {
    expect(dirOf("a/b/c.json")).toBe("a/b");
    expect(dirOf("c.json")).toBe("");
    expect(baseName("C:\\work\\api")).toBe("api");
    expect(isWithinDir("auth/login.json", "auth")).toBe(true);
    expect(isWithinDir("authx/login.json", "auth")).toBe(false);
    expect(isWithinDir("x.json", "")).toBe(true);
    expect(withJsonExtension("a/b")).toBe("a/b.json");
  });
});

describe("envFiles", () => {
  it("recognises root env files", () => {
    expect(isRootEnvFile("env.json")).toBe(true);
    expect(isRootEnvFile("env.dev.json")).toBe(true);
    expect(isRootEnvFile("auth/env.json")).toBe(false);
  });

  it("lists layers in precedence order", () => {
    expect(envLayers(["dev"]).map(envFileKey)).toEqual(["base", "named:dev", "secrets", "captured"]);
    expect(envLayerInfo({ kind: "named", name: "dev" }).path).toBe("env.dev.json");
  });
});

const response = (body: string, headers: [string, string][] = [], truncated = false) =>
  ({ body, headers, truncated }) as HttpResponse;

describe("responseBody", () => {
  it("parses JSON only when it plausibly is JSON", () => {
    expect(parseBody(response('{"a":1}'))).toEqual({ kind: "json", value: { a: 1 } });
    expect(parseBody(response("1", [["Content-Type", "application/json"]]))).toEqual({ kind: "json", value: 1 });
    expect(parseBody(response("1"))).toEqual({ kind: "text" });
    expect(parseBody(response('{"a":', [], true))).toEqual({ kind: "partialJson" });
    expect(parseBody(response("plain", [], true))).toEqual({ kind: "text" });
  });

  it("indents JSON text exactly like JSON.stringify", () => {
    const value = { a: [1, { b: "x, y: {z}", c: [] }, {}], 'q\\"': null, n: -1.5e3, t: true };
    expect(indentJsonText(JSON.stringify(value))).toBe(JSON.stringify(value, null, 2));
    expect(indentJsonText(" [ ] ")).toBe("[]");
  });

  it("indents truncated JSON bodies", () => {
    const truncated = response('{"list":[{"code":"E1","name":"Mentoring, Pre-Ser', [], true);
    expect(displayBody(truncated, parseBody(truncated))).toBe(
      '{\n  "list": [\n    {\n      "code": "E1",\n      "name": "Mentoring, Pre-Ser',
    );
  });
});

describe("search", () => {
  it("finds non-overlapping matches case-insensitively", () => {
    expect(findMatches("aAaA", "aa")).toEqual([0, 2]);
    expect(findMatches("abc", "a")).toEqual([]);
    expect(stepMatch(0, -1, 3)).toBe(2);
    expect(matchCountLabel(0, 0)).toBe("0");
    expect(matchCountLabel(1, 4)).toBe("2/4");
  });
});

describe("format and colors", () => {
  it("formats", () => {
    expect(formatBytes(512)).toBe("512 B");
    expect(formatBytes(2048)).toBe("2.0 KB");
    expect(timeAgo(0, 90_000)).toBe("1m");
    expect(stableStringify({ b: 1, a: [1, { d: undefined, c: 2 }] })).toBe('{"a":[1,{"c":2}],"b":1}');
  });

  it("colors by method and status class", () => {
    expect(methodColor("get")).toBe("text-emerald-400");
    expect(methodColor("TRACE")).toBe("text-muted-foreground");
    expect([200, 302, 404, 503].map(statusColor)).toEqual([
      "text-emerald-400",
      "text-sky-400",
      "text-amber-400",
      "text-rose-400",
    ]);
  });
});
