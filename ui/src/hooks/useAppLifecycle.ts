import { useEffect } from "react";
import { backendEvents } from "@/api/client";
import { flushPendingSave } from "@/actions/autosave";
import { handleFilesChanged } from "@/actions/fsSync";
import { restoreLastProject } from "@/actions/project";
import { reportErrors } from "@/lib/errors";

const flushOnUnload = () => void flushPendingSave();

/** Restore the last project, save before closing, and follow external file changes. */
export function useAppLifecycle(): void {
  useEffect(() => {
    void reportErrors(restoreLastProject);
    window.addEventListener("beforeunload", flushOnUnload);
    return () => window.removeEventListener("beforeunload", flushOnUnload);
  }, []);

  useEffect(() => {
    const unlisten = backendEvents.fsChanged.listen((event) => void handleFilesChanged(event.payload));
    return () => void unlisten.then((stop) => stop());
  }, []);
}
