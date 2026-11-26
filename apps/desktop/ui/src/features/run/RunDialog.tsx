import { Loader2 } from "lucide-react";
import { cancelInFlight, dismissRun } from "@/actions/request";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { cn } from "@/lib/utils";
import { useRunStore, type FolderRun } from "@/stores/runStore";
import { RunResultRow } from "./RunResultRow";

function RunProgress({ run }: { run: FolderRun }) {
  const passed = run.results.filter((result) => result.ok).length;
  const failed = run.results.length - passed;
  const percent = (run.results.length / Math.max(run.total, 1)) * 100;
  return (
    <>
      <DialogDescription className="flex items-center gap-3">
        <span className="text-success">{passed} passed</span>
        {failed > 0 && <span className="text-danger">{failed} failed</span>}
        <span>
          {run.results.length}/{run.total}
        </span>
      </DialogDescription>
      <div className="h-1 overflow-hidden rounded-full bg-muted">
        <div
          className={cn("h-full transition-all", failed ? "bg-danger" : "bg-brand")}
          style={{ width: `${percent}%` }}
        />
      </div>
    </>
  );
}

/** Live results of a folder run; closable once it has finished. */
export function RunDialog() {
  const run = useRunStore((s) => s.run);
  const running = !!run && !run.done;

  return (
    <Dialog open={!!run} onOpenChange={(open) => !open && dismissRun()}>
      <DialogContent className="sm:max-w-2xl">
        <DialogHeader>
          <DialogTitle>
            Run <span className="font-mono text-sm text-muted-foreground">{run?.dir ? `${run.dir}/` : "all"}</span>
          </DialogTitle>
        </DialogHeader>
        {run && <RunProgress run={run} />}
        <div className="max-h-96 overflow-auto">
          {run?.results.map((result) => (
            <RunResultRow key={result.relPath} result={result} />
          ))}
          {running && (
            <div className="flex items-center gap-2 px-2 py-1.5 text-sm text-muted-foreground">
              <Loader2 className="size-4 animate-spin" /> Running…
            </div>
          )}
        </div>
        {running && (
          <Button variant="secondary" onClick={() => void cancelInFlight()} className="self-end">
            Stop
          </Button>
        )}
      </DialogContent>
    </Dialog>
  );
}
