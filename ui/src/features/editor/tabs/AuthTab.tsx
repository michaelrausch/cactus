import type { AuthMode } from "@/api/client";
import { Field } from "@/components/Field";
import { TemplateField } from "@/components/template/TemplateField";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { AUTH_KIND_LABELS, AUTH_KINDS, isAuthKind, type AuthKind } from "@/domain/http";
import { authKind, authMode, otherAuthType, withAuth } from "@/domain/scenario";
import type { ScenarioTabProps } from "../types";

/** Looks like `Input`; the border is on the box so the text inside lines up with its colours. */
const FIELD_BOX =
  "rounded-lg border border-input bg-input/30 transition-colors focus-within:border-ring focus-within:ring-3 focus-within:ring-ring/50";
/** `Input`'s height less its border. */
const FIELD_TEXT = "h-[calc(2rem-2px)] px-2.5 py-1 font-mono text-xs md:text-sm";

/** What a freshly chosen auth kind starts as; `none` removes auth. */
const AUTH_DEFAULTS: Record<AuthKind, AuthMode | "inherit" | undefined> = {
  none: undefined,
  inherit: "inherit",
  bearer: { type: "bearer", token: "" },
  basic: { type: "basic", username: "", password: "" },
  apikey: { type: "apikey", key: "", value: "", placement: "header" },
};

function TextField({
  label,
  value,
  placeholder,
  onChange,
}: {
  label: string;
  value: string;
  placeholder?: string;
  onChange: (value: string) => void;
}) {
  return (
    <Field label={label}>
      <TemplateField
        className={FIELD_BOX}
        fieldClassName={FIELD_TEXT}
        value={value}
        placeholder={placeholder}
        aria-label={label}
        onChange={onChange}
      />
    </Field>
  );
}

function AuthFields({ auth, onChange }: { auth: AuthMode; onChange: (auth: AuthMode) => void }) {
  switch (auth.type) {
    case "bearer":
      return (
        <TextField
          label="Token"
          value={auth.token}
          placeholder="{{token}}"
          onChange={(token) => onChange({ ...auth, token })}
        />
      );
    case "basic":
      return (
        <div className="grid grid-cols-2 gap-3">
          <TextField label="Username" value={auth.username} onChange={(username) => onChange({ ...auth, username })} />
          <TextField
            label="Password"
            value={auth.password}
            placeholder="{{password}}"
            onChange={(password) => onChange({ ...auth, password })}
          />
        </div>
      );
    case "apikey":
      return (
        <div className="grid grid-cols-[1fr_1fr_auto] items-end gap-3">
          <TextField
            label="Key"
            value={auth.key}
            placeholder="X-API-Key"
            onChange={(key) => onChange({ ...auth, key })}
          />
          <TextField
            label="Value"
            value={auth.value}
            placeholder="{{apiKey}}"
            onChange={(value) => onChange({ ...auth, value })}
          />
          <Field label="Add to">
            <Select
              value={auth.placement ?? "header"}
              onValueChange={(placement) =>
                onChange({ ...auth, placement: placement === "query" ? "query" : "header" })
              }
            >
              <SelectTrigger className="w-28">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="header">Header</SelectItem>
                <SelectItem value="query">Query</SelectItem>
              </SelectContent>
            </Select>
          </Field>
        </div>
      );
  }
}

export function AuthTab({ scenario, onChange }: ScenarioTabProps) {
  const kind = authKind(scenario);
  const mode = authMode(scenario);
  const chooseKind = (next: string) => {
    if (isAuthKind(next)) onChange(withAuth(scenario, AUTH_DEFAULTS[next]));
  };

  return (
    <>
      <Field label="Type">
        <Select value={kind} onValueChange={chooseKind}>
          <SelectTrigger className="w-48">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {AUTH_KINDS.map((option) => (
              <SelectItem key={option} value={option}>
                {AUTH_KIND_LABELS[option]}
              </SelectItem>
            ))}
            {kind === "other" && <SelectItem value="other">{otherAuthType(scenario)} (kept as written)</SelectItem>}
          </SelectContent>
        </Select>
      </Field>
      {mode && <AuthFields auth={mode} onChange={(next) => onChange(withAuth(scenario, next))} />}
    </>
  );
}
