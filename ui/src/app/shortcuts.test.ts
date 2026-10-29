import { describe, expect, it } from "vitest";
import { shortcutForKey, shortcutLabel, withShortcut } from "./shortcuts";

describe("shortcuts", () => {
  it("labels and looks up keys", () => {
    expect(shortcutLabel("send")).toBe("Ctrl+Enter");
    expect(withShortcut("New scenario", "newScenario")).toBe("New scenario (Ctrl+N)");
    expect(shortcutForKey("p")).toBe("switcher");
    expect(shortcutForKey("z")).toBeUndefined();
  });
});
