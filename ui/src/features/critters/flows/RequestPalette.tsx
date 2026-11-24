import { useState, type DragEvent } from "react";
import { KeyRound, Plus, Search } from "lucide-react";
import { editFlow } from "@/actions/flows";
import type { ScenarioEntry } from "@/api/client";
import { MethodBadge } from "@/components/MethodBadge";
import { insertStep, removeStep } from "@/domain/flowEdit";
import { isAuthRequest } from "@/domain/scenario";
import { cn } from "@/lib/utils";
import { useFlowStore } from "@/stores/flowStore";
import { useProjectStore } from "@/stores/projectStore";
import { REQUEST_MIME, STEP_MIME } from "./dragTypes";

const matches = (entry: ScenarioEntry, query: string): boolean => {
  const q = query.trim().toLowerCase();
  return !q || [entry.title, entry.relPath, entry.method].some((text) => text.toLowerCase().includes(q));
};

/** Login requests first, otherwise in tree order. */
const authFirst = (entries: ScenarioEntry[]): ScenarioEntry[] =>
  entries.toSorted((a, b) => Number(isAuthRequest(b.tags)) - Number(isAuthRequest(a.tags)));

/** Add a request after the selected step, or at the end. */
function addStep(path: string) {
  const { selected, model } = useFlowStore.getState();
  const at = selected === null ? (model?.steps.length ?? 0) : selected + 1;
  editFlow((flow) => insertStep(flow, at, path));
  useFlowStore.getState().select(at);
}

/** The project's requests, to drag into the flow (or add with +). Dropping a step here removes it. */
export function RequestPalette() {
  const entries = useProjectStore((s) => s.entries);
  const [query, setQuery] = useState("");
  const [removing, setRemoving] = useState(false);
  const shown = authFirst(entries.filter((entry) => matches(entry, query)));

  const onDragOver = (event: DragEvent) => {
    if (!event.dataTransfer.types.includes(STEP_MIME)) return;
    event.preventDefault();
    event.dataTransfer.dropEffect = "move";
    setRemoving(true);
  };
  const onDrop = (event: DragEvent) => {
    setRemoving(false);
    const index = Number(event.dataTransfer.getData(STEP_MIME));
    if (!event.dataTransfer.types.includes(STEP_MIME) || Number.isNaN(index)) return;
    event.preventDefault();
    editFlow((flow) => removeStep(flow, index));
  };

  return (
    <div
      onDragOver={onDragOver}
      onDragLeave={() => setRemoving(false)}
      onDrop={onDrop}
      className={cn("flex h-full flex-col", removing && "bg-danger/10")}
    >
      <div className="flex items-center gap-2 border-b px-2.5 py-1.5 text-muted-foreground">
        <Search className="size-3.5 shrink-0" />
        <input
          value={query}
          onChange={(event) => setQuery(event.target.value)}
          placeholder="Requests"
          aria-label="Search requests"
          className="min-w-0 flex-1 bg-transparent text-sm text-foreground outline-none placeholder:text-muted-foreground"
        />
      </div>
      {removing && <p className="px-3 py-2 text-xs text-danger">Drop to remove the step</p>}
      <ul aria-label="Requests" className="min-h-0 flex-1 overflow-y-auto p-1">
        {shown.map((entry) => (
          <li
            key={entry.relPath}
            draggable
            onDragStart={(event) => {
              // WebKit only starts a drag that carries data.
              event.dataTransfer.setData(REQUEST_MIME, entry.relPath);
              event.dataTransfer.setData("text/plain", entry.relPath);
              event.dataTransfer.effectAllowed = "copy";
            }}
            title={entry.relPath}
            className="group flex h-8 cursor-grab items-center gap-2 rounded-md px-1.5 text-sm hover:bg-accent/50 active:cursor-grabbing"
          >
            <MethodBadge method={entry.method} className="w-10 shrink-0 text-2xs" />
            <span className="min-w-0 flex-1 truncate">{entry.title}</span>
            {isAuthRequest(entry.tags) && (
              <KeyRound className="size-3 shrink-0 text-muted-foreground" aria-label="Logs in" />
            )}
            <button
              type="button"
              aria-label={`Add ${entry.title} as a step`}
              onClick={() => addStep(entry.relPath)}
              className="grid size-6 shrink-0 place-items-center rounded-md text-muted-foreground opacity-0 focus-ring group-hover:opacity-100 hover:bg-accent hover:text-foreground focus-visible:opacity-100"
            >
              <Plus className="size-3.5" />
            </button>
          </li>
        ))}
      </ul>
    </div>
  );
}
