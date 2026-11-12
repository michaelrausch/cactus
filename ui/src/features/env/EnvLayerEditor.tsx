import { useEffect, useImperativeHandle, useState, type KeyboardEvent, type Ref } from "react";
import { Eye, EyeOff, Lock, LockOpen, Pencil, Trash2 } from "lucide-react";
import { ensureUnlocked } from "@/actions/secrets";
import { backend, type EnvFile } from "@/api/client";
import { KeyValueTable } from "@/components/KeyValueTable";
import { Button } from "@/components/ui/button";
import { DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { envLayerInfo } from "@/domain/envFiles";
import { recordFromRows, rowsFingerprint, rowsFromRecord, type Row } from "@/domain/rows";
import { reportErrors } from "@/lib/errors";
import { useSecretsStore } from "@/stores/secretsStore";

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
  onRename?: (name: string) => void;
  onDelete?: (name: string) => void;
  onSaved?: () => void;
  /** Laid out on a page rather than in a dialog, e.g. a critter's Context tab. */
  inline?: boolean;
}

/** Edits one layer file. Mount with a `key` per file so it loads fresh. */
export function EnvLayerEditor({
  project,
  file,
  ref,
  onRename,
  onDelete,
  onSaved,
  inline = false,
}: EnvLayerEditorProps) {
  const [rows, setRows] = useState<Row[] | null>(null);
  /** Fingerprint of the rows as they are on disk. */
  const [savedFingerprint, setSavedFingerprint] = useState<string | null>(null);
  const info = envLayerInfo(file);
  const secret = file.kind === "secrets" || file.kind === "critterSecrets";
  /** Secrets start hidden, so a screen share doesn't show them. */
  const [revealed, setRevealed] = useState(!secret);
  const encryption = useSecretsStore((s) => (secret ? (s.info?.status ?? null) : null));
  const anySealed = useSecretsStore((s) => s.info?.hasSealedValues ?? false);
  /** Locked with encrypted values: the layer shows its names only. */
  const hidden = encryption === "locked" && anySealed;
  const [lockedNames, setLockedNames] = useState<string[] | null>(null);

  useEffect(() => {
    if (secret && encryption === null) return; // Until it's known whether values can be read.
    let current = true;
    void reportErrors(
      async () => {
        if (hidden) {
          const names = await backend.secretNames(project, file);
          if (current) setLockedNames(names);
          return;
        }
        const env = await backend.loadEnv(project, file);
        if (!current) return;
        const loaded = rowsFromRecord(env);
        setLockedNames(null);
        setRows(loaded);
        setSavedFingerprint(rowsFingerprint(loaded));
      },
      `Couldn't load ${envLayerInfo(file).path}`,
    );
    return () => {
      current = false;
    };
  }, [project, file, secret, encryption, hidden]);

  const dirty = rows !== null && rowsFingerprint(rows) !== savedFingerprint;

  const save = async (): Promise<boolean> => {
    if (!rows) return true;
    if (encryption === "locked" && !(await ensureUnlocked({ force: true }))) return false;
    const saved = await reportErrors(async () => {
      await backend.saveEnv(project, file, recordFromRows(rows));
      return true;
    });
    if (!saved) return false;
    setSavedFingerprint(rowsFingerprint(rows));
    onSaved?.();
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

  const title = (
    <>
      {info.label} <span className="font-mono text-xs font-normal text-muted-foreground">{info.path}</span>
      {encryption && encryption !== "off" && (
        <span className="text-xs font-normal text-muted-foreground">
          {encryption === "locked" ? <Lock className="inline size-3" /> : <LockOpen className="inline size-3" />}{" "}
          encrypted
        </span>
      )}
      {dirty && (
        <span className="size-1.5 self-center rounded-full bg-warning" title="Unsaved changes">
          <span className="sr-only">Unsaved changes</span>
        </span>
      )}
    </>
  );
  const buttons = (
    <>
      {file.kind === "named" && onRename && onDelete && (
        <>
          <Button variant="ghost" onClick={() => onRename(file.name)}>
            <Pencil /> Rename…
          </Button>
          <Button variant="ghost" className="hover:text-danger sm:mr-auto" onClick={() => onDelete(file.name)}>
            <Trash2 /> Delete…
          </Button>
        </>
      )}
      {hidden && (
        <Button onClick={() => void ensureUnlocked({ force: true })}>
          <LockOpen /> Unlock…
        </Button>
      )}
      {secret && !hidden && (
        <Button variant="ghost" aria-pressed={revealed} onClick={() => setRevealed(!revealed)}>
          {revealed ? <EyeOff /> : <Eye />} {revealed ? "Hide values" : "Show values"}
        </Button>
      )}
      {secret && encryption === "off" && import.meta.env.SECRETS_ENCRYPTION && (
        <Button variant="ghost" onClick={() => useSecretsStore.getState().setDialog("enable")}>
          <Lock /> Encrypt…
        </Button>
      )}
      {secret && <span className="hidden sm:mr-auto sm:block" />}
      {file.kind === "captured" && (
        <Button variant="ghost" onClick={() => setRows([])}>
          <Trash2 /> Clear all
        </Button>
      )}
      {!hidden && (
        <Button onClick={() => void save()} disabled={!dirty}>
          Save
        </Button>
      )}
    </>
  );
  const table = (
    <div className="min-h-0 flex-1 overflow-auto">
      {lockedNames ? (
        <ul className="divide-y text-sm">
          {lockedNames.map((name) => (
            <li key={name} className="flex justify-between gap-4 px-2 py-1.5">
              <span className="font-mono">{name}</span>
              <span className="text-muted-foreground">encrypted</span>
            </li>
          ))}
        </ul>
      ) : (
        rows && (
          <KeyValueTable rows={rows} onChange={setRows} keyPlaceholder="Variable" valueColumn={{ masked: !revealed }} />
        )
      )}
    </div>
  );

  if (inline) {
    return (
      <div className="flex min-w-0 flex-col" onKeyDown={onKeyDown}>
        <h3 className="mb-3 flex items-baseline gap-2 text-sm font-semibold">{title}</h3>
        {table}
        <div className="mt-3 flex flex-wrap justify-end gap-2">{buttons}</div>
      </div>
    );
  }
  return (
    <div className="flex min-w-0 flex-1 flex-col p-5" onKeyDown={onKeyDown}>
      <DialogHeader className="mb-4">
        <DialogTitle className="flex items-baseline gap-2">{title}</DialogTitle>
        <DialogDescription className="sr-only">The variables in {info.path}.</DialogDescription>
      </DialogHeader>
      {table}
      <DialogFooter className="mt-4">{buttons}</DialogFooter>
    </div>
  );
}
