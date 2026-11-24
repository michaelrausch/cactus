import type { Auth } from "@/api/client";
import { Field, Hint } from "@/components/Field";
import { Input } from "@/components/ui/input";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { AUTH_DEFAULTS, AUTH_KIND_LABELS, isAuthKind, type AuthKind } from "@/domain/http";
import { withAuth } from "@/domain/scenario";
import type { ScenarioTabProps } from "../types";

const AUTH_KINDS = Object.keys(AUTH_KIND_LABELS) as AuthKind[];
const SECRET_INPUT = "font-mono text-xs";

function AuthFields({ auth, onChange }: { auth: Auth; onChange: (auth: Auth) => void }) {
  if (auth.type === "bearer") {
    return (
      <Field label="Token">
        <Input
          className={SECRET_INPUT}
          value={auth.token}
          placeholder="{{token}}"
          onChange={(e) => onChange({ ...auth, token: e.target.value })}
        />
      </Field>
    );
  }
  return (
    <div className="grid grid-cols-2 gap-3">
      <Field label="Username">
        <Input
          className={SECRET_INPUT}
          value={auth.username}
          onChange={(e) => onChange({ ...auth, username: e.target.value })}
        />
      </Field>
      <Field label="Password">
        <Input
          className={SECRET_INPUT}
          value={auth.password}
          placeholder="{{password}}"
          onChange={(e) => onChange({ ...auth, password: e.target.value })}
        />
      </Field>
    </div>
  );
}

export function AuthTab({ scenario, onChange }: ScenarioTabProps) {
  const auth = scenario.auth ?? null;
  const setAuth = (next: Auth | null) => onChange(withAuth(scenario, next));
  const chooseKind = (kind: string) => {
    if (isAuthKind(kind)) setAuth(AUTH_DEFAULTS[kind]);
  };

  return (
    <>
      <Field label="Type">
        <Select value={auth?.type ?? "none"} onValueChange={chooseKind}>
          <SelectTrigger className="w-48">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {AUTH_KINDS.map((kind) => (
              <SelectItem key={kind} value={kind}>
                {AUTH_KIND_LABELS[kind]}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </Field>
      {auth && <AuthFields auth={auth} onChange={setAuth} />}
      <Hint>Use {"{{vars}}"} here and keep the real values in Secrets, which are never committed.</Hint>
    </>
  );
}
