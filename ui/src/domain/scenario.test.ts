import { describe, expect, it } from "vitest";
import type { Scenario } from "@/api/client";
import {
  authKind,
  bodyKind,
  bodyText,
  displayTitle,
  jsonProblem,
  pathParamNames,
  switchBodyKind,
  withAuth,
  withBody,
  withHeaders,
  withPathParamValues,
  withUrl,
} from "./scenario";

const scenario = (http: Partial<Scenario["http"]> = {}): Scenario => ({
  info: { name: "", type: "http" },
  http: { method: "GET", url: "", ...http },
});

describe("scenario", () => {
  it("titles untitled requests", () => {
    expect(displayTitle(scenario())).toBe("Untitled");
  });

  it("reads body kinds", () => {
    expect(bodyKind(scenario())).toBe("none");
    expect(bodyKind(scenario({ body: { type: "json", data: "{}" } }))).toBe("json");
    expect(bodyKind(scenario({ body: { type: "file", data: [] } }))).toBe("other");
    expect(bodyKind(scenario({ body: [{ title: "A", body: { type: "text", data: "" } }] }))).toBe("other");
  });

  describe("switchBodyKind", () => {
    const form = scenario({
      body: {
        type: "form-urlencoded",
        data: [
          { name: "a", value: "1" },
          { name: "b", value: "2", disabled: true },
        ],
      },
    });

    it("none drops the body", () => {
      expect("body" in switchBodyKind(form, "none").http).toBe(false);
    });

    it("text kinds keep the text", () => {
      const json = withBody(scenario(), { type: "json", data: '{"x": {{n}}}' });
      expect(switchBodyKind(json, "text").http.body).toEqual({ type: "text", data: '{"x": {{n}}}' });
      expect(bodyText(switchBodyKind(form, "xml"))).toBe("");
    });

    it("fields carry over between form kinds", () => {
      const multipart = switchBodyKind(form, "multipart-form");
      expect(multipart.http.body).toEqual({
        type: "multipart-form",
        data: [
          { name: "a", value: "1", type: "text" },
          { name: "b", value: "2", disabled: true, type: "text" },
        ],
      });
      expect(switchBodyKind(multipart, "form-urlencoded").http.body).toEqual(form.http.body);
    });
  });

  it("checks JSON but allows placeholders", () => {
    expect(jsonProblem('{"n": {{num}}, "s": "{{s}}"}')).toBeNull();
    expect(jsonProblem("")).toBeNull();
    expect(jsonProblem("{nope")).not.toBeNull();
  });

  it("sets and removes auth", () => {
    expect(authKind(scenario({ auth: "inherit" }))).toBe("inherit");
    const withBearer = withAuth(scenario(), { type: "bearer", token: "t" });
    expect(authKind(withBearer)).toBe("bearer");
    expect("auth" in withAuth(withBearer, undefined).http).toBe(false);
    expect(authKind(scenario({ auth: { type: "oauth2", flow: "implicit" } }))).toBe("other");
  });

  it("keeps path params in step with the URL", () => {
    expect(pathParamNames("{{base}}/users/:id/posts/:postId?x=:no")).toEqual(["id", "postId"]);
    const first = withUrl(scenario(), "/users/:id");
    expect(first.http.params).toEqual([{ name: "id", value: "", type: "path" }]);
    const filled = withPathParamValues(first, { id: "7" });
    const renamed = withUrl(filled, "/users/:id/posts/:post");
    expect(renamed.http.params).toEqual([
      { name: "id", value: "7", type: "path" },
      { name: "post", value: "", type: "path" },
    ]);
    expect("params" in withUrl(renamed, "/users").http).toBe(false);
  });

  it("rebuilds query params like the backend, in place", () => {
    const s = scenario({
      params: [
        { name: "id", value: "7", type: "path" },
        { name: "old", value: "0", type: "query" },
        { name: "off", value: "1", type: "query", disabled: true },
      ],
    });
    // Same case as `query_params_follow_the_url` in core/src/scenario/model.rs.
    const next = withUrl(s, "http://h/:id?a=1&b={{x}}#top");
    expect(next.http.params?.map((p) => [p.name, p.value])).toEqual([
      ["id", "7"],
      ["a", "1"],
      ["off", "1"],
      ["b", "{{x}}"],
    ]);
  });

  it("leaves empty header lists out", () => {
    expect("headers" in withHeaders(scenario({ headers: [{ name: "A", value: "1" }] }), []).http).toBe(false);
  });
});
