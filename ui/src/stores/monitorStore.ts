import { create } from "zustand";
import type { ScheduleState, ScheduleStates } from "@/api/client";

interface MonitorState {
  /** Schedules enabled on this machine, with their last run. */
  schedules: ScheduleState[];
  /** Whether every schedule is paused. */
  paused: boolean;

  reset: () => void;
  schedulesLoaded: (states: ScheduleStates) => void;
  pausedChanged: (paused: boolean) => void;
}

export const useMonitorStore = create<MonitorState>()((set) => ({
  schedules: [],
  paused: false,

  reset: () => set({ schedules: [], paused: false }),
  schedulesLoaded: ({ schedules, paused }) => set({ schedules, paused }),
  pausedChanged: (paused) => set({ paused }),
}));

/** Schedules whose last run failed. */
export const selectFailingSchedules = (state: MonitorState): ScheduleState[] =>
  state.schedules.filter((schedule) => schedule.last && !schedule.last.ok);
