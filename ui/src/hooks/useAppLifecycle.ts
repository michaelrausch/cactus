import { useEffect } from "react";
import { backendEvents } from "@/api/client";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { handleFilesChanged } from "@/actions/fsSync";
import { scheduledRunFinished, showAlert, showFlowNotification } from "@/actions/alerts";
import { refreshContext } from "@/actions/critters";
import { flowAskClosed, flowAsked, flowDisplayed, flowStepFinished, flowStepStarted } from "@/actions/flows";
import { refreshGitStatus } from "@/actions/git";
import { restoreLastProject } from "@/actions/project";
import { historyChanged } from "@/actions/request";
import { hasUnsavedChanges, saveAll } from "@/actions/saving";
import { checkForUpdate } from "@/actions/update";
import { refreshVariableNames } from "@/actions/variables";
import { reportErrors } from "@/lib/errors";
import { useCritterStore } from "@/stores/critterStore";
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
    description: "Some requests have unsaved edits, which closing would lose.",
    action: "Save all and close",
    onConfirm: async () => {
      if (await saveAll()) closeWindow();
    },
    alternative: { action: "Discard changes", onConfirm: closeWindow, destructive: true },
  });
}

/** Restore the last project, check for a newer version, ask before closing with unsaved edits, follow external file changes and keep variable names and git status current. */
export function useAppLifecycle(): void {
  useEffect(() => {
    void reportErrors(restoreLastProject);
    void checkForUpdate();
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

  // Writes to `.cactus/` aren't watched, so the backend says when a critter's
  // context or a scheduled run changed.
  useEffect(() => {
    const unlisteners = [
      backendEvents.contextChanged.listen((event) => {
        const { critter } = event.payload;
        void refreshContext(critter);
        if (critter === useCritterStore.getState().activeId) void refreshVariableNames();
      }),
      backendEvents.historyChanged.listen(({ payload }) => void historyChanged(payload.path)),
      backendEvents.scheduledRunFinished.listen(() => scheduledRunFinished()),
      backendEvents.monitorAlert.listen(({ payload }) => showAlert(payload)),
      backendEvents.flowStepStarted.listen(({ payload }) =>
        flowStepStarted(payload.critter, payload.flow, payload.start),
      ),
      backendEvents.flowProgress.listen(({ payload }) => flowStepFinished(payload.critter, payload.flow, payload.step)),
      // Tool steps: a question for the person running the flow, and notifications.
      backendEvents.flowAsked.listen(({ payload }) => flowAsked(payload)),
      backendEvents.flowAskClosed.listen(({ payload }) => flowAskClosed(payload.id)),
      backendEvents.flowDisplayed.listen(({ payload }) => flowDisplayed(payload)),
      backendEvents.flowNotified.listen(({ payload }) => void showFlowNotification(payload.title, payload.body)),
    ];
    return () => unlisteners.forEach((unlisten) => void unlisten.then((stop) => stop()));
  }, []);

  // `.git/` isn't watched, so catch commits and checkouts made elsewhere on return.
  useEffect(() => {
    window.addEventListener("focus", refreshOnFocus);
    return () => window.removeEventListener("focus", refreshOnFocus);
  }, []);

  // Idle critters stop moving while the window is in the background (critters.css).
  useEffect(() => {
    const root = document.documentElement;
    const blurred = () => root.toggleAttribute("data-window-blurred", true);
    const focused = () => root.toggleAttribute("data-window-blurred", false);
    window.addEventListener("blur", blurred);
    window.addEventListener("focus", focused);
    return () => {
      window.removeEventListener("blur", blurred);
      window.removeEventListener("focus", focused);
      focused();
    };
  }, []);
}
