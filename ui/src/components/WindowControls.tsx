import type { ReactNode } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Minus, Square, X } from "lucide-react";
import { cn } from "@/lib/utils";

function ControlButton({
  label,
  onClick,
  className,
  children,
}: {
  label: string;
  onClick: () => void;
  className?: string;
  children: ReactNode;
}) {
  return (
    <button
      type="button"
      aria-label={label}
      title={label}
      onClick={onClick}
      className={cn(
        "grid h-12 w-11 place-items-center text-muted-foreground focus-ring transition hover:bg-accent hover:text-foreground [&_svg]:size-4",
        className,
      )}
    >
      {children}
    </button>
  );
}

/** Minimize, maximize and close for undecorated windows (Windows, Linux). */
export function WindowControls() {
  const win = getCurrentWindow();
  return (
    <div className="-mr-4 ml-auto flex self-start">
      <ControlButton label="Minimize" onClick={() => void win.minimize()}>
        <Minus />
      </ControlButton>
      <ControlButton label="Maximize" onClick={() => void win.toggleMaximize()}>
        <Square className="size-3.5!" />
      </ControlButton>
      <ControlButton label="Close" onClick={() => void win.close()} className="hover:bg-[#e81123] hover:text-white">
        <X />
      </ControlButton>
    </div>
  );
}
