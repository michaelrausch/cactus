import { useEffect } from "react";
import { backendEvents } from "@/api/client";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { handleFilesChanged } from "@/actions/fsSync";
import { refreshGitStatus } from "@/actions/git";
import { restoreLastProject } from "@/actions/project";
import { hasUnsavedChanges, saveAll } from "@/actions/saving";
import { refreshVariableNames } from "@/actions/variables";
import { reportErrors } from "@/lib/errors";
import { useEditorStore } from "@/stores/editorStore";
import { useGitStore } from "@/stores/gitStore";
import { useProjectStore } from "@/stores/projectStore";
import { useUiStore } from "@/stores/uiStore";

/** Fallback for a plain browser; the desktop window is guarded by `onCloseRequested` below. */
const warnOnUnload = (event: BeforeUnloadEvent) => {
  if (hasUnsavedChanges()) event.preventDefault();
};

const closeWindow = () => void getCurrentWindow().destroy();

const refreshOnFocus = () => void refreshGitStatus();

/** Ask before a close that would lose unsaved edits. */
function guardClose(event: { preventDefault: () => void }): void {
  if (!hasUnsavedChanges()) return;
  event.preventDefault();
  useUiStore.getState().askConfirm({
    title: "Unsaved changes",
    description: "Some scenarios have unsaved edits, which closing would lose.",
    action: "Save all and close",
    destructive: false,
    onConfirm: async () => {
      if (await saveAll()) closeWindow();
    },
    alternative: { action: "Discard changes", onConfirm: closeWindow },
  });
}

/** Restore the last project, ask before closing with unsaved edits, follow external file changes and keep variable names and git status current. */
export function useAppLifecycle(): void {
  useEffect(() => {
    void reportErrors(restoreLastProject);
    window.addEventListener("beforeunload", warnOnUnload);
    const unlistenClose = getCurrentWindow().onCloseRequested(guardClose);
    return () => {
      window.removeEventListener("beforeunload", warnOnUnload);
      void unlistenClose.then((stop) => stop());
    };
  }, []);

  // The variables in scope depend on the scenario's folder and the active env;
  // git status on the project.
  // Subscribed outside React so the app doesn't re-render on every change.
  useEffect(() => {
    const unsubscribeEditor = useEditorStore.subscribe((now, before) => {
      if (now.path !== before.path) void refreshVariableNames();
    });
    const unsubscribeProject = useProjectStore.subscribe((now, before) => {
      if (now.project !== before.project || now.activeEnv !== before.activeEnv) void refreshVariableNames();
      if (now.project !== before.project) {
        useGitStore.getState().reset();
        void refreshGitStatus();
      }
    });
    return () => {
      unsubscribeEditor();
      unsubscribeProject();
    };
  }, []);

  useEffect(() => {
    const unlisten = backendEvents.fsChanged.listen((event) => void handleFilesChanged(event.payload));
    return () => void unlisten.then((stop) => stop());
  }, []);

  // `.git/` isn't watched, so catch commits and checkouts made elsewhere on return.
  useEffect(() => {
    window.addEventListener("focus", refreshOnFocus);
    return () => window.removeEventListener("focus", refreshOnFocus);
  }, []);
}
