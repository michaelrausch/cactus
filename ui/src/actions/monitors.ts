import { isPermissionGranted, requestPermission } from "@tauri-apps/plugin-notification";
import { backend } from "@/api/client";
import { reportErrors, warnQuietly } from "@/lib/errors";
import { useMonitorStore } from "@/stores/monitorStore";
import { useProjectStore } from "@/stores/projectStore";

const currentProject = (): string | null => useProjectStore.getState().project;

/** Reload the scheduled flows and their last runs. Only a hint if it fails. */
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

/**
 * Ask once, when a flow gets a schedule, whether alerts may become OS
 * notifications. If not, alerts stay in the app; the schedule control says so.
 */
export async function ensureNotificationPermission(): Promise<void> {
  if (useMonitorStore.getState().notifications !== "unknown") return;
  const granted = await warnQuietly(
    async () => (await isPermissionGranted()) || (await requestPermission()) === "granted",
    "Couldn't ask to show notifications",
  );
  useMonitorStore.getState().notificationsChecked(granted ? "granted" : "denied");
}

/** Run a scheduled flow once now, recorded and alerted on like a timed run. */
export async function runScheduledNow(critter: string, flow: string): Promise<void> {
  const project = currentProject();
  if (!project) return;
  // The record arrives as a `ScheduledRunFinished` event, like a timed run's.
  await reportErrors(() => backend.runScheduledNow(project, critter, flow), "Run failed");
}
