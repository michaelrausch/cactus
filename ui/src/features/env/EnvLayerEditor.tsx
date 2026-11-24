import { useEffect, useImperativeHandle, useState, type Ref } from "react";
import { Trash2 } from "lucide-react";
import { toast } from "sonner";
import { backend, type EnvFile } from "@/api/client";
import { KeyValueTable } from "@/components/KeyValueTable";
import { Button } from "@/components/ui/button";
import { DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { envLayerInfo } from "@/domain/envFiles";
import { recordFromRows, rowsFingerprint, rowsFromRecord, type Row } from "@/domain/rows";
import { errorMessage, reportErrors } from "@/lib/errors";

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
}

/** Edits one layer file. Mount with a `key` per file so it loads fresh. */
export function EnvLayerEditor({ project, file, ref }: EnvLayerEditorProps) {
  const [rows, setRows] = useState<Row[] | null>(null);
  /** Fingerprint of the rows as they are on disk. */
  const [savedFingerprint, setSavedFingerprint] = useState<string | null>(null);
  const info = envLayerInfo(file);

  useEffect(() => {
    let current = true;
    backend.loadEnv(project, file).then(
      (env) => {
        if (!current) return;
        const loaded = rowsFromRecord(env);
        setRows(loaded);
        setSavedFingerprint(rowsFingerprint(loaded));
      },
      (error: unknown) => toast.error(errorMessage(error)),
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
    toast.success(`Saved ${info.path}`);
    return true;
  };

  useImperativeHandle(ref, () => ({ hasUnsavedChanges: () => dirty, save }));

  return (
    <div className="flex min-w-0 flex-1 flex-col p-5">
      <DialogHeader className="mb-4">
        <DialogTitle className="flex items-baseline gap-2">
          {info.label} <span className="font-mono text-xs font-normal text-muted-foreground">{info.path}</span>
          {dirty && <span className="size-1.5 self-center rounded-full bg-amber-400" title="Unsaved changes" />}
        </DialogTitle>
        <DialogDescription>{info.note}</DialogDescription>
      </DialogHeader>
      <div className="min-h-0 flex-1 overflow-auto">
        {rows && <KeyValueTable rows={rows} onChange={setRows} keyPlaceholder="Variable" />}
      </div>
      <DialogFooter className="mt-4">
        {file.kind === "captured" && (
          <Button variant="ghost" onClick={() => setRows([])}>
            <Trash2 /> Clear all
          </Button>
        )}
        <Button onClick={() => void save()}>Save</Button>
      </DialogFooter>
    </div>
  );
}
