import { useEffect, useState, type ReactNode } from "react";
import { LockKeyhole, Smartphone } from "lucide-react";
import type { TotpSetup } from "@/api/bindings";
import { disableTotp, enableTotp, refreshVaultStatus, setPassword, startTotp } from "@/actions/vault";
import { Button } from "@/components/ui/button";
import { reportErrors } from "@/lib/errors";
import { cn } from "@/lib/utils";
import { useVaultStore } from "@/stores/vaultStore";
import { CodeInput, ErrorLine, PasswordInput, PasswordSetup, StackedForm, mismatch, useRunner } from "./PasswordFields";

type Mode = "view" | "encrypt" | "change" | "decrypt" | "totpOn" | "totpOff";

/** Encryption of the project's secrets and captured values, and its authenticator code, set up in place. */
export function PasswordPanel({ project }: { project: string }) {
  const status = useVaultStore((s) => s.status);
  const totp = useVaultStore((s) => s.totp);
  const [mode, setMode] = useState<Mode>("view");
  useEffect(() => void reportErrors(() => refreshVaultStatus(project)), [project]);
  if (status !== "noPassword" && status !== "unlocked") return null;
  const view = () => setMode("view");
  const on = status === "unlocked";
  const idle = mode === "view";

  return (
    <div className="divide-y rounded-xl border bg-card/60">
      <Row icon={<LockKeyhole />} label="Encryption" on={on}>
        {idle &&
          (on ? (
            <>
              <Button variant="outline" size="sm" onClick={() => setMode("change")}>
                Change password
              </Button>
              <Button variant="ghost" size="sm" className="hover:text-danger" onClick={() => setMode("decrypt")}>
                Turn off
              </Button>
            </>
          ) : (
            <Button variant="outline" size="sm" onClick={() => setMode("encrypt")}>
              Set up
            </Button>
          ))}
      </Row>
      {on && (
        <Row icon={<Smartphone />} label="Authenticator" on={totp}>
          {idle && (
            <Button
              variant={totp ? "ghost" : "outline"}
              size="sm"
              className={cn(totp && "hover:text-danger")}
              onClick={() => setMode(totp ? "totpOff" : "totpOn")}
            >
              {totp ? "Turn off" : "Set up"}
            </Button>
          )}
        </Row>
      )}
      {!idle && (
        <div className="px-4 py-4">
          {mode === "totpOn" ? (
            <TotpSetupForm close={view} />
          ) : (
            <div className="mx-auto max-w-xs">
              {mode === "encrypt" && <EncryptForm close={view} />}
              {mode === "change" && <ChangeForm close={view} />}
              {mode === "decrypt" && <DecryptForm close={view} />}
              {mode === "totpOff" && <TotpOffForm close={view} />}
            </div>
          )}
        </div>
      )}
    </div>
  );
}

function Row({ icon, label, on, children }: { icon: ReactNode; label: string; on: boolean; children: ReactNode }) {
  return (
    <div className="flex items-center gap-3 px-4 py-2.5">
      <span className={cn("[&_svg]:size-4", on ? "text-brand-2" : "text-muted-foreground")}>{icon}</span>
      <span className="text-sm">{label}</span>
      <span
        className={cn(
          "rounded-full px-2 py-0.5 text-xs",
          on ? "bg-brand-2/15 text-brand-2" : "bg-muted text-muted-foreground",
        )}
      >
        {on ? "On" : "Off"}
      </span>
      <div className="ml-auto flex gap-2">{children}</div>
    </div>
  );
}

function Actions({ close, children }: { close: () => void; children: ReactNode }) {
  return (
    <div className="flex justify-end gap-2">
      <Button type="button" variant="ghost" size="sm" onClick={close}>
        Cancel
      </Button>
      {children}
    </div>
  );
}

function EncryptForm({ close }: { close: () => void }) {
  return (
    <div className="flex flex-col gap-2">
      <PasswordSetup onDone={close} autoFocus />
      <Button variant="ghost" size="sm" onClick={close}>
        Cancel
      </Button>
    </div>
  );
}

function ChangeForm({ close }: { close: () => void }) {
  const [current, setCurrent] = useState("");
  const [next, setNext] = useState("");
  const [confirm, setConfirm] = useState("");
  const { run, busy, error } = useRunner();
  const ready = !busy && current && next;
  return (
    <StackedForm onSubmit={() => ready && run(() => mismatch(next, confirm) ?? setPassword(current, next), close)}>
      <PasswordInput label="Current password" value={current} onChange={setCurrent} autoFocus />
      <PasswordInput label="New password" value={next} onChange={setNext} isNew />
      <PasswordInput label="Confirm" value={confirm} onChange={setConfirm} isNew />
      <ErrorLine error={error} />
      <Actions close={close}>
        <Button type="submit" size="sm" disabled={!ready}>
          Change password
        </Button>
      </Actions>
    </StackedForm>
  );
}

function DecryptForm({ close }: { close: () => void }) {
  const [current, setCurrent] = useState("");
  const { run, busy, error } = useRunner();
  const ready = !busy && current;
  return (
    <StackedForm onSubmit={() => ready && run(() => setPassword(current, null), close)}>
      <PasswordInput label="Password" value={current} onChange={setCurrent} autoFocus />
      <ErrorLine error={error} />
      <Actions close={close}>
        <Button type="submit" variant="destructive" size="sm" disabled={!ready}>
          Turn off encryption
        </Button>
      </Actions>
    </StackedForm>
  );
}

/** Scan or type in a fresh secret, then confirm with a code from the app. */
function TotpSetupForm({ close }: { close: () => void }) {
  const [setup, setSetup] = useState<TotpSetup | string | null>(null);
  const [code, setCode] = useState("");
  const { run, busy, error } = useRunner();
  // Once per opening: each start makes a new secret.
  useEffect(() => {
    let current = true;
    void startTotp().then((result) => current && setSetup(result));
    return () => {
      current = false;
    };
  }, []);
  if (setup === null) return null;
  if (typeof setup === "string") return <ErrorLine error={setup} />;
  const ready = !busy && code.trim().length >= 6;
  return (
    <div className="flex items-start gap-5">
      {/* Our own backend's SVG, from the `qrcode` crate. */}
      <div
        className="size-[180px] shrink-0 overflow-hidden rounded-lg [&_svg]:size-full"
        dangerouslySetInnerHTML={{ __html: setup.qrSvg }}
      />
      <StackedForm onSubmit={() => ready && run(() => enableTotp(setup.secret, code), close)}>
        <div className="flex flex-col gap-1.5">
          <span className="text-xs text-muted-foreground">Secret</span>
          <code className="rounded-md bg-muted px-2 py-1.5 font-mono text-xs break-all select-all">{setup.secret}</code>
        </div>
        <CodeInput value={code} onChange={setCode} autoFocus />
        <ErrorLine error={error} />
        <Actions close={close}>
          <Button type="submit" size="sm" disabled={!ready}>
            Turn on
          </Button>
        </Actions>
      </StackedForm>
    </div>
  );
}

function TotpOffForm({ close }: { close: () => void }) {
  const [code, setCode] = useState("");
  const { run, busy, error } = useRunner();
  const ready = !busy && code.trim().length >= 6;
  return (
    <StackedForm onSubmit={() => ready && run(() => disableTotp(code), close)}>
      <CodeInput value={code} onChange={setCode} autoFocus />
      <ErrorLine error={error} />
      <Actions close={close}>
        <Button type="submit" variant="destructive" size="sm" disabled={!ready}>
          Turn off
        </Button>
      </Actions>
    </StackedForm>
  );
}
