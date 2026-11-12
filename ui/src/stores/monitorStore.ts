import { create } from "zustand";
import type { MonitorAlert, ScheduleState, ScheduleStates } from "@/api/client";

/** Whether alerts can go out as OS notifications. */
export type NotificationPermission = "unknown" | "granted" | "denied";

interface MonitorState {
  /** Schedules enabled on this machine, with their last run. */
  schedules: ScheduleState[];
  /** Whether every schedule is paused. */
  paused: boolean;
  /** Alerts raised while the window was in the background, newest last. */
  unseen: MonitorAlert[];
  notifications: NotificationPermission;
  /** A critter whose sprite plays `fail` once, after an alert. */
  flashing: string | null;

  reset: () => void;
  schedulesLoaded: (states: ScheduleStates) => void;
  pausedChanged: (paused: boolean) => void;
  alertMissed: (alert: MonitorAlert) => void;
  alertsSeen: () => void;
  notificationsChecked: (permission: NotificationPermission) => void;
  flashed: (critter: string | null) => void;
}

export const useMonitorStore = create<MonitorState>()((set) => ({
  schedules: [],
  paused: false,
  unseen: [],
  notifications: "unknown",
  flashing: null,

  reset: () => set({ schedules: [], paused: false, unseen: [], flashing: null }),
  schedulesLoaded: ({ schedules, paused }) => set({ schedules, paused }),
  pausedChanged: (paused) => set({ paused }),
  alertMissed: (alert) => set((state) => ({ unseen: [...state.unseen, alert] })),
  alertsSeen: () => set({ unseen: [] }),
  notificationsChecked: (notifications) => set({ notifications }),
  flashed: (flashing) => set({ flashing }),
}));

/** Schedules whose last run failed. */
export const selectFailingSchedules = (state: MonitorState): ScheduleState[] =>
  state.schedules.filter((schedule) => schedule.last && !schedule.last.ok);
