import { useMemo } from "react";
import { Play, Plus } from "lucide-react";
import { runFolder } from "@/actions/request";
import { createScenarioIn } from "@/actions/scenario";
import { groupByFolder, type FolderGroup } from "@/domain/entries";
import { useEditorStore } from "@/stores/editorStore";
import { useProjectStore } from "@/stores/projectStore";
import { useResponseStore } from "@/stores/responseStore";
import { ScenarioListItem } from "./ScenarioListItem";

function FolderHeader({ dir }: { dir: string }) {
  return (
    <div className="group mt-3 flex h-6 items-center gap-1 px-2 text-[11px] font-medium tracking-wide text-muted-foreground/80 uppercase">
      <span className="flex-1 truncate">{dir}</span>
      <span className="hidden items-center gap-1 group-hover:flex">
        <button
          type="button"
          title={`Run ${dir}/`}
          onClick={() => void runFolder(dir)}
          className="hover:text-foreground"
        >
          <Play className="size-3.5" />
        </button>
        <button
          type="button"
          title={`New in ${dir}/`}
          onClick={() => createScenarioIn(dir)}
          className="hover:text-foreground"
        >
          <Plus className="size-3.5" />
        </button>
      </span>
    </div>
  );
}

function FolderSection({ group }: { group: FolderGroup }) {
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
        />
      ))}
    </div>
  );
}

export function ScenarioList({ filter }: { filter: string }) {
  const project = useProjectStore((s) => s.project);
  const entries = useProjectStore((s) => s.entries);
  const groups = useMemo(() => groupByFolder(entries, filter), [entries, filter]);

  return (
    <div className="min-h-0 flex-1 overflow-y-auto px-2 pb-2">
      {groups.map((group) => (
        <FolderSection key={group.dir} group={group} />
      ))}
      {project && entries.length === 0 && (
        <p className="px-2 py-4 text-sm text-muted-foreground">
          No scenarios yet. Press Ctrl+N or import a collection.
        </p>
      )}
    </div>
  );
}
