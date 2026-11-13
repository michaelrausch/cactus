import { useMemo, useState, type DragEvent } from "react";
import { Play, Plus } from "lucide-react";
import { runFolder } from "@/actions/request";
import { createScenarioIn, moveScenario } from "@/actions/scenario";
import type { ChangeKind, ProjectPath } from "@/api/client";
import { groupByFolder, type FolderGroup } from "@/domain/entries";
import { changesByPath } from "@/domain/git";
import { cn } from "@/lib/utils";
import { useEditorStore } from "@/stores/editorStore";
import { useGitStore } from "@/stores/gitStore";
import { useProjectStore } from "@/stores/projectStore";
import { useResponseStore } from "@/stores/responseStore";
import { REQUEST_DRAG_TYPE, ScenarioListItem } from "./ScenarioListItem";
import { shortcutLabel } from "@/app/shortcuts";

const FOLDER_ACTION =
  "focus-ring grid size-6 place-items-center rounded-md hover:bg-sidebar-accent hover:text-foreground";

const isRequestDrag = (event: DragEvent): boolean => event.dataTransfer.types.includes(REQUEST_DRAG_TYPE);

/** Dropping a dragged request here moves it into `dir`. `over` is for highlighting. */
function useDropInto(dir: string) {
  const [over, setOver] = useState(false);
  const handlers = {
    onDragOver: (event: DragEvent) => {
      if (!isRequestDrag(event)) return;
      event.preventDefault();
      event.stopPropagation();
      event.dataTransfer.dropEffect = "move";
      setOver(true);
    },
    onDragLeave: (event: DragEvent) => {
      if (!event.currentTarget.contains(event.relatedTarget as Node | null)) setOver(false);
    },
    onDrop: (event: DragEvent) => {
      setOver(false);
      const path = event.dataTransfer.getData(REQUEST_DRAG_TYPE);
      if (!path) return;
      event.preventDefault();
      event.stopPropagation();
      void moveScenario(path as ProjectPath, dir);
    },
  };
  return { over, handlers };
}

function FolderHeader({ dir }: { dir: string }) {
  return (
    <div className="group mt-3 flex h-6 items-center gap-1 pr-1 pl-2 text-xs font-medium tracking-wide text-muted-foreground uppercase">
      <span className="flex-1 truncate">{dir}</span>
      {/* Faded rather than hidden, so the buttons stay in the tab order. */}
      <span className="flex items-center opacity-0 group-focus-within:opacity-100 group-hover:opacity-100">
        <button
          type="button"
          title={`Run ${dir}/`}
          aria-label={`Run ${dir}/`}
          onClick={() => void runFolder(dir)}
          className={FOLDER_ACTION}
        >
          <Play className="size-3.5" />
        </button>
        <button
          type="button"
          title={`New request in ${dir}/`}
          aria-label={`New request in ${dir}/`}
          onClick={() => createScenarioIn(dir)}
          className={FOLDER_ACTION}
        >
          <Plus className="size-3.5" />
        </button>
      </span>
    </div>
  );
}

function FolderSection({ group, changes }: { group: FolderGroup; changes: Record<string, ChangeKind> }) {
  const selectedPath = useEditorStore((s) => s.path);
  const lastStatusByPath = useResponseStore((s) => s.lastStatusByPath);
  const { over, handlers } = useDropInto(group.dir);
  return (
    <div
      {...handlers}
      className={cn("mb-1 rounded-lg transition-colors", over && "ring-brand/50 bg-sidebar-accent/40 ring-1")}
    >
      {group.dir && <FolderHeader dir={group.dir} />}
      {group.entries.map((entry) => (
        <ScenarioListItem
          key={entry.relPath}
          entry={entry}
          selected={entry.relPath === selectedPath}
          lastStatus={lastStatusByPath[entry.relPath]}
          change={changes[entry.relPath]}
        />
      ))}
    </div>
  );
}

/** Requests grouped by folder, each marked with its git change and last status. Drag a request onto a folder to move it. */
export function ScenarioList({ filter }: { filter: string }) {
  const project = useProjectStore((s) => s.project);
  const entries = useProjectStore((s) => s.entries);
  const folders = useProjectStore((s) => s.folders);
  const groups = useMemo(() => groupByFolder(entries, filter, folders), [entries, filter, folders]);
  const gitStatus = useGitStore((s) => s.status);
  const changes = useMemo(() => changesByPath(gitStatus), [gitStatus]);
  // Dropped anywhere outside a folder: the project root.
  const root = useDropInto("");

  return (
    <div {...root.handlers} className={cn("rounded-lg pb-2", root.over && "bg-sidebar-accent/20")}>
      {groups.map((group) => (
        <FolderSection key={group.dir} group={group} changes={changes} />
      ))}
      {project && entries.length === 0 && (
        <p className="px-2 py-4 text-sm text-muted-foreground">
          No requests yet. Press {shortcutLabel("newScenario")} or import a collection.
        </p>
      )}
    </div>
  );
}
