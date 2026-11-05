/** The themes in `themes.css`, in the order the picker shows them. */

export type ThemeGroup = "default" | "variation" | "fun";

export interface Theme {
  id: string;
  name: string;
  description: string;
  group: ThemeGroup;
  scheme: "dark" | "light";
}

export const THEMES = [
  {
    id: "cactus",
    name: "Cactus",
    description: "Near-black, a green gradient, a pink flower.",
    group: "default",
    scheme: "dark",
  },
  { id: "agave", name: "Agave", description: "Cool blue-teal.", group: "variation", scheme: "dark" },
  {
    id: "prickly-pear",
    name: "Prickly Pear",
    description: "Plum and magenta, a yellow flower.",
    group: "variation",
    scheme: "dark",
  },
  { id: "sage", name: "Sage", description: "Muted grey-green. Calm.", group: "variation", scheme: "dark" },
  { id: "desert-day", name: "Desert Day", description: "Light, warm sand.", group: "variation", scheme: "light" },
  { id: "synthwave", name: "Synthwave", description: "Neon pink and cyan over a grid.", group: "fun", scheme: "dark" },
  { id: "phosphor", name: "Phosphor", description: "A green CRT terminal.", group: "fun", scheme: "dark" },
  {
    id: "sunset-mesa",
    name: "Sunset Mesa",
    description: "Dusk purple and a low orange sun.",
    group: "fun",
    scheme: "dark",
  },
  { id: "bloom", name: "Bloom", description: "Pastels, and a lot of flower.", group: "fun", scheme: "dark" },
  { id: "aqua", name: "Aqua", description: "Pinstripes and lickable blue gel.", group: "fun", scheme: "light" },
  { id: "win95", name: "Windows 95", description: "Silver, navy and bevelled buttons.", group: "fun", scheme: "light" },
] as const satisfies readonly Theme[];

export type ThemeEntry = (typeof THEMES)[number];
export type ThemeId = ThemeEntry["id"];

export const DEFAULT_THEME: ThemeId = "cactus";

export const THEME_GROUPS: readonly { group: ThemeGroup; label: string }[] = [
  { group: "default", label: "Default" },
  { group: "variation", label: "Variations" },
  { group: "fun", label: "Fun" },
];

const isThemeId = (id: string): id is ThemeId => THEMES.some((theme) => theme.id === id);

/** The theme for `id`, or the default if it is unknown (e.g. removed since it was saved). */
export function resolveTheme(id: string | null | undefined): ThemeEntry {
  const found = id && isThemeId(id) ? id : DEFAULT_THEME;
  return THEMES.find((theme) => theme.id === found) ?? THEMES[0];
}

/** Show `theme` on the whole app: the `data-theme` the CSS keys on, plus `.dark` for shadcn's `dark:` variants. */
export function applyTheme(theme: Theme, root: HTMLElement = document.documentElement): void {
  root.dataset.theme = theme.id;
  root.classList.toggle("dark", theme.scheme === "dark");
}
