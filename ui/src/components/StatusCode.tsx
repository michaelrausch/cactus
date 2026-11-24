import type { ReactNode } from "react";
import { cn } from "@/lib/utils";
import { statusColor } from "@/theme/httpColors";

interface StatusCodeProps {
  status: number;
  className?: string;
  /** Defaults to the status number. */
  children?: ReactNode;
}

export function StatusCode({ status, className, children }: StatusCodeProps) {
  return <span className={cn("font-mono", statusColor(status), className)}>{children ?? status}</span>;
}
