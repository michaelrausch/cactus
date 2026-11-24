import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { useUiStore } from "@/stores/uiStore";

/** The app-wide confirmation, driven by `useUiStore().askConfirm`. */
export function ConfirmDialog() {
  const confirm = useUiStore((s) => s.confirm);
  const close = useUiStore((s) => s.closeConfirm);
  const destructive = confirm?.destructive ?? false;
  const choose = (onConfirm: (() => Promise<void> | void) | undefined) => {
    close();
    void onConfirm?.();
  };
  return (
    <Dialog open={!!confirm} onOpenChange={(open) => !open && close()}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>{confirm?.title}</DialogTitle>
          <DialogDescription>{confirm?.description}</DialogDescription>
        </DialogHeader>
        <DialogFooter>
          {/* A destructive action never takes Enter: focus starts on Cancel. */}
          <Button variant="ghost" autoFocus={destructive} onClick={close}>
            Cancel
          </Button>
          {confirm?.alternative && (
            <Button
              variant={confirm.alternative.destructive ? "destructive" : "outline"}
              onClick={() => choose(confirm.alternative?.onConfirm)}
            >
              {confirm.alternative.action}
            </Button>
          )}
          <Button
            variant={destructive ? "destructive" : "default"}
            autoFocus={!destructive}
            onClick={() => choose(confirm?.onConfirm)}
          >
            {confirm?.action}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
