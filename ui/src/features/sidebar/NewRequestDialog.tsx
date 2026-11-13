import { useState } from "react";
import { createRequest } from "@/actions/scenario";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { MethodBadge } from "@/components/MethodBadge";
import { HTTP_METHODS, suggestedTitle } from "@/domain/http";
import { cn } from "@/lib/utils";
import { methodColor } from "@/theme/httpColors";
import { useProjectStore } from "@/stores/projectStore";
import { useUiStore } from "@/stores/uiStore";

/** Select can't hold an empty value, so the project root goes by this. */
const ROOT = "/";

/** Mounted per opening, so it starts fresh each time. */
function NewRequestForm({ initialDir, onDone }: { initialDir: string; onDone: () => void }) {
  const folders = useProjectStore((s) => s.folders);
  const entries = useProjectStore((s) => s.entries);
  const [method, setMethod] = useState<string>("GET");
  const [url, setUrl] = useState("");
  const [name, setName] = useState("");
  const [dir, setDir] = useState(initialDir || ROOT);
  const [submitting, setSubmitting] = useState(false);

  // Folders with requests but no folder.yml are folders too.
  const entryDirs = entries.map((entry) => entry.relPath.split("/").slice(0, -1).join("/")).filter(Boolean);
  const choices = [...new Set([...folders, ...entryDirs, initialDir].filter(Boolean))].toSorted();
  const suggested = suggestedTitle(method, url);
  const title = name.trim() || suggested || "Untitled";

  const submit = async () => {
    if (submitting) return;
    setSubmitting(true);
    const created = await createRequest(dir === ROOT ? "" : dir, title, method, url);
    setSubmitting(false);
    if (created) onDone();
  };

  return (
    <form
      className="flex flex-col gap-4"
      onSubmit={(event) => {
        event.preventDefault();
        void submit();
      }}
    >
      <DialogHeader>
        <DialogTitle>New request</DialogTitle>
      </DialogHeader>

      <div className="flex items-center gap-1 rounded-lg border bg-card p-1 focus-within:border-brand-2/40">
        <Select value={method} onValueChange={setMethod}>
          <SelectTrigger
            aria-label="Method"
            className={cn(
              "h-8 w-[6.5rem] border-0 bg-transparent font-mono text-xs font-bold shadow-none dark:bg-transparent dark:hover:bg-accent",
              methodColor(method),
            )}
          >
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {HTTP_METHODS.map((option) => (
              <SelectItem key={option} value={option}>
                <MethodBadge method={option} className="text-xs" />
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
        <input
          autoFocus
          value={url}
          onChange={(event) => setUrl(event.target.value)}
          placeholder="{{baseUrl}}/path"
          aria-label="URL"
          className="h-8 min-w-0 flex-1 bg-transparent px-2 font-mono text-sm outline-none placeholder:text-muted-foreground"
        />
      </div>

      <div className="grid grid-cols-[1fr_11rem] gap-3">
        <label className="flex flex-col gap-1.5 text-sm">
          <span className="font-medium">Name</span>
          <Input value={name} placeholder={suggested || "Get user"} onChange={(event) => setName(event.target.value)} />
        </label>
        <label className="flex flex-col gap-1.5 text-sm">
          <span className="font-medium">Folder</span>
          <Select value={dir} onValueChange={setDir}>
            <SelectTrigger aria-label="Folder" className="w-full">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value={ROOT}>/</SelectItem>
              {choices.map((folder) => (
                <SelectItem key={folder} value={folder}>
                  {folder}/
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </label>
      </div>

      <DialogFooter>
        <Button type="submit" disabled={submitting}>
          Create
        </Button>
      </DialogFooter>
    </form>
  );
}

export function NewRequestDialog() {
  const dir = useUiStore((s) => s.newRequestDir);
  const setDir = useUiStore((s) => s.setNewRequestDir);
  return (
    <Dialog open={dir !== null} onOpenChange={(open) => !open && setDir(null)}>
      <DialogContent className="sm:max-w-lg" aria-describedby={undefined}>
        {dir !== null && <NewRequestForm initialDir={dir} onDone={() => setDir(null)} />}
      </DialogContent>
    </Dialog>
  );
}
