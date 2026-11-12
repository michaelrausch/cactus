import { useState, type ReactNode } from "react";
import {
  changePassword,
  disableEncryption,
  enableEncryption,
  forgotPassword,
  resetSecrets,
  unlockDialogClosed,
  unlockSecrets,
} from "@/actions/secrets";
import { Field } from "@/components/Field";
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
import { newPasswordProblem } from "@/domain/password";
import { baseName } from "@/domain/paths";
import { useProjectStore } from "@/stores/projectStore";
import { useSecretsStore, type SecretsDialog } from "@/stores/secretsStore";
import { AutoLockSelect } from "./AutoLockSelect";

const DEFAULT_AUTO_LOCK = 15;

/** Every secrets dialog, one open at a time; see `useSecretsStore().dialog`. */
export function SecretsDialogs() {
  const dialog = useSecretsStore((s) => s.dialog);
  const close = () => {
    if (dialog === "unlock") unlockDialogClosed(false);
    else useSecretsStore.getState().setDialog(null);
  };
  return (
    <Dialog open={dialog !== null} onOpenChange={(open) => !open && close()}>
      <DialogContent className="sm:max-w-sm">{dialog && <DialogBody dialog={dialog} close={close} />}</DialogContent>
    </Dialog>
  );
}

function DialogBody({ dialog, close }: { dialog: SecretsDialog; close: () => void }) {
  switch (dialog) {
    case "enable":
      return <EnableForm close={close} />;
    case "unlock":
      return <UnlockForm />;
    case "changePassword":
      return <ChangePasswordForm close={close} />;
    case "disable":
      return <DisableForm close={close} />;
    case "reset":
      return <ResetForm close={close} />;
  }
}

function PasswordInput({
  value,
  onChange,
  autoFocus,
  autoComplete = "current-password",
}: {
  value: string;
  onChange: (value: string) => void;
  autoFocus?: boolean;
  autoComplete?: "current-password" | "new-password";
}) {
  return (
    <Input
      type="password"
      value={value}
      autoFocus={autoFocus}
      autoComplete={autoComplete}
      spellCheck={false}
      onChange={(event) => onChange(event.target.value)}
    />
  );
}

/** A form that runs `submit` and shows its error in place, so the password can be fixed and retried. */
function PasswordForm({
  title,
  description,
  children,
  action,
  destructive,
  submit,
  aside,
}: {
  title: string;
  description?: ReactNode;
  children: ReactNode;
  action: string;
  destructive?: boolean;
  /** Resolves to an error to show, or `null` when done. */
  submit: () => Promise<string | null> | string | null;
  aside?: ReactNode;
}) {
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  return (
    <form
      className="flex flex-col gap-4"
      onSubmit={(event) => {
        event.preventDefault();
        if (busy) return;
        setBusy(true);
        void Promise.resolve(submit()).then((problem) => {
          setBusy(false);
          setError(problem);
        });
      }}
    >
      <DialogHeader>
        <DialogTitle>{title}</DialogTitle>
        {description ? (
          <DialogDescription asChild>
            <div>{description}</div>
          </DialogDescription>
        ) : (
          <DialogDescription className="sr-only">{title}</DialogDescription>
        )}
      </DialogHeader>
      {children}
      {error && (
        <p role="alert" className="text-sm text-danger">
          {error}
        </p>
      )}
      <DialogFooter className="items-center">
        {aside && <div className="sm:mr-auto">{aside}</div>}
        <Button type="submit" variant={destructive ? "destructive" : "default"} disabled={busy}>
          {busy ? "Working…" : action}
        </Button>
      </DialogFooter>
    </form>
  );
}

function EnableForm({ close }: { close: () => void }) {
  const [password, setPassword] = useState("");
  const [confirm, setConfirm] = useState("");
  const [autoLock, setAutoLock] = useState<number | null>(DEFAULT_AUTO_LOCK);
  return (
    <PasswordForm
      title="Encrypt secrets"
      description="A forgotten password can't be recovered."
      action="Encrypt"
      submit={async () => {
        const problem = newPasswordProblem(password, confirm) ?? (await enableEncryption(password, autoLock));
        if (!problem) close();
        return problem;
      }}
    >
      <Field label="Password">
        <PasswordInput value={password} onChange={setPassword} autoFocus autoComplete="new-password" />
      </Field>
      <Field label="Confirm">
        <PasswordInput value={confirm} onChange={setConfirm} autoComplete="new-password" />
      </Field>
      <Field label="Lock when idle">
        <AutoLockSelect value={autoLock} onChange={setAutoLock} />
      </Field>
    </PasswordForm>
  );
}

function UnlockForm() {
  const [password, setPassword] = useState("");
  return (
    <PasswordForm
      title="Unlock secrets"
      action="Unlock"
      submit={async () => {
        const problem = await unlockSecrets(password);
        if (problem) return problem;
        // A password change was interrupted, so some values are still under the old one.
        if (useSecretsStore.getState().info?.status === "locked") {
          setPassword("");
          return "Now enter your previous password";
        }
        unlockDialogClosed(true);
        return null;
      }}
      aside={
        <Button type="button" variant="link" className="px-0 text-muted-foreground" onClick={forgotPassword}>
          Forgot password?
        </Button>
      }
    >
      <Field label="Password">
        <PasswordInput value={password} onChange={setPassword} autoFocus />
      </Field>
    </PasswordForm>
  );
}

function ChangePasswordForm({ close }: { close: () => void }) {
  const [current, setCurrent] = useState("");
  const [next, setNext] = useState("");
  const [confirm, setConfirm] = useState("");
  return (
    <PasswordForm
      title="Change password"
      action="Change"
      submit={async () => {
        const problem = newPasswordProblem(next, confirm) ?? (await changePassword(current, next));
        if (!problem) close();
        return problem;
      }}
    >
      <Field label="Current password">
        <PasswordInput value={current} onChange={setCurrent} autoFocus />
      </Field>
      <Field label="New password">
        <PasswordInput value={next} onChange={setNext} autoComplete="new-password" />
      </Field>
      <Field label="Confirm">
        <PasswordInput value={confirm} onChange={setConfirm} autoComplete="new-password" />
      </Field>
    </PasswordForm>
  );
}

function DisableForm({ close }: { close: () => void }) {
  const [password, setPassword] = useState("");
  return (
    <PasswordForm
      title="Turn off encryption"
      action="Turn off"
      destructive
      submit={async () => {
        const problem = await disableEncryption(password);
        if (!problem) close();
        return problem;
      }}
    >
      <Field label="Password">
        <PasswordInput value={password} onChange={setPassword} autoFocus />
      </Field>
    </PasswordForm>
  );
}

function ResetForm({ close }: { close: () => void }) {
  const project = useProjectStore((s) => s.project);
  const name = project ? baseName(project) : "";
  const [typed, setTyped] = useState("");
  return (
    <PasswordForm
      title="Reset secrets"
      description={`Deletes all encrypted values. Type ${name} to confirm.`}
      action="Reset"
      destructive
      submit={async () => {
        if (typed.trim() !== name) return `Type ${name} to confirm.`;
        const problem = await resetSecrets();
        if (!problem) close();
        return problem;
      }}
    >
      <Input value={typed} autoFocus spellCheck={false} placeholder={name} onChange={(e) => setTyped(e.target.value)} />
    </PasswordForm>
  );
}
