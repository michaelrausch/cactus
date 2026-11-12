import { BellRing, X } from "lucide-react";
import { viewAlert } from "@/actions/alerts";
import { Button } from "@/components/ui/button";
import { useMonitorStore } from "@/stores/monitorStore";

/** Alerts raised while the window was in the background, waiting at the top when it's back. */
export function AlertsBanner() {
  const unseen = useMonitorStore((s) => s.unseen);
  const dismiss = useMonitorStore((s) => s.alertsSeen);
  const latest = unseen.at(-1);
  if (!latest) return null;
  return (
    <div
      role="status"
      className="fixed top-3 left-1/2 z-50 flex -translate-x-1/2 items-center gap-2 rounded-full border bg-popover py-1 pr-1 pl-3 text-sm text-popover-foreground shadow-lg shadow-shadow/30"
    >
      <BellRing className="size-4 text-warning" />
      {unseen.length} {unseen.length === 1 ? "alert" : "alerts"}
      <Button size="sm" variant="ghost" className="h-7" onClick={() => void viewAlert(latest)}>
        View
      </Button>
      <button
        type="button"
        aria-label="Dismiss"
        onClick={dismiss}
        className="grid size-7 place-items-center rounded-full text-muted-foreground focus-ring hover:bg-accent hover:text-foreground"
      >
        <X className="size-3.5" />
      </button>
    </div>
  );
}
