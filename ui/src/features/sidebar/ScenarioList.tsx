import { useMemo } from "react";
import { Play, Plus } from "lucide-react";
import { runFolder } from "@/actions/request";
import { createScenarioIn } from "@/actions/scenario";
import type { ChangeKind } from "@/api/client";
import { groupByFolder, type FolderGroup } from "@/domain/entries";
import { changesByPath } from "@/domain/git";
import { useEditorStore } from "@/stores/editorStore";
import { useGitStore } from "@/stores/gitStore";
import { useProjectStore } from "@/stores/projectStore";
import { useResponseStore } from "@/stores/responseStore";
import { ScenarioListItem } from "./ScenarioListItem";
import { shortcutLabel } from "@/app/shortcuts";

const FOLDER_ACTION =
  "focus-ring grid size-6 place-items-center rounded-md hover:bg-sidebar-accent hover:text-foreground";

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
  return (
    <div className="mb-1">
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

/** Requests grouped by folder, each marked with its git change and last status. */
export function ScenarioList({ filter }: { filter: string }) {
  const project = useProjectStore((s) => s.project);
  const entries = useProjectStore((s) => s.entries);
  const groups = useMemo(() => groupByFolder(entries, filter), [entries, filter]);
  const gitStatus = useGitStore((s) => s.status);
  const changes = useMemo(() => changesByPath(gitStatus), [gitStatus]);

  return (
    <div>
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
