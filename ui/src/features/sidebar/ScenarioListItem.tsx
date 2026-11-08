import { memo, type ReactNode } from "react";
import { Copy, Pencil, Trash2 } from "lucide-react";
import type { ChangeKind, ScenarioEntry } from "@/api/client";
import { deleteScenario, duplicateScenario, renameScenario, selectScenario } from "@/actions/scenario";
import { GitMarker } from "@/components/GitMarker";
import { MethodBadge } from "@/components/MethodBadge";
import { StatusCode } from "@/components/StatusCode";
import { cn } from "@/lib/utils";
import { isDirty, useEditorStore } from "@/stores/editorStore";

interface ScenarioListItemProps {
  entry: ScenarioEntry;
  selected: boolean;
  lastStatus: number | undefined;
  /** The file's git change, if the project is in a repository. */
  change: ChangeKind | undefined;
}

function RowAction({ label, onClick, children }: { label: string; onClick: () => void; children: ReactNode }) {
  return (
    <button
      type="button"
      title={label}
      aria-label={label}
      onClick={onClick}
      className="grid size-6 place-items-center rounded-md focus-ring hover:bg-sidebar-accent hover:text-foreground"
    >
      {children}
    </button>
  );
}

/** Memoised: only the rows whose selection, status, git change or dirty flag change re-render. */
export const ScenarioListItem = memo(function ScenarioListItem({
  entry,
  selected,
  lastStatus,
  change,
}: ScenarioListItemProps) {
  const path = entry.relPath;
  const dirty = useEditorStore((s) => isDirty(s.buffers[path]));
  return (
    <div
      className={cn(
        "group relative flex h-8 items-center rounded-lg pr-1 transition-colors hover:bg-sidebar-accent/60",
        selected && "bg-sidebar-accent hover:bg-sidebar-accent",
      )}
    >
      {selected && <span className="absolute top-2 bottom-2 left-0 w-0.5 rounded-full bg-brand" />}
      <button
        type="button"
        onClick={() => void selectScenario(path)}
        title={path}
        aria-current={selected || undefined}
        className="flex h-full min-w-0 flex-1 cursor-default items-center gap-2 rounded-lg pl-2 text-left focus-ring"
      >
        <MethodBadge method={entry.method} className="w-11 shrink-0 text-2xs" />
        <span
          className={cn("min-w-0 flex-1 truncate text-sm", selected ? "text-foreground" : "text-sidebar-foreground/90")}
        >
          {entry.title}
        </span>
        {dirty && (
          <span title="Unsaved changes" className="size-1.5 shrink-0 rounded-full bg-brand">
            <span className="sr-only">, unsaved changes</span>
          </span>
        )}
        {change && <GitMarker kind={change} className="group-focus-within:hidden group-hover:hidden" />}
        {lastStatus !== undefined && (
          <StatusCode status={lastStatus} className="text-xs group-focus-within:hidden group-hover:hidden" />
        )}
      </button>
      <span className="hidden shrink-0 items-center text-muted-foreground group-focus-within:flex group-hover:flex">
        <RowAction label="Duplicate" onClick={() => void duplicateScenario(path)}>
          <Copy className="size-3.5" />
        </RowAction>
        <RowAction label="Rename file" onClick={() => renameScenario(path)}>
          <Pencil className="size-3.5" />
        </RowAction>
        <RowAction label="Delete" onClick={() => deleteScenario(path)}>
          <Trash2 className="size-3.5 hover:text-danger" />
        </RowAction>
      </span>
    </div>
  );
});
