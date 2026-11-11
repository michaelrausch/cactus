import type { Species } from "@/api/client";

/**
 * Critter pixel art. Each species is one 16×16 drawing, one character per
 * pixel (`.` is transparent), plus the rows that change mid-stride. Every
 * other frame is derived: blinking, and the eyes for passing and failing.
 */

export type CritterState = "idle" | "work" | "pass" | "fail";

export const SPRITE_SIZE = 16;

/** The eye pixel, swapped per frame. */
const EYE = "e";

/** Colours every species shares: the eye, and the eye when passing or failing. */
const SHARED_PALETTE: Record<string, string> = {
  [EYE]: "#1b1b1f",
  w: "#fff6c9",
  x: "#e0483e",
};

export interface SpeciesInfo {
  name: string;
  /** One line for the species picker. */
  description: string;
  /** The critter's colour in the UI, e.g. the editor's top line. */
  accent: string;
  /** Character → colour, besides the shared ones. */
  palette: Record<string, string>;
  /** The drawing, 16 rows of 16 characters. */
  body: readonly string[];
  /** The second `work` frame: rows from `from` down, mid-stride. */
  stride: { from: number; rows: readonly string[] };
  /** What a closed eye looks like: usually the skin around it. */
  lid: string;
}

const blank = (rows: number): string[] => Array.from({ length: rows }, () => ".".repeat(SPRITE_SIZE));

export const SPECIES: Record<Species, SpeciesInfo> = {
  roadrunner: {
    name: "Roadrunner",
    description: "Quick and impatient. Good for smoke tests.",
    accent: "#4f86c6",
    palette: { b: "#8a6440", t: "#5c4128", l: "#e3cfa8", c: "#4f86c6", y: "#e0a93f" },
    body: [
      ...blank(1),
      ".........cc.....",
      "........ccbb....",
      ".........bbeb...",
      "..t......bbbbyyy",
      "..tt.....bbb....",
      "...tt...bbbb....",
      "....ttbbbbbb....",
      ".....tbbbbbl....",
      "......bbbbll....",
      ".......bbll.....",
      ".......y.y......",
      ".......y.y......",
      ".......y..y.....",
      "......y...y.....",
      ".....yy...yy....",
    ],
    stride: {
      from: 11,
      rows: ["........y.......", "........y.......", "........yy......", "........y.y.....", ".......yy.yy...."],
    },
    lid: "b",
  },
  tortoise: {
    name: "Tortoise",
    description: "Slow and steady. Patient with long flows.",
    accent: "#5f8f4e",
    palette: { s: "#6f9f58", p: "#3f6b37", g: "#b2c47a" },
    body: [
      ...blank(8),
      ".....pppppp.....",
      "....pssssssp....",
      "...psspssspsp...",
      "..pssspsssspsgg.",
      "..pssssssssspgeg",
      "..ppppppppppppgg",
      "...gg.gg.gg.gg..",
      "...gg.......gg..",
    ],
    stride: { from: 14, rows: ["..gg.gg..gg.gg..", "..gg........gg.."] },
    lid: "g",
  },
  gecko: {
    name: "Gecko",
    description: "Sticks to anything. Great for long sessions.",
    accent: "#7cc36a",
    palette: { g: "#7cc36a", d: "#4c9445" },
    body: [
      ...blank(9),
      "...........ggg..",
      ".dd.......gggegg",
      "d..d.gggggggggg.",
      "d...ggdgggdggg..",
      ".dddgggggggg....",
      "....g.g...g.g...",
      "...gg.gg.gg.gg..",
    ],
    stride: { from: 14, rows: [".....g.g.g.g....", "....gg.gg.gg.gg."] },
    lid: "g",
  },
  "elf-owl": {
    name: "Elf owl",
    description: "Small and watchful. Keeps an eye on things.",
    accent: "#c9a14a",
    palette: { b: "#8a6a4a", l: "#dccbab", y: "#f2c94c", o: "#d98b3a" },
    body: [
      ...blank(4),
      "....b......b....",
      "....bbbbbbbb....",
      "...bbyybbyybb...",
      "...byeybbyeyb...",
      "...bbyybbyybb...",
      "...bbbboobbbb...",
      "...bllllllllb...",
      "...blbllllblb...",
      "...bllllllllb...",
      "....bllllllb....",
      ".....bbbbbb.....",
      ".....o.oo.o.....",
    ],
    stride: { from: 15, rows: ["......oo..oo...."] },
    lid: "y",
  },
  armadillo: {
    name: "Armadillo",
    description: "Armoured. Plays the locked-down account.",
    accent: "#a08c7a",
    palette: { a: "#a8968a", s: "#6f6156", p: "#d9a08e" },
    body: [
      ...blank(10),
      "......aaaaa.....",
      "....aasasasaa...",
      "...aasasasasaap.",
      ".paasasasasaapep",
      "p.aaaaaaaaaaappp",
      "...pp.pp..pp.pp.",
    ],
    stride: { from: 15, rows: ["..pp.pp..pp.pp.."] },
    lid: "p",
  },
  scorpion: {
    name: "Scorpion",
    description: "Prickly. Tries what it shouldn't.",
    accent: "#c46a3a",
    palette: { r: "#c46a3a", d: "#7d3f22", t: "#f0c060" },
    body: [
      ...blank(5),
      "....rrr.........",
      "...r...r........",
      "...r....t.......",
      "...r............",
      "...r.........r.r",
      "...rr........rr.",
      "....rrrrrrrr.rr.",
      "....rdrdrdrerr..",
      "....rrrrrrrrr...",
      "...d.d.d.d.d....",
      "..d.d.d.d.d.....",
    ],
    stride: { from: 14, rows: ["....d.d.d.d.d...", "...d.d.d.d.d...."] },
    lid: "r",
  },
  jackalope: {
    name: "Jackalope",
    description: "Rare and a little odd. For edge cases.",
    accent: "#b89a78",
    palette: { b: "#b89a78", l: "#ece2d0", a: "#7a6248", p: "#e39aa6" },
    body: [
      ".........a.a.a..",
      "..........aaa...",
      "...........a....",
      "..........bbb...",
      ".........bbebb..",
      ".........bbbbbp.",
      ".........bbbb...",
      "....bbbbbbbb....",
      "...bbbbbbbbb....",
      "..lbbbbbbbbl....",
      "..lbbbbbbbll....",
      "...bbbbbblll....",
      "....bbbbbbb.....",
      "....bb....bb....",
      "...bbb....bb....",
      "...bbbb...bbb...",
    ],
    stride: { from: 13, rows: [".....bb..bb.....", "....bbb..bb.....", "...bbb....bbb..."] },
    lid: "b",
  },
  tumbleweed: {
    name: "Tumbleweed",
    description: "Drifts in with no credentials at all.",
    accent: "#c2a45e",
    palette: { t: "#c2a45e", d: "#80673a" },
    body: [
      ...blank(8),
      ".....tttttt.....",
      "...ttdttttdtt...",
      "..tttdttdttdtt..",
      "..ttdtettetdtt..",
      "..tdttdttdttdt..",
      "..ttdttdttdttt..",
      "...tdttdttdtt...",
      ".....tdttdt.....",
    ],
    // Rolling: the twigs move round while the face stays put.
    stride: {
      from: 9,
      rows: [
        "...tdttttdttt...",
        "..ttdttdttdttt..",
        "..tdttettetdtt..",
        "..ttdttdttdttt..",
        "..tdttdttdttdt..",
        "...ttdttdttdt...",
        ".....dttdtt.....",
      ],
    },
    lid: "t",
  },
};

