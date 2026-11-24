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
  // Ctrl+Enter commits while the Changes dialog is open.
  send: () => {
    if (!useUiStore.getState().gitDialogOpen) void sendCurrentScenario();
  },
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

function onKeyDown(event: KeyboardEvent): void {
  if (!(event.ctrlKey || event.metaKey)) return;
  const shortcut = shortcutForKey(event.key);
  if (!shortcut) return;
  event.preventDefault();
  SHORTCUT_ACTIONS[shortcut]();
}

export function useKeyboardShortcuts(): void {
  useEffect(() => {
    // Capture phase, so a focused field or dialog can't swallow a shortcut first.
    window.addEventListener("keydown", onKeyDown, true);
    return () => window.removeEventListener("keydown", onKeyDown, true);
  }, []);
}
