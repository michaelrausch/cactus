import { useEffect, useState } from "react";
import { X } from "lucide-react";
import { cancelGetToken, clearToken, getToken, loadTokenStatus } from "@/actions/oauth2";
import type { AuthMode, OAuth2Flow, TokenStatus } from "@/api/client";
import { IconButton } from "@/components/IconButton";
import { Button } from "@/components/ui/button";
import { EXPIRY_TICK_MS, expiryText } from "@/domain/jwt";
import { useNow } from "@/hooks/useNow";
import { useCritterStore } from "@/stores/critterStore";
import { useProjectStore } from "@/stores/projectStore";
import { ChoiceField, TextField } from "./AuthFields";

type OAuth2Mode = Extract<AuthMode, { type: "oauth2" }>;

const GRANTS: Record<OAuth2Flow, string> = {
  client_credentials: "Client credentials",
  resource_owner_password_credentials: "Password",
  authorization_code: "Authorization code",
};

const CLIENT_AUTH = { basic_auth_header: "Header", body: "Body" } as const;

const PKCE = { S256: "S256", plain: "Plain", off: "Off" } as const;
type PkceChoice = keyof typeof PKCE;

/** How long a status lookup waits for typing to stop. */
const STATUS_DEBOUNCE_MS = 400;

function pkceChoice(auth: OAuth2Mode): PkceChoice {
  if (!auth.pkce || auth.pkce.disabled) return "off";
  return auth.pkce.method ?? "S256";
}

function withPkce(auth: OAuth2Mode, choice: PkceChoice): OAuth2Mode {
  const { pkce: _previous, ...rest } = auth;
  return choice === "off" ? rest : { ...rest, pkce: { method: choice } };
}

export function OAuth2Fields({ auth, onChange }: { auth: OAuth2Mode; onChange: (auth: AuthMode) => void }) {
  const credentials = auth.credentials ?? { clientId: "" };
  const owner = auth.resourceOwner ?? { username: "", password: "" };
  const setCredentials = (change: Partial<typeof credentials>) =>
    onChange({ ...auth, credentials: { ...credentials, ...change } });
  const setOwner = (change: Partial<typeof owner>) => onChange({ ...auth, resourceOwner: { ...owner, ...change } });
  const code = auth.flow === "authorization_code";

  return (
    <div className="flex flex-col gap-3">
      <div className="grid grid-cols-[auto_auto_1fr] items-end gap-3">
        <ChoiceField
          label="Grant"
          className="w-48"
          value={auth.flow}
          options={GRANTS}
          onChange={(flow) => onChange({ ...auth, flow })}
        />
        <ChoiceField
          label="Client auth"
          value={credentials.placement ?? "basic_auth_header"}
          options={CLIENT_AUTH}
          onChange={(placement) => setCredentials({ placement })}
        />
        {code && (
          <ChoiceField
            label="PKCE"
            value={pkceChoice(auth)}
            options={PKCE}
            onChange={(choice) => onChange(withPkce(auth, choice))}
          />
        )}
      </div>
      {code && (
        <TextField
          label="Auth URL"
          value={auth.authorizationUrl ?? ""}
          onChange={(authorizationUrl) => onChange({ ...auth, authorizationUrl })}
        />
      )}
      <TextField
        label="Token URL"
        value={auth.accessTokenUrl ?? ""}
        onChange={(accessTokenUrl) => onChange({ ...auth, accessTokenUrl })}
      />
      {code && (
        <TextField
          label="Callback URL"
          value={auth.callbackUrl ?? ""}
          placeholder="http://localhost:8765/callback"
          onChange={(callbackUrl) => onChange({ ...auth, callbackUrl })}
        />
      )}
      <div className="grid grid-cols-2 gap-3">
        <TextField
          label="Client ID"
          value={credentials.clientId}
          onChange={(clientId) => setCredentials({ clientId })}
        />
        <TextField
          label="Client secret"
          value={credentials.clientSecret ?? ""}
          placeholder="{{clientSecret}}"
          onChange={(clientSecret) => setCredentials({ clientSecret })}
        />
      </div>
      {auth.flow === "resource_owner_password_credentials" && (
        <div className="grid grid-cols-2 gap-3">
          <TextField label="Username" value={owner.username} onChange={(username) => setOwner({ username })} />
          <TextField
            label="Password"
            value={owner.password}
            placeholder="{{password}}"
            onChange={(password) => setOwner({ password })}
          />
        </div>
      )}
      <TextField label="Scope" value={auth.scope ?? ""} onChange={(scope) => onChange({ ...auth, scope })} />
      <TokenRow auth={auth} />
    </div>
  );
}

function tokenText(status: TokenStatus | null, now: number): string {
  if (!status?.hasToken) return "No token";
  if (status.expiresAt === null) return "Token";
  return `Token ${expiryText(status.expiresAt, now)}`;
}

/** The cached token for the active env and critter, and getting or clearing it. */
function TokenRow({ auth }: { auth: OAuth2Mode }) {
  const activeEnv = useProjectStore((s) => s.activeEnv);
  const activeCritter = useCritterStore((s) => s.activeId);
  const [status, setStatus] = useState<TokenStatus | null>(null);
  const [getting, setGetting] = useState(false);
  const now = useNow(EXPIRY_TICK_MS);
  // The cached token depends on the settings, so look again as they change.
  const settings = JSON.stringify(auth);

  useEffect(() => {
    let current = true;
    const timer = setTimeout(() => {
      void loadTokenStatus().then((found) => current && setStatus(found ?? null));
    }, STATUS_DEBOUNCE_MS);
    return () => {
      current = false;
      clearTimeout(timer);
    };
    // The backend reads the auth, env and critter itself; look again when they change.
    // oxlint-disable-next-line react/exhaustive-effect-dependencies
  }, [settings, activeEnv, activeCritter]);

  const get = async () => {
    setGetting(true);
    const got = await getToken();
    setGetting(false);
    if (got) setStatus(got);
  };
  const clear = async () => {
    if (await clearToken()) setStatus(await loadTokenStatus().then((found) => found ?? null));
  };

  return (
    <div className="flex items-center gap-2 pt-1 text-sm">
      <span className="text-muted-foreground">{tokenText(status, now)}</span>
      {status?.hasToken && (
        <IconButton label="Clear token" onClick={() => void clear()}>
          <X />
        </IconButton>
      )}
      {getting && auth.flow === "authorization_code" ? (
        <Button size="sm" variant="outline" className="ml-auto" onClick={() => void cancelGetToken()}>
          Waiting for browser… Cancel
        </Button>
      ) : (
        <Button size="sm" variant="outline" className="ml-auto" disabled={getting} onClick={() => void get()}>
          Get token
        </Button>
      )}
    </div>
  );
}
