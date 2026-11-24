import { backend } from "@/api/client";
import { reportErrors, warnQuietly } from "@/lib/errors";
import { useMonitorStore } from "@/stores/monitorStore";
import { useProjectStore } from "@/stores/projectStore";

const currentProject = (): string | null => useProjectStore.getState().project;

/** Reload the enabled schedules and their last runs. Only a hint if it fails. */
export async function refreshSchedules(): Promise<void> {
  const project = currentProject();
  if (!project) return;
  const states = await warnQuietly(() => backend.scheduleStates(project), "Couldn't load schedules");
  if (states && currentProject() === project) useMonitorStore.getState().schedulesLoaded(states);
}

/** Pause or resume every scheduled flow on this machine. */
export async function setSchedulesPaused(paused: boolean): Promise<void> {
  await reportErrors(async () => {
    await backend.setSchedulesPaused(paused);
    useMonitorStore.getState().pausedChanged(paused);
  });
}
