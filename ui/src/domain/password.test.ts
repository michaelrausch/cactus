import { describe, expect, it } from "vitest";
import { autoLockLabel, newPasswordProblem } from "./password";

describe("new passwords", () => {
  it("need eight characters and a matching confirmation", () => {
    expect(newPasswordProblem("seven77", "seven77")).toBe("Use at least 8 characters");
    expect(newPasswordProblem("password1", "password2")).toBe("Passwords don't match");
    expect(newPasswordProblem("password1", "password1")).toBeNull();
  });

  it("count characters, not UTF-16 units", () => {
    expect(newPasswordProblem("🌵🌵🌵🌵", "🌵🌵🌵🌵")).not.toBeNull();
    expect(newPasswordProblem("🌵🌵🌵🌵🌵🌵🌵🌵", "🌵🌵🌵🌵🌵🌵🌵🌵")).toBeNull();
  });

  it("label auto-lock choices", () => {
    expect([5, 60, 240, null].map(autoLockLabel)).toEqual(["After 5 min", "After 1 hour", "After 4 hours", "Never"]);
  });
});
