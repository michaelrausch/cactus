import { cn } from "@/lib/utils";
import { methodColor } from "@/theme/httpColors";

/** An HTTP method in its colour. Size it with `className`, e.g. `text-2xs`. */
export function MethodBadge({ method, className }: { method: string; className?: string }) {
  return <span className={cn("font-mono font-bold", methodColor(method), className)}>{method.toUpperCase()}</span>;
}
