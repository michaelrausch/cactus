import { describe, expect, it } from "vitest";
import { claudeCodeCommand, mcpConfigJson, parsePort } from "./mcp";

const endpoint = { url: "http://127.0.0.1:47821/mcp", token: "abc" };

describe("mcp", () => {
  it("builds client configs with the token as a bearer header", () => {
    expect(JSON.parse(mcpConfigJson(endpoint))).toEqual({
      mcpServers: {
        cactus: { type: "http", url: endpoint.url, headers: { Authorization: "Bearer abc" } },
      },
    });
    expect(claudeCodeCommand(endpoint)).toBe(
      'claude mcp add --transport http cactus http://127.0.0.1:47821/mcp --header "Authorization: Bearer abc"',
    );
  });

  it("accepts only unprivileged ports", () => {
    expect(parsePort(" 47821 ")).toBe(47821);
    expect(parsePort("80")).toBeNull();
    expect(parsePort("70000")).toBeNull();
    expect(parsePort("12a")).toBeNull();
    expect(parsePort("")).toBeNull();
  });
});
