import { describe, expect, it } from "vitest";
import type { Scenario } from "@/api/client";
import { bodyToEditorText, inferBodyType, parseLooseJson, switchBodyType, withAuth } from "./scenario";

const scenario = (patch: Partial<Scenario> = {}): Scenario => ({
  title: "",
  description: "",
  method: "GET",
  url: "",
  headers: {},
  body: null,
  ...patch,
});

describe("scenario", () => {
  it("infers the body type like the backend", () => {
    expect(inferBodyType(scenario())).toBe("none");
    expect(inferBodyType(scenario({ body: "x" }))).toBe("text");
    expect(inferBodyType(scenario({ body: { a: 1 } }))).toBe("json");
    expect(inferBodyType(scenario({ body: { a: "1" }, bodyType: "form" }))).toBe("form");
  });

  it("renders bodies for the editor", () => {
    expect(bodyToEditorText(null)).toBe("");
    expect(bodyToEditorText("raw")).toBe("raw");
    expect(bodyToEditorText({ a: 1 })).toBe('{\n  "a": 1\n}');
  });

  it("parses loosely", () => {
    expect(parseLooseJson("true")).toBe(true);
    expect(parseLooseJson("not json")).toBe("not json");
  });

  describe("switchBodyType", () => {
    const form = scenario({ bodyType: "form", body: { a: "1", n: 2 } });

    it("none drops the body and the explicit type", () => {
      const next = switchBodyType(form, "none", "");
      expect(next.body).toBeNull();
      expect("bodyType" in next).toBe(false);
    });

    it("text keeps the editor text", () => {
      expect(switchBodyType(form, "text", "hello")).toMatchObject({ bodyType: "text", body: "hello" });
    });

    it("json parses the editor text, or starts an object", () => {
      expect(switchBodyType(form, "json", '{"x":1}').body).toEqual({ x: 1 });
      expect(switchBodyType(form, "json", "not json").body).toEqual({});
      expect(switchBodyType(form, "json", "").body).toBeNull();
    });

    it("form keeps only string fields", () => {
      expect(switchBodyType(form, "multipart", "").body).toEqual({ a: "1" });
      expect(switchBodyType(scenario({ body: "raw" }), "form", "raw").body).toEqual({});
    });
  });

  it("removes auth instead of storing null", () => {
    const withBearer = withAuth(scenario(), { type: "bearer", token: "t" });
    expect(withBearer.auth).toEqual({ type: "bearer", token: "t" });
    expect("auth" in withAuth(withBearer, null)).toBe(false);
  });
});
