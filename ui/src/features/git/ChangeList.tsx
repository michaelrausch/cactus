import { useMemo } from "react";
import { Minus, Plus, Undo2 } from "lucide-react";
import type { DiffSide, FileStatus } from "@/api/client";
import { discard, discardAll, openInEditor, showDiff, stage, stageAll, unstage, unstageAll } from "@/actions/git";
import { GitMarker } from "@/components/GitMarker";
import { IconButton } from "@/components/IconButton";
import { changeLabel, changeOn } from "@/domain/git";
import { cn } from "@/lib/utils";
import { useGitStore } from "@/stores/gitStore";
import { useProjectStore } from "@/stores/projectStore";

/** Stage or unstage, plus discard for unstaged changes. */
export function FileActions({ file, side }: { file: FileStatus; side: DiffSide }) {
  if (side === "staged") {
    return (
      <IconButton label="Unstage" tone="sidebar" onClick={() => void unstage(file)}>
        <Minus />
      </IconButton>
    );
  }
  return (
    <>
      <IconButton label="Discard changes" tone="sidebar" onClick={() => discard(file)}>
        <Undo2 />
      </IconButton>
      <IconButton label="Stage" tone="sidebar" onClick={() => stage([file.path])}>
        <Plus />
      </IconButton>
    </>
  );
}

function ChangeRow({ file, side, titles }: { file: FileStatus; side: DiffSide; titles: Record<string, string> }) {
  const selected = useGitStore((s) => s.selected?.path === file.path && s.selected.side === side);
  const kind = changeOn(file, side);
  if (!kind) return null;
  const { name, detail } = changeLabel(file.path, titles);
  const isRequest = file.path in titles;
  return (
    <div
      className={cn(
        "group flex h-8 items-center gap-1 rounded-md pr-1 hover:bg-sidebar-accent/60",
        selected && "bg-sidebar-accent hover:bg-sidebar-accent",
      )}
    >
      <button
        type="button"
        onClick={() => void showDiff({ path: file.path, side })}
        onDoubleClick={() => void openInEditor(file.path)}
        title={[
          file.renamedFrom ? `${file.renamedFrom} → ${file.path}` : file.path,
          isRequest && "Double-click to open",
        ]
          .filter(Boolean)
          .join("\n")}
        className="flex h-full min-w-0 flex-1 items-center gap-2 rounded-md pl-2 text-left focus-ring"
      >
        <GitMarker kind={kind} />
        <span className="shrink-0 truncate text-sm">{name}</span>
        {detail && <span className="min-w-0 truncate text-xs text-muted-foreground">{detail}</span>}
      </button>
      <span className="hidden items-center group-focus-within:flex group-hover:flex">
        <FileActions file={file} side={side} />
      </span>
    </div>
  );
}

function SectionActions({ side, count }: { side: DiffSide; count: number }) {
  if (side === "staged") {
    return (
      <IconButton label="Unstage all" tone="sidebar" onClick={() => void unstageAll()}>
        <Minus />
      </IconButton>
    );
  }
  return (
    <>
      <IconButton label="Discard all" tone="sidebar" onClick={() => discardAll(count)}>
        <Undo2 />
      </IconButton>
      <IconButton label="Stage all" tone="sidebar" onClick={stageAll}>
        <Plus />
      </IconButton>
    </>
  );
}

/** One side's changed files, with actions for all of them. Hidden when empty. */
export function ChangeList({ side, files }: { side: DiffSide; files: FileStatus[] }) {
  const entries = useProjectStore((s) => s.entries);
  const titles = useMemo(() => Object.fromEntries(entries.map((entry) => [entry.relPath, entry.title])), [entries]);
  if (files.length === 0) return null;
  return (
    <div className="mb-3">
      <div className="flex h-7 items-center pr-1 pl-2 text-xs font-medium tracking-wide text-muted-foreground uppercase">
        <span className="flex-1">
          {side === "staged" ? "Staged" : "Changes"} · {files.length}
        </span>
        <SectionActions side={side} count={files.length} />
      </div>
      {files.map((file) => (
        <ChangeRow key={file.path} file={file} side={side} titles={titles} />
      ))}
    </div>
  );
}
