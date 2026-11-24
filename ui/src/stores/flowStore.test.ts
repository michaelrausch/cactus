import { beforeEach, describe, expect, it } from "vitest";
import type { Flow, FlowStepResult } from "@/api/client";
import { selectFlowDirty, useFlowStore } from "./flowStore";

const flow = (...requests: string[]): Flow => ({ name: "F", steps: requests.map((request) => ({ request })) });
const store = () => useFlowStore.getState();

const result = (index: number): FlowStepResult => ({
  index,
  request: "a.yml",
  title: "A",
  ok: true,
  skipped: false,
  response: null,
  error: null,
  watched: {},
});

describe("flowStore", () => {
  beforeEach(() => {
    store().closed();
    store().opened({ critter: "bob", flow: "f" }, flow("a"), "name: F\n", {});
  });

  it("undoes and redoes visual edits", () => {
    store().edited(flow("a", "b"));
    store().edited(flow("a", "b", "c"));
    expect(selectFlowDirty(store())).toBe(true);
    store().undone();
    expect(store().model?.steps).toHaveLength(2);
    store().undone();
    expect(store().model).toEqual(flow("a"));
    expect(selectFlowDirty(store())).toBe(false);
    store().undone();
    expect(store().model).toEqual(flow("a"));
    store().redone();
    store().redone();
    expect(store().model?.steps).toHaveLength(3);
    store().undone();
    store().edited(flow("x"));
    expect(store().redo).toHaveLength(0);
  });

  it("caps the history at 100 edits", () => {
    for (let n = 0; n < 120; n++) store().edited(flow(`s${n}`));
    expect(store().undo).toHaveLength(100);
  });

  it("keeps the selection on a step that exists", () => {
    store().edited(flow("a", "b", "c"));
    store().select(2);
    store().edited(flow("a"));
    expect(store().selected).toBe(0);
    store().edited(flow());
    expect(store().selected).toBeNull();
  });

  it("marks run results stale on edit and fills them in as steps finish", () => {
    store().runStarted(2);
    store().stepFinished(result(0));
    expect(store().run.current).toBe(1);
    store().runFinished({ at: 0, env: null, ok: true, elapsedMs: 5, steps: [result(0), result(1)] });
    expect(store().run.running).toBe(false);
    store().edited(flow("a", "b"));
    expect(store().run.stale).toBe(true);
  });

  it("tracks YAML edits separately from the model", () => {
    store().modeChanged("yaml");
    store().textEdited("name: G\n");
    expect(selectFlowDirty(store())).toBe(true);
    store().textParsed(null, { message: "bad", line: 2, column: 1 });
    expect(store().parseProblem?.line).toBe(2);
    store().saved(flow("a"), "name: G\n");
    expect(selectFlowDirty(store())).toBe(false);
    expect(store().parseProblem).toBeNull();
  });
});
