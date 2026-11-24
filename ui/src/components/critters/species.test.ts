import { readFileSync } from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";
import { THEMES } from "@/theme/themes";
import { SPECIES, SPECIES_IDS, SPRITE_SIZE, framesOf, paletteOf, pixelRuns, seedOf, spriteFrames } from "./species";

describe("critters.css", () => {
  // Read from disk: Vitest replaces CSS imports with empty strings.
  const css = readFileSync(path.join(import.meta.dirname, "critters.css"), "utf8");
  const lightBlock = css.slice(css.indexOf("/* Light themes"), css.indexOf("drop-shadow"));
  const outlined = new Set([...lightBlock.matchAll(/data-theme="([^"]+)"/g)].map((match) => match[1]));

  it("outlines sprites on every light theme, and only there", () => {
    for (const theme of THEMES) expect(outlined.has(theme.id), theme.id).toBe(theme.scheme === "light");
  });
});

describe("species", () => {
  it("covers every species the backend knows", () => {
    expect(SPECIES_IDS).toEqual([
      "roadrunner",
      "tortoise",
      "gecko",
      "elf-owl",
      "armadillo",
      "scorpion",
      "jackalope",
      "tumbleweed",
    ]);
  });

  describe.each(SPECIES_IDS)("%s", (id) => {
    const species = SPECIES[id];
    const frames = framesOf(species);
    const palette = paletteOf(species);

    it("draws every frame 16×16 with known colours", () => {
      for (const [state, list] of Object.entries(frames)) {
        for (const rows of list) {
          expect(rows, state).toHaveLength(SPRITE_SIZE);
          for (const row of rows) {
            expect(row, `${state}: ${row}`).toHaveLength(SPRITE_SIZE);
            for (const char of row) if (char !== ".") expect(palette, `${state}: ${char}`).toHaveProperty(char);
          }
        }
      }
    });

    it("has eyes to blink, a stride that fits, and frames per state", () => {
      expect(species.body.join("")).toContain("e");
      expect(species.stride.from + species.stride.rows.length).toBe(SPRITE_SIZE);
      expect([frames.idle.length, frames.work.length, frames.pass.length, frames.fail.length]).toEqual([2, 2, 1, 1]);
      expect(frames.idle[1]).not.toEqual(frames.idle[0]);
      expect(frames.work[1]).not.toEqual(frames.work[0]);
    });

    it("draws with few enough rects", () => {
      for (const state of ["idle", "work", "pass", "fail"] as const) {
        for (const frame of spriteFrames(id, state)) expect(frame.runs.length).toBeLessThanOrEqual(90);
      }
    });
  });

  it("merges runs of one colour and skips transparent pixels", () => {
    expect(pixelRuns(["aab.", ".bbb"], { a: "red", b: "blue" })).toEqual([
      { x: 0, y: 0, width: 2, fill: "red" },
      { x: 2, y: 0, width: 1, fill: "blue" },
      { x: 1, y: 1, width: 3, fill: "blue" },
    ]);
  });

  it("caches frames and seeds stably", () => {
    expect(spriteFrames("gecko", "idle")).toBe(spriteFrames("gecko", "idle"));
    expect(seedOf("bob")).toBe(seedOf("bob"));
    expect(seedOf("bob")).not.toBe(seedOf("alice"));
    expect(seedOf("bob")).toBeGreaterThanOrEqual(0);
    expect(seedOf("bob")).toBeLessThan(1);
  });
});
