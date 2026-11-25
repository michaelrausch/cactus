/** The themes in `themes.css`, in the order the picker shows them. */

export interface Theme {
  id: string;
  name: string;
  scheme: "dark" | "light";
}

export const THEMES = [
  { id: "cactus", name: "Default dark", scheme: "dark" },
  { id: "vanilla", name: "Default light", scheme: "light" },
  { id: "agave", name: "Agave", scheme: "dark" },
  { id: "prickly-pear", name: "Prickly Pear", scheme: "dark" },
  { id: "sage", name: "Sage", scheme: "dark" },
  { id: "oasis", name: "Oasis", scheme: "dark" },
  { id: "jacaranda", name: "Jacaranda", scheme: "dark" },
  { id: "ember", name: "Ember", scheme: "dark" },
  { id: "aurora", name: "Aurora", scheme: "dark" },
  { id: "stargazer", name: "Stargazer", scheme: "dark" },
  { id: "sunset-mesa", name: "Sunset Mesa", scheme: "dark" },
  { id: "bloom", name: "Bloom", scheme: "dark" },
  { id: "synthwave", name: "Synthwave", scheme: "dark" },
  { id: "ocean-drive", name: "Ocean Drive", scheme: "dark" },
  { id: "phosphor", name: "Phosphor", scheme: "dark" },
  { id: "amber", name: "Amber", scheme: "dark" },
  { id: "blueprint", name: "Blueprint", scheme: "dark" },
  { id: "desert-day", name: "Desert Day", scheme: "light" },
  { id: "salt-flat", name: "Salt Flat", scheme: "light" },
  { id: "adobe", name: "Adobe", scheme: "light" },
  { id: "matcha", name: "Matcha", scheme: "light" },
  { id: "paper", name: "Paper", scheme: "light" },
  { id: "citrus", name: "Citrus", scheme: "light" },
  { id: "bubblegum", name: "Bubblegum", scheme: "light" },
  { id: "aqua", name: "Aqua", scheme: "light" },
  { id: "win95", name: "Windows 95", scheme: "light" },
] as const satisfies readonly Theme[];

export type ThemeEntry = (typeof THEMES)[number];
export type ThemeId = ThemeEntry["id"];

export const DEFAULT_THEME: ThemeId = "cactus";

/** The main dark and light themes, shown apart from (and above) the rest in the picker. */
export const MAIN_THEME_IDS: readonly ThemeId[] = ["cactus", "vanilla"];

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
