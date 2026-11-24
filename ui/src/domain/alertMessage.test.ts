import { describe, expect, it } from "vitest";
import { alertMessage } from "./alertMessage";

describe("alertMessage", () => {
  it("words alerts", () => {
    const failing = { kind: "failing", step: 1, request: "status.yml", reason: "status 500" } as const;
    expect(alertMessage(failing, "Bob", "Health")).toEqual({
      title: "Health failing",
      description: "Step 2 (status.yml): status 500",
      tone: "warning",
    });
    expect(alertMessage({ kind: "recovered" }, "Bob", "Health").tone).toBe("success");
    const changed = { kind: "changed", step: 0, expression: "res.body.version", before: "1", after: "2" } as const;
    expect(alertMessage(changed, "Bob", "Health").description).toBe("1 → 2");
  });
});
