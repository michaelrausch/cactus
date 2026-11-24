import type { ChangeKind } from "@/api/client";
import { kindLabel, markerFor } from "@/domain/git";
import { cn } from "@/lib/utils";
import { changeColor } from "@/theme/gitColors";

/** A file's git change as one coloured letter, e.g. `M`. */
export function GitMarker({ kind, className }: { kind: ChangeKind; className?: string }) {
  return (
    <span
      title={kindLabel(kind)}
      className={cn("w-3 shrink-0 text-center font-mono text-xs font-semibold", changeColor(kind), className)}
    >
      {markerFor(kind)}
    </span>
  );
}
