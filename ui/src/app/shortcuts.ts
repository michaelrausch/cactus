/** Ctrl/Cmd keyboard shortcuts. Labels in the UI are rendered from this table. */

interface ShortcutDefinition {
  /** `KeyboardEvent.key` to match while Ctrl or Cmd is held. */
  key: string;
  description: string;
}

export const SHORTCUTS = {
  save: { key: "s", description: "save" },
  send: { key: "Enter", description: "send" },
  newScenario: { key: "n", description: "new scenario" },
  openProject: { key: "o", description: "open folder" },
  switcher: { key: "p", description: "jump to scenario" },
  duplicate: { key: "d", description: "duplicate" },
  variables: { key: "e", description: "variables" },
  changes: { key: "g", description: "git changes" },
  toggleResponse: { key: "j", description: "toggle response" },
  settings: { key: ",", description: "settings" },
} as const satisfies Record<string, ShortcutDefinition>;

export type ShortcutId = keyof typeof SHORTCUTS;

const SHORTCUT_IDS = Object.keys(SHORTCUTS) as ShortcutId[];

/** Shown on the welcome screen. */
export const WELCOME_SHORTCUTS: readonly ShortcutId[] = ["send", "switcher", "variables", "toggleResponse"];

const displayKey = (key: string): string => (key.length === 1 ? key.toUpperCase() : key);

/** `["Ctrl", "N"]` */
export const shortcutKeys = (id: ShortcutId): string[] => ["Ctrl", displayKey(SHORTCUTS[id].key)];

/** `"Ctrl+N"`, for tooltips. */
export const shortcutLabel = (id: ShortcutId): string => shortcutKeys(id).join("+");

/** `"New scenario (Ctrl+N)"` */
export const withShortcut = (title: string, id: ShortcutId): string => `${title} (${shortcutLabel(id)})`;

export const shortcutForKey = (key: string): ShortcutId | undefined =>
  SHORTCUT_IDS.find((id) => SHORTCUTS[id].key.toLowerCase() === key.toLowerCase());
