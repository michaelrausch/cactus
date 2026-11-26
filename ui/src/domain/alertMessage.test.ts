import { describe, expect, it } from "vitest";
import type { AlertEvent } from "@/api/client";
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
    const found: AlertEvent = {
      kind: "newFindings",
      step: 0,
      request: "Headers",
      findings: [
        { severity: "high", rule: "headers.hsts" },
        { severity: "medium", rule: "headers.csp" },
      ],
    };
    expect(alertMessage(found, "Bob", "Scan")).toEqual({
      title: "Scan: new findings",
      description: "Step 1 (Headers): 1 high · 1 medium",
      tone: "warning",
    });
  });
});
