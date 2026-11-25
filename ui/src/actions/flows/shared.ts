import { toast } from "sonner";
import { useFlowStore } from "@/stores/flowStore";
import { useProjectStore } from "@/stores/projectStore";

export const currentProject = (): string | null => useProjectStore.getState().project;
export const flowStore = () => useFlowStore.getState();

/** Whether `critter`'s `flow` is the one in the editor. */
export const isOpen = (critter: string, flow: string): boolean => {
  const { open } = flowStore();
  return open?.critter === critter && open.flow === flow;
};

/** The toast for an edit that can't continue until the YAML parses. */
export const fixFirst = (line: number | null) =>
  toast.warning(line === null ? "Fix the YAML first" : `Fix line ${line} first`);
