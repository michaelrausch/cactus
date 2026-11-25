import type { DiffLine } from "@/api/client";
import { GitMarker } from "@/components/GitMarker";
import { changeOn } from "@/domain/git";
import { cn } from "@/lib/utils";
import { useGitStore } from "@/stores/gitStore";

const LINE_STYLES: Record<DiffLine["kind"], { prefix: string; className: string }> = {
  context: { prefix: " ", className: "" },
  added: { prefix: "+", className: "bg-success/10 text-success" },
  removed: { prefix: "-", className: "bg-danger/10 text-danger" },
};

const Notice = ({ children }: { children: string }) => (
  <p className="px-4 py-3 font-sans text-sm text-muted-foreground">{children}</p>
);

function Line({ line }: { line: DiffLine }) {
  const style = LINE_STYLES[line.kind];
  return (
    <div className={cn("grid grid-cols-[2.75rem_2.75rem_1fr]", style.className)}>
      <span className="pr-2 text-right text-muted-foreground select-none">{line.oldLine ?? ""}</span>
      <span className="pr-2 text-right text-muted-foreground select-none">{line.newLine ?? ""}</span>
      <span className="pr-4 whitespace-pre">
        <span className="select-none">{style.prefix} </span>
        {line.text}
      </span>
    </div>
  );
}

/** The selected file's staged or unstaged changes, with actions for it. */
export function DiffView() {
  const selected = useGitStore((s) => s.selected);
  const file = useGitStore((s) => s.status?.files.find((f) => f.path === s.selected?.path));
  const diff = useGitStore((s) => s.diff);

  if (!selected || !file) {
    return (
      <div className="grid flex-1 place-items-center">
        <Notice>Select a file to see its changes.</Notice>
      </div>
    );
  }
  const kind = changeOn(file, selected.side);
  return (
    <div className="flex min-h-0 min-w-0 flex-1 flex-col">
      <div className="flex h-12 shrink-0 items-center gap-2 border-b pr-12 pl-4 text-sm">
        {kind && <GitMarker kind={kind} />}
        <span className="min-w-0 truncate font-medium">{selected.path}</span>
        <span className="shrink-0 text-xs text-muted-foreground">
          {selected.side === "staged" ? "staged" : "not staged"}
        </span>
      </div>
      <div className="min-h-0 flex-1 overflow-auto py-2 font-mono text-xs leading-5">
        {diff?.binary && <Notice>Binary file</Notice>}
        {diff && !diff.binary && diff.hunks.length === 0 && <Notice>No textual changes</Notice>}
        {diff?.hunks.map((hunk) => (
          // Hunk ranges never repeat within a file.
          <div key={hunk.header} className="mb-2 min-w-fit">
            <div className="bg-muted/40 px-4 text-muted-foreground">{hunk.header}</div>
            {hunk.lines.map((line) => (
              <Line key={`${line.oldLine}:${line.newLine}`} line={line} />
            ))}
          </div>
        ))}
        {diff?.truncated && <Notice>The diff is too long to show in full.</Notice>}
      </div>
    </div>
  );
}
