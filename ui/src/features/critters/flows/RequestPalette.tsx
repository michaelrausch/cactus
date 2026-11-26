import { useState, type DragEvent, type ReactNode } from "react";
import { KeyRound, Plus, Search } from "lucide-react";
import { editFlow } from "@/actions/flows";
import type { Flow, ScenarioEntry } from "@/api/client";
import { MethodBadge } from "@/components/MethodBadge";
import { insertStep, insertTool, removeStep } from "@/domain/flowEdit";
import { isAuthRequest } from "@/domain/scenario";
import { cn } from "@/lib/utils";
import { useFlowStore } from "@/stores/flowStore";
import { useProjectStore } from "@/stores/projectStore";
import { REQUEST_MIME, STEP_MIME, TOOL_MIME } from "./dragTypes";
import { TOOL_SECTIONS, TOOLS } from "./tools";

const matches = (query: string, ...texts: string[]): boolean => {
  const q = query.trim().toLowerCase();
  return !q || texts.some((text) => text.toLowerCase().includes(q));
};

/** Login requests first, otherwise in tree order. */
const authFirst = (entries: ScenarioEntry[]): ScenarioEntry[] =>
  entries.toSorted((a, b) => Number(isAuthRequest(b.tags)) - Number(isAuthRequest(a.tags)));

/** Add a step after the selected one, or at the end. */
function addStep(insert: (flow: Flow, at: number) => Flow) {
  const { selected, model } = useFlowStore.getState();
  const at = selected === null ? (model?.steps.length ?? 0) : selected + 1;
  editFlow((flow) => insert(flow, at));
  useFlowStore.getState().select(at);
}

/** One draggable entry, with + to add it after the selected step. */
function PaletteRow({
  mime,
  data,
  title,
  name,
  onAdd,
  children,
}: {
  mime: string;
  data: string;
  title: string;
  name: string;
  onAdd: () => void;
  children: ReactNode;
}) {
  return (
    <li
      draggable
      onDragStart={(event) => {
        // WebKit only starts a drag that carries data.
        event.dataTransfer.setData(mime, data);
        event.dataTransfer.setData("text/plain", data);
        event.dataTransfer.effectAllowed = "copy";
      }}
      title={title}
      className="group flex h-8 cursor-grab items-center gap-2 rounded-md px-1.5 text-sm hover:bg-accent/50 active:cursor-grabbing"
    >
      {children}
      <button
        type="button"
        aria-label={`Add ${name} as a step`}
        onClick={onAdd}
        className="grid size-6 shrink-0 place-items-center rounded-md text-muted-foreground opacity-0 focus-ring group-hover:opacity-100 hover:bg-accent hover:text-foreground focus-visible:opacity-100"
      >
        <Plus className="size-3.5" />
      </button>
    </li>
  );
}

/** The project's requests, then the tools, to drag into the flow (or add with +). Dropping a step here removes it. */
export function RequestPalette() {
  const entries = useProjectStore((s) => s.entries);
  const [query, setQuery] = useState("");
  const [removing, setRemoving] = useState(false);
  const requests = authFirst(entries.filter((entry) => matches(query, entry.title, entry.relPath, entry.method)));
  const sections = TOOL_SECTIONS.map(({ label, tools }) => ({
    label,
    tools: tools.filter((id) => matches(query, TOOLS[id].label, id, label)),
  })).filter((section) => section.tools.length > 0);

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
          placeholder="Search"
          aria-label="Search requests and tools"
          className="min-w-0 flex-1 bg-transparent text-sm text-foreground outline-none placeholder:text-muted-foreground"
        />
      </div>
      {removing && <p className="px-3 py-2 text-xs text-danger">Drop to remove the step</p>}
      <div className="min-h-0 flex-1 overflow-y-auto p-1">
        <ul aria-label="Requests">
          {requests.map((entry) => (
            <PaletteRow
              key={entry.relPath}
              mime={REQUEST_MIME}
              data={entry.relPath}
              title={entry.relPath}
              name={entry.title}
              onAdd={() => addStep((flow, at) => insertStep(flow, at, entry.relPath))}
            >
              <MethodBadge method={entry.method} className="w-10 shrink-0 text-2xs" />
              <span className="min-w-0 flex-1 truncate">{entry.title}</span>
              {isAuthRequest(entry.tags) && (
                <KeyRound className="size-3 shrink-0 text-muted-foreground" aria-label="Logs in" />
              )}
            </PaletteRow>
          ))}
        </ul>
        {sections.map((section) => (
          <section key={section.label}>
            <h4 className="px-1.5 pt-3 pb-1 text-xs font-semibold tracking-wide text-muted-foreground uppercase">
              {section.label}
            </h4>
            <ul aria-label={section.label}>
              {section.tools.map((id) => {
                const { label, icon: Icon, create } = TOOLS[id];
                return (
                  <PaletteRow
                    key={id}
                    mime={TOOL_MIME}
                    data={id}
                    title={label}
                    name={label}
                    onAdd={() => addStep((flow, at) => insertTool(flow, at, create()))}
                  >
                    <span className="grid w-10 shrink-0 place-items-center text-muted-foreground">
                      <Icon className="size-3.5" />
                    </span>
                    <span className="min-w-0 flex-1 truncate">{label}</span>
                  </PaletteRow>
                );
              })}
            </ul>
          </section>
        ))}
      </div>
    </div>
  );
}
