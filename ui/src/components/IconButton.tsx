import type { ReactNode } from "react";
import { cn } from "@/lib/utils";

const TONES = {
  toolbar: "hover:bg-accent",
  sidebar: "hover:bg-sidebar-accent",
} as const;

interface IconButtonProps {
  /** Tooltip, also the accessible name. */
  label: string;
  onClick: () => void;
  tone?: keyof typeof TONES;
  active?: boolean;
  children: ReactNode;
}

export function IconButton({ label, onClick, tone = "toolbar", active = false, children }: IconButtonProps) {
  return (
    <button
      type="button"
      title={label}
      aria-label={label}
      onClick={onClick}
      className={cn(
        "grid size-7 place-items-center rounded-md text-muted-foreground transition hover:text-foreground [&_svg]:size-4",
        TONES[tone],
        active && "bg-accent text-foreground",
      )}
    >
      {children}
    </button>
  );
}