export const SPECIES_IDS = Object.keys(SPECIES) as Species[];

const withEyes = (rows: readonly string[], eye: string): string[] => rows.map((row) => row.replaceAll(EYE, eye));

/** The frames of each state, as rows of characters. */
export function framesOf(species: SpeciesInfo): Record<CritterState, string[][]> {
  const { body, stride, lid } = species;
  const striding = [...body.slice(0, stride.from), ...stride.rows];
  return {
    idle: [[...body], withEyes(body, lid)],
    work: [[...body], striding],
    pass: [withEyes(body, "w")],
    fail: [withEyes(body, "x")],
  };
}

/** A horizontal run of one colour. */
export interface PixelRun {
  x: number;
  y: number;
  width: number;
  fill: string;
}

/** Merge each row's same-coloured neighbours into one run, so a frame draws as a few dozen rects. */
export function pixelRuns(rows: readonly string[], palette: Record<string, string>): PixelRun[] {
  const colours = { ...SHARED_PALETTE, ...palette };
  const runs: PixelRun[] = [];
  rows.forEach((row, y) => {
    let x = 0;
    while (x < row.length) {
      const char = row[x] ?? ".";
      let end = x + 1;
      while (row[end] === char) end++;
      const fill = colours[char];
      if (char !== "." && fill) runs.push({ x, y, width: end - x, fill });
      x = end;
    }
  });
  return runs;
}

/** Every character a species draws with, shared colours included. */
export const paletteOf = (species: SpeciesInfo): Record<string, string> => ({ ...SHARED_PALETTE, ...species.palette });

/** One frame, ready to draw. */
export interface SpriteFrame {
  /** `"idle-1"`: unique within a species. */
  key: string;
  /** Its place among the state's frames, which critters.css keys on. */
  index: number;
  runs: PixelRun[];
}

const cache = new Map<string, SpriteFrame[]>();

/** The frames of `state`, computed once per species and state. */
export function spriteFrames(species: Species, state: CritterState): SpriteFrame[] {
  const cacheKey = `${species}:${state}`;
  let frames = cache.get(cacheKey);
  if (!frames) {
    const info = SPECIES[species];
    frames = framesOf(info)[state].map((rows, index) => ({
      key: `${state}-${index}`,
      index,
      runs: pixelRuns(rows, info.palette),
    }));
    cache.set(cacheKey, frames);
  }
  return frames;
}

/** A stable fraction in [0, 1) from `text`, so critters don't blink in unison. */
export function seedOf(text: string): number {
  let hash = 2166136261;
  for (let i = 0; i < text.length; i++) {
    hash ^= text.charCodeAt(i);
    hash = Math.imul(hash, 16777619);
  }
  return (hash >>> 0) / 2 ** 32;
}
