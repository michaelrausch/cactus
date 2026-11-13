import { describe, expect, it } from "vitest";
import { isSaving, saveGeneration, writingFile } from "./saving";

const deferred = () => {
  let resolve!: () => void;
  const promise = new Promise<void>((r) => (resolve = r));
  return { promise, resolve };
};

describe("writingFile", () => {
  it("runs saves of one file one after another", async () => {
    const first = deferred();
    const order: string[] = [];
    const a = writingFile("a.yml", async () => {
      order.push("a start");
      await first.promise;
      order.push("a end");
    });
    const b = writingFile("a.yml", async () => {
      order.push("b");
    });
    await Promise.resolve();
    expect(order).toEqual(["a start"]);
    expect(isSaving("a.yml")).toBe(true);
    first.resolve();
    await Promise.all([a, b]);
    expect(order).toEqual(["a start", "a end", "b"]);
    expect(isSaving("a.yml")).toBe(false);
  });

  it("lets other files save alongside", async () => {
    const held = deferred();
    const a = writingFile("x.yml", () => held.promise);
    await writingFile("y.yml", async () => {});
    expect(isSaving("x.yml")).toBe(true);
    held.resolve();
    await a;
  });

  it("carries on after a failed save, and counts every save", async () => {
    const before = saveGeneration("f.yml");
    await expect(writingFile("f.yml", () => Promise.reject(new Error("disk full")))).rejects.toThrow();
    await expect(writingFile("f.yml", async () => 1)).resolves.toBe(1);
    expect(saveGeneration("f.yml")).toBe(before + 2);
    expect(isSaving("f.yml")).toBe(false);
  });
});
