import { describe, expect, it } from "vitest";
import { BODY_EXPRESSION, captureNameFor, childExpression } from "./jsonPick";

describe("captureNameFor", () => {
  it("names a capture after its last key", () => {
    expect(captureNameFor("res.body.data.access_token")).toBe("access_token");
    expect(captureNameFor("res.body['access-token']")).toBe("access_token");
    expect(captureNameFor("res.body.items[0]")).toBe("value");
    expect(captureNameFor("res.body")).toBe("body");
  });
});

describe("childExpression", () => {
  it("writes keys and indexes the way expressions read them", () => {
    expect(childExpression(BODY_EXPRESSION, "token", false)).toBe("res.body.token");
    expect(childExpression("res.body.items", "0", true)).toBe("res.body.items[0]");
    expect(childExpression(BODY_EXPRESSION, "content-type", false)).toBe("res.body['content-type']");
    expect(childExpression(BODY_EXPRESSION, "it's", false)).toBe("res.body['it\\'s']");
  });
});
