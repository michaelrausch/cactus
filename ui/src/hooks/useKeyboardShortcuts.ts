import { useEffect } from "react";
import { toggleCrittersView } from "@/actions/critters";
import { runOpenFlow, saveOpenFlow } from "@/actions/flows";
import { openChanges } from "@/actions/git";
import { openProject } from "@/actions/project";
import { sendCurrentScenario } from "@/actions/request";
import { saveScenario } from "@/actions/saving";
import { createScenarioIn, duplicateScenario } from "@/actions/scenario";
import { shortcutForKey, type ShortcutId } from "@/app/shortcuts";
import { useEditorStore } from "@/stores/editorStore";
import { hasResponseContent, useResponseStore } from "@/stores/responseStore";
import { useUiStore } from "@/stores/uiStore";

/** Marks the flow editor, where Ctrl+S saves the flow and Ctrl+Enter runs it. */
export const FLOW_EDITOR_ATTRIBUTE = "data-flow-editor";

const inFlowEditor = (): boolean => Boolean(document.activeElement?.closest(`[${FLOW_EDITOR_ATTRIBUTE}]`));

const SHORTCUT_ACTIONS: Record<ShortcutId, () => void> = {
  save: () => {
    if (inFlowEditor()) {
      void saveOpenFlow();
      return;
    }
    const { path } = useEditorStore.getState();
    if (path) void saveScenario(path);
  },
  send: () => {
    if (inFlowEditor()) runOpenFlow();
    else void sendCurrentScenario();
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
  toggleResponse: () => {
    if (hasResponseContent(useResponseStore.getState())) useUiStore.getState().toggleResponse();
  },
  settings: () => useUiStore.getState().setSettingsOpen(true),
  critters: toggleCrittersView,
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
