import type { ReactNode } from "react";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
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
  disabled?: boolean;
  children: ReactNode;
}

/** An icon-only button with a tooltip that shows on hover and on keyboard focus. */
export function IconButton({
  label,
  onClick,
  tone = "toolbar",
  active = false,
  disabled = false,
  children,
}: IconButtonProps) {
  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <button
          type="button"
          aria-label={label}
          // aria-disabled rather than disabled, so the tooltip still shows.
          aria-disabled={disabled || undefined}
          onClick={disabled ? undefined : onClick}
          className={cn(
            "grid size-7 shrink-0 place-items-center rounded-md text-muted-foreground focus-ring transition hover:text-foreground [&_svg]:size-4",
            TONES[tone],
            active && "bg-accent text-foreground",
            disabled && "opacity-40 hover:bg-transparent hover:text-muted-foreground",
          )}
        >
          {children}
        </button>
      </TooltipTrigger>
      <TooltipContent>{label}</TooltipContent>
    </Tooltip>
  );
}
