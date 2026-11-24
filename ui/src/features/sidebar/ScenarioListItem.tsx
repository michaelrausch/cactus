import type { MouseEvent, ReactNode } from "react";
import { Copy, Pencil, Trash2 } from "lucide-react";
import type { ScenarioEntry } from "@/api/client";
import { deleteScenario, duplicateScenario, renameScenario, selectScenario } from "@/actions/scenario";
import { MethodBadge } from "@/components/MethodBadge";
import { StatusCode } from "@/components/StatusCode";
import { cn } from "@/lib/utils";

interface ScenarioListItemProps {
  entry: ScenarioEntry;
  selected: boolean;
  lastStatus: number | undefined;
}

function RowAction({ label, onClick, children }: { label: string; onClick: () => void; children: ReactNode }) {
  const handleClick = (event: MouseEvent) => {
    event.stopPropagation();
    onClick();
  };
  return (
    <button type="button" title={label} aria-label={label} onClick={handleClick}>
      {children}
    </button>
  );
}

export function ScenarioListItem({ entry, selected, lastStatus }: ScenarioListItemProps) {
  const path = entry.relPath;
  return (
    <div
      onClick={() => void selectScenario(path)}
      title={path}
      className={cn(
        "group relative flex h-8 cursor-default items-center gap-2 rounded-lg px-2 transition-colors hover:bg-sidebar-accent/60",
        selected && "bg-sidebar-accent hover:bg-sidebar-accent",
      )}
    >
      {selected && <span className="absolute top-2 bottom-2 left-0 w-0.5 rounded-full bg-brand" />}
      <MethodBadge method={entry.method} className="w-11 shrink-0 text-[10px]" />
      <span
        className={cn(
          "min-w-0 flex-1 truncate text-[13px]",
          selected ? "text-foreground" : "text-sidebar-foreground/90",
        )}
      >
        {entry.title}
      </span>
      {lastStatus !== undefined && <StatusCode status={lastStatus} className="text-[11px] group-hover:hidden" />}
      <span className="hidden items-center gap-1.5 text-muted-foreground group-hover:flex">
        <RowAction label="Duplicate" onClick={() => void duplicateScenario(path)}>
          <Copy className="size-3.5 hover:text-foreground" />
        </RowAction>
        <RowAction label="Rename file" onClick={() => renameScenario(path)}>
          <Pencil className="size-3.5 hover:text-foreground" />
        </RowAction>
        <RowAction label="Delete" onClick={() => deleteScenario(path)}>
          <Trash2 className="size-3.5 hover:text-rose-400" />
        </RowAction>
      </span>
    </div>
  );
}
