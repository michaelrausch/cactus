import { describe, expect, it } from "vitest";
import { suggestedTitle } from "./http";

describe("suggestedTitle", () => {
  it("names a request after the last meaningful part of its URL", () => {
    expect(suggestedTitle("GET", "{{baseUrl}}/users/{{id}}")).toBe("Get users");
    expect(suggestedTitle("POST", "https://api.example.com/v1/user-profiles?x=1")).toBe("Post user profiles");
    expect(suggestedTitle("DELETE", "/orders/:orderId/items/42")).toBe("Delete items");
  });

  it("has nothing to suggest without a path", () => {
    expect(suggestedTitle("GET", "")).toBe("");
    expect(suggestedTitle("GET", "{{baseUrl}}")).toBe("");
    expect(suggestedTitle("GET", "https://example.com/")).toBe("");
  });
});
