import { useEffect } from "react";
import { openChanges } from "@/actions/git";
import { openProject } from "@/actions/project";
import { sendCurrentScenario } from "@/actions/request";
import { saveScenario } from "@/actions/saving";
import { createScenarioIn, duplicateScenario } from "@/actions/scenario";
import { shortcutForKey, type ShortcutId } from "@/app/shortcuts";
import { useEditorStore } from "@/stores/editorStore";
import { useUiStore } from "@/stores/uiStore";

const SHORTCUT_ACTIONS: Record<ShortcutId, () => void> = {
  save: () => {
    const { path } = useEditorStore.getState();
    if (path) void saveScenario(path);
  },
  send: () => void sendCurrentScenario(),
  newScenario: () => createScenarioIn(""),
  openProject: () => void openProject(),
  switcher: () => useUiStore.getState().toggleSwitcher(),
  duplicate: () => {
    const { path } = useEditorStore.getState();
    if (path) void duplicateScenario(path);
  },
  variables: () => useUiStore.getState().setEnvDialogOpen(true),
  changes: openChanges,
  toggleResponse: () => useUiStore.getState().toggleResponse(),
  settings: () => useUiStore.getState().setSettingsOpen(true),
};

/** Radix marks every open dialog, the switcher included, with `role="dialog"`. */
const dialogOpen = (): boolean => document.querySelector('[role="dialog"]') !== null;

function onKeyDown(event: KeyboardEvent): void {
  if (!(event.ctrlKey || event.metaKey)) return;
  const shortcut = shortcutForKey(event.key);
  if (!shortcut) return;
  // Never the webview's own Ctrl+S, Ctrl+P and so on.
  event.preventDefault();
  // A shortcut acts on the window behind a dialog, so it waits until the
  // dialog closes. Dialogs handle their own keys (Ctrl+S in Variables,
  // Ctrl+Enter in Changes); Ctrl+P still closes the switcher it opened.
  if (dialogOpen() && !(shortcut === "switcher" && useUiStore.getState().switcherOpen)) return;
  SHORTCUT_ACTIONS[shortcut]();
}

export function useKeyboardShortcuts(): void {
  useEffect(() => {
    // Capture phase, so a focused field or dialog can't swallow a shortcut first.
    window.addEventListener("keydown", onKeyDown, true);
    return () => window.removeEventListener("keydown", onKeyDown, true);
  }, []);
}
