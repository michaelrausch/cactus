import type { CritterSummary } from "@/api/client";
import { useFlowStore } from "@/stores/flowStore";
import { FlowEditor } from "./FlowEditor";
import { FlowList } from "./FlowList";

/** The critter's flows, or the one being edited. */
export function FlowsTab({ critter }: { critter: CritterSummary }) {
  const editing = useFlowStore((s) => s.open?.critter === critter.id);
  return editing ? <FlowEditor critter={critter} /> : <FlowList critter={critter} />;
}
