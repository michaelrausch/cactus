/** The themes in `themes.css`, then the Pro edition's, in the order the picker shows them. */
import { PRO_THEMES } from "@edition/themes";

export interface Theme {
  id: string;
  name: string;
  scheme: "dark" | "light";
}

/** Every edition's themes: the main dark and light ones. */
const BASE_THEMES = [
  { id: "cactus", name: "Default dark", scheme: "dark" },
  { id: "vanilla", name: "Default light", scheme: "light" },
] as const satisfies readonly Theme[];

export const THEMES: readonly [Theme, ...Theme[]] = [...BASE_THEMES, ...PRO_THEMES];

export type ThemeEntry = Theme;
export type ThemeId = string;

export const DEFAULT_THEME: ThemeId = "cactus";

/** The main dark and light themes, shown apart from (and above) the rest in the picker. */
export const MAIN_THEME_IDS: readonly ThemeId[] = BASE_THEMES.map((theme) => theme.id);

/** The theme for `id`, or the default if it is unknown (e.g. removed since it was saved, or a Pro theme in the free edition). */
export function resolveTheme(id: string | null | undefined): ThemeEntry {
  return THEMES.find((theme) => theme.id === id) ?? THEMES.find((theme) => theme.id === DEFAULT_THEME) ?? THEMES[0];
}

/** Show `theme` on the whole app: the `data-theme` the CSS keys on, plus `.dark` for shadcn's `dark:` variants. */
export function applyTheme(theme: Theme, root: HTMLElement = document.documentElement): void {
  root.dataset.theme = theme.id;
  root.classList.toggle("dark", theme.scheme === "dark");
}
