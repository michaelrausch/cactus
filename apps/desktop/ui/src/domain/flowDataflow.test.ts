import { describe, expect, it } from "vitest";
import type { Scenario } from "@/api/client";
import { flowDataflow, placeholderNames, requestCaptures, requestTemplates, type ToolDataflowOf } from "./flowDataflow";

const request = (patch: Partial<Scenario["http"]> = {}, runtime?: Scenario["runtime"]): Scenario => ({
  info: { name: "", type: "http" },
  http: { method: "GET", url: "", ...patch },
  ...(runtime ? { runtime } : {}),
});

const capture = (name: string, phase?: "before-request") => ({
  type: "set-variable" as const,
  ...(phase ? { phase } : {}),
  selector: { expression: "res.body.x", method: "jsonq" },
  variable: { name, scope: "runtime" },
});

/** Stands in for the tool registry: a prompt sets two names, anything else reads two. */
const toolDataflow: ToolDataflowOf = (step) =>
  step.tool === "prompt"
    ? { templates: [], sets: ["id", "userId"], problems: [] }
    : { templates: ["{{id}} {{nope}}"], sets: [], problems: ["No URL"] };

describe("flow data flow", () => {
  const login = request(
    { method: "POST", url: "{{baseUrl}}/login", body: { type: "json", data: '{"u":"{{user}}"}' } },
    {
      actions: [capture("token"), capture("early", "before-request")],
    },
  );
  const me = request({
    url: "{{baseUrl}}/me/{{id}}",
    headers: [
      { name: "Authorization", value: "Bearer {{token}}" },
      { name: "X-Off", value: "{{off}}", disabled: true },
    ],
    params: [{ name: "id", value: "{{userId}}", type: "path" }],
  });
  const requests = { "login.yml": login, "me.yml": me, "loading.yml": undefined, "gone.yml": null };
  const scope = { baseUrl: { kind: "environment", name: "staging" } } as const;

  it("reads every templated part of a request", () => {
    expect(placeholderNames(requestTemplates(me))).toEqual(["baseUrl", "id", "token", "userId"]);
    expect(requestCaptures(login)).toEqual(["token"]);
  });

  it("follows captures from step to step and labels other sources", () => {
    const flow = flowDataflow(
      [
        { request: "login.yml", with: { user: "bob" } },
        { request: "me.yml", capture: { id: "res.body.id" } },
        { request: "me.yml" },
      ],
      requests,
      scope,
      "Bob",
    );
    expect(flow.uses[0]).toEqual([
      { name: "baseUrl", source: { kind: "scope", label: "staging" } },
      { name: "user", source: { kind: "with" } },
    ]);
    expect(flow.uses[2]?.find((u) => u.name === "token")?.source).toEqual({ kind: "step", step: 0 });
    expect(flow.uses[2]?.find((u) => u.name === "id")?.source).toEqual({ kind: "step", step: 1 });
    expect(flow.captures).toEqual([["token"], ["id"], []]);
    expect(flow.passing).toEqual([["token"], ["id", "token"]]);
    expect(flow.problems.map((p) => [p.step, p.message])).toEqual([
      [1, "{{id}} isn't set by anything"],
      [1, "{{userId}} isn't set by anything"],
      [2, "{{userId}} isn't set by anything"],
    ]);
  });

  it("takes what tool steps read, set and get wrong from the tool registry", () => {
    const flow = flowDataflow(
      [{ tool: "prompt" }, { request: "me.yml" }, { tool: "webhook" }],
      requests,
      { ...scope, token: { kind: "environment", name: "staging" } },
      "Bob",
      toolDataflow,
    );
    expect(flow.captures).toEqual([["id", "userId"], [], []]);
    expect(flow.passing).toEqual([["id", "userId"], ["id"]]);
    expect(flow.uses[2]).toEqual([
      { name: "id", source: { kind: "step", step: 0 } },
      { name: "nope", source: { kind: "missing" } },
    ]);
    expect(flow.problems.map((p) => [p.step, p.message])).toEqual([
      [2, "No URL"],
      [2, "{{nope}} isn't set by anything"],
    ]);
  });

  it("reports missing requests, and waits for ones still loading", () => {
    const flow = flowDataflow([{ request: "gone.yml" }, { request: "" }, { request: "loading.yml" }], requests, scope);
    expect(flow.problems.map((p) => p.message)).toEqual(["gone.yml doesn't exist or doesn't parse", "No request"]);
  });
});
