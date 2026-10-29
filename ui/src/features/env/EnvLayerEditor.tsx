import { useEffect, useState } from "react";
import { Trash2 } from "lucide-react";
import { toast } from "sonner";
import { backend, type EnvFile } from "@/api/client";
import { KeyValueTable } from "@/components/KeyValueTable";
import { Button } from "@/components/ui/button";
import { DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { envLayerInfo } from "@/domain/envFiles";
import { recordFromRows, rowsFromRecord, type Row } from "@/domain/rows";
import { errorMessage, reportErrors } from "@/lib/errors";

/** Edits one layer file. Mount with a `key` per file so it loads fresh. */
export function EnvLayerEditor({ project, file }: { project: string; file: EnvFile }) {
  const [rows, setRows] = useState<Row[] | null>(null);
  const info = envLayerInfo(file);

  useEffect(() => {
    let current = true;
    backend.loadEnv(project, file).then(
      (env) => current && setRows(rowsFromRecord(env)),
      (error: unknown) => toast.error(errorMessage(error)),
    );
    return () => {
      current = false;
    };
  }, [project, file]);

  const save = () =>
    reportErrors(async () => {
      if (!rows) return;
      await backend.saveEnv(project, file, recordFromRows(rows));
      toast.success(`Saved ${info.path}`);
    });

  return (
    <div className="flex min-w-0 flex-1 flex-col p-5">
      <DialogHeader className="mb-4">
        <DialogTitle className="flex items-baseline gap-2">
          {info.label} <span className="font-mono text-xs font-normal text-muted-foreground">{info.path}</span>
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
