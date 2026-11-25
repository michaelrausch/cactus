import { beforeEach, describe, expect, it } from "vitest";
import type { Flow, FlowStepResult, FlowStepStart } from "@/api/client";
import { selectFlowDirty, useFlowStore } from "./flowStore";

const flow = (...requests: string[]): Flow => ({ name: "F", steps: requests.map((request) => ({ request })) });
const store = () => useFlowStore.getState();

const result = (index: number, ok = true): FlowStepResult => ({
  index,
  request: "a.yml",
  tool: null,
  output: null,
  title: "A",
  method: "GET",
  url: "http://h/a",
  ok,
  skipped: false,
  response: null,
  error: null,
  watched: {},
});

const start = (index: number): FlowStepStart => ({
  index,
  request: "a.yml",
  tool: null,
  title: "A",
  method: "GET",
  url: `http://h/${index}`,
});

describe("flowStore", () => {
  beforeEach(() => {
    store().closed();
    store().opened({ critter: "bob", flow: "f" }, flow("a"), "name: F\n", {});
  });

  it("keeps edits made while a save was in flight", () => {
    store().edited(flow("a", "b"));
    // Saved [a, b]; then the user added c before the save came back.
    store().edited(flow("a", "b", "c"));
    store().savedBehindEdits(flow("a", "b"), "name: F\nsteps: [a, b]\n");
    expect(store().model?.steps).toHaveLength(3);
    expect(selectFlowDirty(store())).toBe(true);
    store().undone();
    expect(selectFlowDirty(store())).toBe(false);
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

  it("shows the run and follows it to the step that fails", () => {
    store().edited(flow("a", "b", "c"));
    store().runStarted(3);
    expect(store().panel).toBe("run");
    store().stepStarted(start(0));
    store().stepFinished(result(0));
    store().stepStarted(start(1));
    expect(store().selected).toBe(1);
    expect(store().run.started[1]?.url).toBe("http://h/1");
    store().stepFinished(result(1, false));
    store().runFinished({ at: 0, env: null, ok: false, elapsedMs: 5, steps: [result(0), result(1, false), result(2)] });
    expect(store().selected).toBe(1);

    // Picking another step stops following.
    store().runStarted(3);
    store().select(2);
    store().stepStarted(start(0));
    expect(store().selected).toBe(2);
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
