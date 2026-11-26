# Add a theme

Themes are pure CSS: a block of custom properties ("tokens") in
`apps/desktop/ui/src/theme/themes.css`, selected by `data-theme` on the root element.
Components only ever use tokens, so a theme never needs component changes.

Four files are involved:

| File                                      | What to do                                               |
| ----------------------------------------- | -------------------------------------------------------- |
| `apps/desktop/ui/src/theme/themes.css`                 | Add a `[data-theme="<id>"]` block                        |
| `apps/desktop/ui/src/theme/themes.ts`                  | List it in `THEMES`                                      |
| `apps/desktop/ui/src/components/critters/critters.css` | Light themes only: add it to the sprite-outline selector |
| `apps/desktop/ui/src/theme/themes.test.ts`             | Nothing; it checks the above agree                       |

## 1. Write the tokens

The base block, `[data-theme] { … }` at the top of `themes.css`, defines every
token with the default (Cactus, dark) values. A theme overrides only what it
changes. Copy the closest existing theme as a starting point; `synthwave` is
a good dark one, `desert-day` a good light one.

```css
/* mesquite: warm brown wood, dark. */
[data-theme="mesquite"] {
  --background: oklch(0.17 0.02 55);
  --foreground: oklch(0.93 0.02 75);
  --card: oklch(0.2 0.025 55);
  --muted-foreground: oklch(0.7 0.04 65);
  --border: oklch(0.9 0.05 70 / 12%);
  --sidebar: oklch(0.15 0.02 55);

  --brand-1: #e0a96d;
  --brand-2: #c0703a;
  --brand-3: #7a9e5f;
  --flower: #f4d35e;
}
```

The groups of tokens:

| Tokens                                                                                                                      | Used for                                                                  |
| --------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------- |
| `--background`, `--foreground`, `--card`, `--popover`, `--muted`, `--accent`, `--border`, `--input`, `--ring`, `--sidebar*` | shadcn surfaces and text                                                  |
| `--brand-1..3`, `--brand-foreground`, `--flower`                                                                            | The logo gradient, primary highlights, the cactus flower                  |
| `--glow-1`, `--glow-2`, `--glow-intensity`                                                                                  | Glows behind the logo and active elements                                 |
| `--success`, `--warning`, `--danger`, `--info`                                                                              | Pass/fail, status codes, alerts                                           |
| `--method-get` … `--method-delete`                                                                                          | HTTP method badges                                                        |
| `--syntax-key`, `--syntax-string`, `--syntax-number`, `--syntax-literal`                                                    | JSON highlighting                                                         |
| `--template-resolved`, `--captured`                                                                                         | `{{variable}}` highlighting                                               |
| `--radius`, `--app-font`                                                                                                    | Corner radius and the UI font                                             |
| `--app-backdrop`, `--app-overlay`                                                                                           | Optional backgrounds behind / effects over the app (gradients, scanlines) |

Rules (`themes.test.ts` enforces them):

- **Only set tokens the base block defines.** A typo like `--sucess` fails
  the test. To add a new token, define it in the base block first, and expose
  it to Tailwind in `apps/desktop/ui/src/index.css` if components should use it as a class.
- **Light themes set `color-scheme: light;`**, because the base is dark.
- **Tokens only.** Anything beyond tokens (custom scrollbars, gloss effects)
  goes in the **Extras** section at the end of `themes.css`, scoped to
  `[data-theme="<id>"]`.
- Prefer `oklch()` for colours, like the rest of the file. Check contrast
  of `--muted-foreground` on `--background`, and of `--danger` against
  `--success`, since pass/fail must stay distinguishable.

## 2. List it

Add it to `THEMES` in `apps/desktop/ui/src/theme/themes.ts`, in the position you want it in
the picker:

```ts
{ id: "mesquite", name: "Mesquite", group: "colorful", scheme: "dark" },
```

`group` is one of `default`, `variation` (close relatives of Cactus),
`colorful` or `fun`. `scheme` toggles the `.dark` class and the sprite outline.

## 3. Light themes: outline the critters

Pale sprites disappear on light backgrounds, so light themes get a 1px
outline. Add the theme to the selector list under `/* Light themes` in
`apps/desktop/ui/src/components/critters/critters.css`. A test checks that every light
theme, and only light themes, is in that list.

## 4. Check it

```sh
npm test --prefix apps/desktop/ui -- themes species && npm run format:check --prefix apps/desktop/ui
```

Then look at it in the app (Settings → Theme): the request editor with a JSON
response, a failing assertion, the git diff view, and the Critters window
(Ctrl+B). The dev-only page `/#sprites` shows every critter sprite on the
current theme.
