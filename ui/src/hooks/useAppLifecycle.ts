import { useEffect } from "react";
import { backendEvents } from "@/api/client";
import { flushPendingSave } from "@/actions/autosave";
import { handleFilesChanged } from "@/actions/fsSync";
import { restoreLastProject } from "@/actions/project";
import { refreshVariableNames } from "@/actions/variables";
import { reportErrors } from "@/lib/errors";
import { useEditorStore } from "@/stores/editorStore";
import { useProjectStore } from "@/stores/projectStore";

const flushOnUnload = () => void flushPendingSave();

/** Restore the last project, save before closing, follow external file changes and keep variable names current. */
export function useAppLifecycle(): void {
  useEffect(() => {
    void reportErrors(restoreLastProject);
    window.addEventListener("beforeunload", flushOnUnload);
    return () => window.removeEventListener("beforeunload", flushOnUnload);
  }, []);

  // The variables in scope depend on the scenario's folder and the active env.
  // Subscribed outside React so the app doesn't re-render on every change.
  useEffect(() => {
    const unsubscribeEditor = useEditorStore.subscribe((now, before) => {
      if (now.path !== before.path) void refreshVariableNames();
    });
    const unsubscribeProject = useProjectStore.subscribe((now, before) => {
      if (now.project !== before.project || now.activeEnv !== before.activeEnv) void refreshVariableNames();
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
}
