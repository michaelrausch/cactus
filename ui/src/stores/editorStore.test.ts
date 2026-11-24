import { beforeEach, describe, expect, it } from "vitest";
import type { Scenario, ProjectPath } from "@/api/client";
import { dirtyPaths, isDirty, useEditorStore } from "./editorStore";

const scenario = (name: string): Scenario =>
  ({ info: { name }, http: { method: "GET", url: "" } }) as unknown as Scenario;
const a = "a.yml" as ProjectPath;
const b = "b.yml" as ProjectPath;
const store = () => useEditorStore.getState();

describe("editorStore buffers", () => {
  beforeEach(() => store().reset());

  it("is clean until edited, and clean again when the edit is undone", () => {
    store().opened(a, scenario("one"));
    expect(isDirty(store().buffers[a])).toBe(false);
    store().edited(scenario("two"));
    expect(isDirty(store().buffers[a])).toBe(true);
    store().edited(scenario("one"));
    expect(isDirty(store().buffers[a])).toBe(false);
  });

  it("keeps a dirty buffer after switching away, and drops a clean one", () => {
    store().opened(a, scenario("one"));
    store().edited(scenario("edited"));
    store().opened(b, scenario("b"));
    expect(dirtyPaths(store().buffers)).toEqual([a]);
    store().shown(a);
    expect(store().scenario?.info.name).toBe("edited");
    expect(store().buffers[b]).toBeUndefined();
  });

  it("saving clears dirty and the conflict", () => {
    store().opened(a, scenario("one"));
    store().edited(scenario("two"));
    store().conflicted(a, "changed");
    store().saved(a, scenario("two"));
    expect(isDirty(store().buffers[a])).toBe(false);
    expect(store().buffers[a]?.conflict).toBeNull();
  });

  it("reloading replaces the edits and remounts the editor", () => {
    store().opened(a, scenario("one"));
    const version = store().editorVersion;
    store().edited(scenario("two"));
    store().reloaded(a, scenario("disk"));
    expect(store().scenario?.info.name).toBe("disk");
    expect(store().editorVersion).toBe(version + 1);
  });

  it("moves the buffer with a renamed file", () => {
    store().opened(a, scenario("one"));
    store().edited(scenario("two"));
    store().moved(a, b);
    expect(store().path).toBe(b);
    expect(isDirty(store().buffers[b])).toBe(true);
    expect(store().buffers[a]).toBeUndefined();
  });
});
