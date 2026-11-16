import { useId, useState, type ReactNode } from "react";
import { setPassword } from "@/actions/vault";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";

/** Runs work that resolves to an error to show, or null when done. */
export function useRunner() {
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const run = (work: () => Promise<string | null> | string, onDone?: () => void) => {
    setBusy(true);
    void Promise.resolve(work()).then((problem) => {
      setBusy(false);
      setError(problem);
      if (!problem) onDone?.();
    });
  };
  return { run, busy, error };
}

export const mismatch = (next: string, confirm: string): string | null =>
  next === confirm ? null : "Passwords don't match";

export function PasswordInput({
  label,
  value,
  onChange,
  isNew,
  autoFocus,
}: {
  label: string;
  value: string;
  onChange: (value: string) => void;
  isNew?: boolean;
  autoFocus?: boolean;
}) {
  const id = useId();
  return (
    <div className="flex flex-col gap-1.5">
      <label htmlFor={id} className="text-xs text-muted-foreground">
        {label}
      </label>
      <Input
        id={id}
        type="password"
        value={value}
        autoFocus={autoFocus}
        autoComplete={isNew ? "new-password" : "current-password"}
        spellCheck={false}
        onChange={(event) => onChange(event.target.value)}
      />
    </div>
  );
}

/** A 6-digit authenticator code. */
export function CodeInput({
  value,
  onChange,
  autoFocus,
}: {
  value: string;
  onChange: (value: string) => void;
  autoFocus?: boolean;
}) {
  const id = useId();
  return (
    <div className="flex flex-col gap-1.5">
      <label htmlFor={id} className="text-xs text-muted-foreground">
        Authenticator code
      </label>
      <Input
        id={id}
        value={value}
        autoFocus={autoFocus}
        inputMode="numeric"
        autoComplete="one-time-code"
        placeholder="000000"
        maxLength={7}
        className="font-mono tracking-widest"
        onChange={(event) => onChange(event.target.value)}
      />
    </div>
  );
}

export function ErrorLine({ error }: { error: string | null }) {
  return error ? (
    <p role="alert" className="text-sm text-danger">
      {error}
    </p>
  ) : null;
}

/** A stacked form that submits on Enter. */
export function StackedForm({ onSubmit, children }: { onSubmit: () => void; children: ReactNode }) {
  return (
    <form
      className="flex w-full flex-col gap-3"
      onSubmit={(event) => {
        event.preventDefault();
        onSubmit();
      }}
    >
      {children}
    </form>
  );
}

/** Choose a first password, which turns on encryption. */
export function PasswordSetup({ onDone, autoFocus = false }: { onDone?: () => void; autoFocus?: boolean }) {
  const [next, setNext] = useState("");
  const [confirm, setConfirm] = useState("");
  const { run, busy, error } = useRunner();
  const submit = () => run(() => mismatch(next, confirm) ?? setPassword(null, next), onDone);
  return (
    <StackedForm onSubmit={() => !busy && next && submit()}>
      <PasswordInput label="Password" value={next} onChange={setNext} isNew autoFocus={autoFocus} />
      <PasswordInput label="Confirm" value={confirm} onChange={setConfirm} isNew />
      <ErrorLine error={error} />
      <Button type="submit" className="mt-1" disabled={busy || !next}>
        Encrypt secrets
      </Button>
      <p className="text-center text-xs text-muted-foreground">A forgotten password can't be recovered.</p>
    </StackedForm>
  );
}
