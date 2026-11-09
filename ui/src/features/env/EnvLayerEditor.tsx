import { useEffect, useImperativeHandle, useState, type KeyboardEvent, type Ref } from "react";
import { Eye, EyeOff, Pencil, Trash2 } from "lucide-react";
import { backend, type EnvFile } from "@/api/client";
import { KeyValueTable } from "@/components/KeyValueTable";
import { Button } from "@/components/ui/button";
import { DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { envLayerInfo } from "@/domain/envFiles";
import { recordFromRows, rowsFingerprint, rowsFromRecord, type Row } from "@/domain/rows";
import { reportErrors } from "@/lib/errors";

/** What the dialog around the editor needs to guard unsaved edits. */
export interface EnvLayerEditorHandle {
  hasUnsavedChanges: () => boolean;
  /** Resolves to whether the layer was saved. */
  save: () => Promise<boolean>;
}

interface EnvLayerEditorProps {
  project: string;
  file: EnvFile;
  ref?: Ref<EnvLayerEditorHandle>;
  /** For a named environment. */
  onRename: (name: string) => void;
  onDelete: (name: string) => void;
}

/** Edits one layer file. Mount with a `key` per file so it loads fresh. */
export function EnvLayerEditor({ project, file, ref, onRename, onDelete }: EnvLayerEditorProps) {
  const [rows, setRows] = useState<Row[] | null>(null);
  /** Fingerprint of the rows as they are on disk. */
  const [savedFingerprint, setSavedFingerprint] = useState<string | null>(null);
  const info = envLayerInfo(file);
  const secret = file.kind === "secrets";
  /** Secrets start hidden, so a screen share doesn't show them. */
  const [revealed, setRevealed] = useState(!secret);

  useEffect(() => {
    let current = true;
    void reportErrors(
      async () => {
        const env = await backend.loadEnv(project, file);
        if (!current) return;
        const loaded = rowsFromRecord(env);
        setRows(loaded);
        setSavedFingerprint(rowsFingerprint(loaded));
      },
      `Couldn't load ${envLayerInfo(file).path}`,
    );
    return () => {
      current = false;
    };
  }, [project, file]);

  const dirty = rows !== null && rowsFingerprint(rows) !== savedFingerprint;

  const save = async (): Promise<boolean> => {
    if (!rows) return true;
    const saved = await reportErrors(async () => {
      await backend.saveEnv(project, file, recordFromRows(rows));
      return true;
    });
    if (!saved) return false;
    setSavedFingerprint(rowsFingerprint(rows));
    return true;
  };

  /** The global Ctrl+S waits while a dialog is open; here it saves this layer. */
  const onKeyDown = (event: KeyboardEvent) => {
    if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "s") {
      event.preventDefault();
      void save();
    }
  };

  useImperativeHandle(ref, () => ({ hasUnsavedChanges: () => dirty, save }));

  return (
    <div className="flex min-w-0 flex-1 flex-col p-5" onKeyDown={onKeyDown}>
      <DialogHeader className="mb-4">
        <DialogTitle className="flex items-baseline gap-2">
          {info.label} <span className="font-mono text-xs font-normal text-muted-foreground">{info.path}</span>
          {dirty && (
            <span className="size-1.5 self-center rounded-full bg-warning" title="Unsaved changes">
              <span className="sr-only">Unsaved changes</span>
            </span>
          )}
        </DialogTitle>
        <DialogDescription className="sr-only">The variables in {info.path}.</DialogDescription>
      </DialogHeader>
      <div className="min-h-0 flex-1 overflow-auto">
        {rows && (
          <KeyValueTable rows={rows} onChange={setRows} keyPlaceholder="Variable" valueColumn={{ masked: !revealed }} />
        )}
      </div>
      <DialogFooter className="mt-4">
        {file.kind === "named" && (
          <>
            <Button variant="ghost" onClick={() => onRename(file.name)}>
              <Pencil /> Rename…
            </Button>
            <Button variant="ghost" className="hover:text-danger sm:mr-auto" onClick={() => onDelete(file.name)}>
              <Trash2 /> Delete…
            </Button>
          </>
        )}
        {secret && (
          <Button variant="ghost" className="sm:mr-auto" aria-pressed={revealed} onClick={() => setRevealed(!revealed)}>
            {revealed ? <EyeOff /> : <Eye />} {revealed ? "Hide values" : "Show values"}
          </Button>
        )}
        {file.kind === "captured" && (
          <Button variant="ghost" onClick={() => setRows([])}>
            <Trash2 /> Clear all
          </Button>
        )}
        <Button onClick={() => void save()} disabled={!dirty}>
          Save
        </Button>
      </DialogFooter>
    </div>
  );
}
