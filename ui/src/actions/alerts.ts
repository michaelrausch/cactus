import { sendNotification } from "@tauri-apps/plugin-notification";
import { toast } from "sonner";
import type { MonitorAlert } from "@/api/client";
import { alertMessage } from "@/domain/alertMessage";
import { warnQuietly } from "@/lib/errors";
import { useCritterStore } from "@/stores/critterStore";
import { useMonitorStore } from "@/stores/monitorStore";
import { switchToFlow } from "./flows";
import { refreshSchedules } from "./monitors";

/** How long an alerted critter's sprite shows `fail`. */
const FLASH_MS = 1200;

let flashTimer: ReturnType<typeof setTimeout> | undefined;

const names = (critter: string, flow: string) => {
  const summary = useCritterStore.getState().critters.find((c) => c.id === critter);
  return { critter: summary?.name ?? critter, flow: summary?.flows.find((f) => f.id === flow)?.name ?? flow };
};

/** A scheduled run finished: refresh the schedule states, which hold each flow's last run. */
export function scheduledRunFinished(): void {
  void refreshSchedules();
}

/**
 * Show an alert: a toast if the window has focus, otherwise an OS
 * notification (when allowed) and an "N alerts" banner for when it's back.
 * Never both.
 */
export function showAlert(alert: MonitorAlert): void {
  const { critter, flow } = names(alert.critter, alert.flow);
  const { title, description, tone } = alertMessage(alert.event, critter, flow);
  if (alert.event.kind === "failing") flash(alert.critter);
  if (document.hasFocus()) {
    toast[tone](title, { description, action: { label: "View", onClick: () => void viewAlert(alert) } });
    return;
  }
  useMonitorStore.getState().alertMissed(alert);
  if (useMonitorStore.getState().notifications === "granted") {
    void warnQuietly(async () => sendNotification({ title, body: description }), "Couldn't show a notification");
  }
}

/** The alerted critter's sprite plays `fail` once. */
function flash(critter: string): void {
  clearTimeout(flashTimer);
  useMonitorStore.getState().flashed(critter);
  flashTimer = setTimeout(() => useMonitorStore.getState().flashed(null), FLASH_MS);
}

/** Show the alert's flow. */
export async function viewAlert(alert: MonitorAlert): Promise<void> {
  useMonitorStore.getState().alertsSeen();
  await switchToFlow(alert.critter, alert.flow);
}
