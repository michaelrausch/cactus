import { useEffect, useId, type ReactNode } from "react";
import { ensureUnlocked, lockSecrets, refreshSecrets, setAutoLock } from "@/actions/secrets";
import { Button } from "@/components/ui/button";
import { AutoLockSelect } from "@/features/secrets/AutoLockSelect";
import { useSecretsStore, type SecretsDialog } from "@/stores/secretsStore";

const open = (dialog: SecretsDialog) => useSecretsStore.getState().setDialog(dialog);

/** A labelled setting, laid out like the Git tab's fields. */
function Field({ label, id, children }: { label: string; id?: string; children: ReactNode }) {
  return (
    <div className="flex flex-col gap-1.5">
      <label htmlFor={id} className="text-sm font-medium">
        {label}
      </label>
      <div className="flex flex-wrap items-center gap-2">{children}</div>
    </div>
  );
}

/** Secret encryption for the open project on this machine. */
export function SecurityTab() {
  const info = useSecretsStore((s) => s.info);
  const autoLockId = useId();
  useEffect(() => void refreshSecrets(), []);
  if (!info) return null;
  const on = info.status !== "off";
  const locked = info.status === "locked";

  return (
    <section className="flex max-w-md flex-col gap-4">
      <h3 className="text-xs font-medium tracking-wide text-muted-foreground uppercase">Secret encryption</h3>
      <Field label="Encryption">
        <Button variant="outline" onClick={() => open(on ? "disable" : "enable")}>
          {on ? "Turn off…" : "Turn on…"}
        </Button>
        {on && (
          <Button
            variant="outline"
            onClick={() => (locked ? void ensureUnlocked({ force: true }) : void lockSecrets())}
          >
            {locked ? "Unlock…" : "Lock now"}
          </Button>
        )}
      </Field>
      {on && (
        <>
          <Field label="Lock when idle" id={autoLockId}>
            <AutoLockSelect id={autoLockId} value={info.autoLockMinutes} onChange={(m) => void setAutoLock(m)} />
          </Field>
          <Field label="Password">
            <Button variant="outline" onClick={() => open("changePassword")}>
              Change…
            </Button>
            <Button variant="ghost" className="hover:text-danger" onClick={() => open("reset")}>
              Forgot password…
            </Button>
          </Field>
        </>
      )}
    </section>
  );
}
