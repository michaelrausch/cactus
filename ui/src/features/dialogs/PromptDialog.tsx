import { useState } from "react";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { useUiStore, type PromptRequest } from "@/stores/uiStore";

/** The app-wide single-line text prompt, driven by `useUiStore().askPrompt`. */
export function PromptDialog() {
  const prompt = useUiStore((s) => s.prompt);
  const close = useUiStore((s) => s.closePrompt);
  return (
    <Dialog open={!!prompt} onOpenChange={(open) => !open && close()}>
      <DialogContent className="sm:max-w-md">{prompt && <PromptForm prompt={prompt} onDone={close} />}</DialogContent>
    </Dialog>
  );
}

/** Mounted per prompt, so its input starts from that prompt's initial value. */
function PromptForm({ prompt, onDone }: { prompt: PromptRequest; onDone: () => void }) {
  const [value, setValue] = useState(prompt.initial);
  const [submitting, setSubmitting] = useState(false);
  /** Closes once the work is done; if it failed (and said so), the text stays to fix and retry. */
  const submit = async () => {
    setSubmitting(true);
    const done = await prompt.onSubmit(value.trim());
    setSubmitting(false);
    if (done) onDone();
  };
  return (
    <>
      <DialogHeader>
        <DialogTitle>{prompt.title}</DialogTitle>
        {prompt.description && <DialogDescription>{prompt.description}</DialogDescription>}
      </DialogHeader>
      <form
        onSubmit={(e) => {
          e.preventDefault();
          if (!submitting) void submit();
        }}
      >
        <Input autoFocus value={value} placeholder={prompt.placeholder} onChange={(e) => setValue(e.target.value)} />
        <DialogFooter className="mt-4">
          <Button type="submit" disabled={submitting}>
            OK
          </Button>
        </DialogFooter>
      </form>
    </>
  );
}
