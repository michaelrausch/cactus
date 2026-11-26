# Add a critter species

A critter's `species` (in `critters/<id>/critter.yml`) only picks its sprite and
accent colour; it has no effect on behaviour. Species are defined in core, so
files only ever hold known names, and drawn in the UI as 16×16 pixel art.

This guide adds a `gila-monster`.

## 1. Core

In `crates/core/src/critter/model.rs`, add the variant to **both** enums: `Species`
and the private `Known` enum inside its `Deserialize` impl, and the match arm
between them:

```rust
pub enum Species {
    #[default]
    Roadrunner,
    // …
    Tumbleweed,
    GilaMonster,
}

// inside `impl<'de> Deserialize<'de> for Species`:
enum Known {
    // …
    Tumbleweed,
    GilaMonster,
}
// …
Ok(Known::GilaMonster) => Self::GilaMonster,
```

`#[serde(rename_all = "kebab-case")]` makes `GilaMonster` read and write as
`gila-monster`. The hand-written `Deserialize` is there so a name this build
doesn't know (say, from a newer cactus) reads as the default, `roadrunner`,
instead of failing to load the critter. Keep it that way.

Regenerate the bindings so the UI's `Species` type includes it:

```sh
cargo test -p cactus
```

## 2. Draw it

In `apps/desktop/ui/src/components/critters/species.ts`, add an entry to `SPECIES`. It's
typed `Record<Species, SpeciesInfo>`, so typecheck fails until you do.

```ts
"gila-monster": {
  name: "Gila Monster",
  description: "Slow, but never lets go of a session.",
  accent: "#e07a3f",
  palette: { o: "#e07a3f", k: "#2b2024", p: "#f2b6a0" },
  body: [
    ...blank(9),
    "..........kkoo..",
    "...okokokokoeko.",
    "..okokokokokokpp",
    ".kokokokokokok..",
    "..k.k.....k.k...",
    "..k.k.....k.k...",
    "................",
  ],
  stride: {
    from: 13,
    rows: [
      ".k...k...k...k..",
      ".k...k...k...k..",
      "................",
    ],
  },
  lid: "k",
},
```

| Field         | Meaning                                                                                                                    |
| ------------- | -------------------------------------------------------------------------------------------------------------------------- |
| `name`        | Shown in the species picker.                                                                                               |
| `description` | One line for the picker. Keep it short and in character.                                                                   |
| `accent`      | The critter's colour in the UI.                                                                                            |
| `palette`     | One character per colour. `.` is transparent; `e` (eye), `w` and `x` (pass/fail eyes) are shared and mustn't be redefined. |
| `body`        | 16 rows of exactly 16 characters, bottom-aligned. It must contain an `e`.                                                  |
| `stride`      | The second frame of the `work` animation: rows from `from` to the bottom, replaced. `from + rows.length` must be 16.       |
| `lid`         | The character a closed eye becomes when blinking, usually the skin around it.                                              |

Every other frame is derived: blinking swaps `e` for `lid`, and passing or
failing swaps it for `w` / `x`. Sprites are drawn as merged horizontal runs of
`<rect>`s and animated with CSS only (`critters.css`), so they stay cheap.

## 3. Test and look at it

Update the expected list in `apps/desktop/ui/src/components/critters/species.test.ts`
(`covers every species the backend knows`). The per-species tests then check
the drawing: every frame is 16×16, every character is in the palette, there
are eyes to blink, the stride fits, and no frame needs more than 90 rects.

```sh
npm test --prefix apps/desktop/ui -- species
```

Then run the app in debug and open `/#sprites` (the dev-only sprite gallery)
to see every state on the current theme. Check a light theme too: light
themes outline sprites, so pale colours still need to read.

## Checklist

- [ ] `Species` variant, `Known` variant and match arm in `crates/core/src/critter/model.rs`
- [ ] `cargo test -p cactus` to regenerate bindings
- [ ] entry in `SPECIES` (`apps/desktop/ui/src/components/critters/species.ts`)
- [ ] expected list in `species.test.ts`
- [ ] checked in `/#sprites` on a dark and a light theme
