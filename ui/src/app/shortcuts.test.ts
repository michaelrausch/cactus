import { describe, expect, it } from "vitest";
import { shortcutForKey, shortcutLabel, withShortcut } from "./shortcuts";

describe("shortcuts", () => {
  it("labels and looks up keys", () => {
    expect(shortcutLabel("send")).toBe("Ctrl+Enter");
    expect(withShortcut("New request", "newScenario")).toBe("New request (Ctrl+N)");
    expect(shortcutForKey("p")).toBe("switcher");
    expect(shortcutForKey("B")).toBe("critters");
    expect(shortcutLabel("critters")).toBe("Ctrl+B");
    expect(shortcutForKey("z")).toBeUndefined();
  });
});
