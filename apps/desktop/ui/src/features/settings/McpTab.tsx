import { useEffect, useState, type ReactNode } from "react";
import { isTauri } from "@tauri-apps/api/core";
import { Copy, RotateCw, Terminal } from "lucide-react";
import { copyText } from "@/actions/clipboard";
import { loadMcpStatus, regenerateMcpToken, saveMcpSettings } from "@/actions/mcp";
import type { McpStatus } from "@/api/client";
import { IconButton } from "@/components/IconButton";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { claudeCodeCommand, mcpConfigJson, parsePort } from "@/domain/mcp";

function Row({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="flex min-h-11 items-center gap-3 px-4 py-1.5 text-sm">
      <span className="flex-1">{label}</span>
      {children}
    </div>
  );
}

function Toggle({ label, checked, onChange }: { label: string; checked: boolean; onChange: (on: boolean) => void }) {
  return (
    <input
      type="checkbox"
      aria-label={label}
      checked={checked}
      onChange={(event) => onChange(event.target.checked)}
      className="size-4 accent-(--brand-2)"
    />
  );
}

/** Mounted per saved port, so it starts from that port. */
function PortInput({ status, onSave }: { status: McpStatus; onSave: (port: number) => void }) {
  const [text, setText] = useState(String(status.port));
  const port = parsePort(text);
  const commit = () => {
    if (port !== null && port !== status.port) onSave(port);
  };
  return (
    <Input
      aria-label="Port"
      title="1024–65535"
      aria-invalid={port === null}
      value={text}
      inputMode="numeric"
      onChange={(event) => setText(event.target.value)}
      onBlur={commit}
      onKeyDown={(event) => event.key === "Enter" && commit()}
      className="h-8 w-24 font-mono text-xs"
    />
  );
}

/** The MCP server agents use to reach the open project. */
export function McpTab() {
  const [status, setStatus] = useState<McpStatus | null>(null);
  useEffect(() => {
    if (!isTauri()) return;
    void loadMcpStatus().then((loaded) => loaded && setStatus(loaded));
  }, []);
  if (!status) return null;

  const save = async (change: Partial<Pick<McpStatus, "enabled" | "allowEdits" | "port">>) => {
    const next = { ...status, ...change };
    const saved = await saveMcpSettings(next.enabled, next.allowEdits, next.port);
    if (saved) setStatus(saved);
  };
  const regenerate = async () => {
    const saved = await regenerateMcpToken();
    if (saved) setStatus(saved);
  };

  return (
    <section className="flex max-w-md flex-col gap-2">
      <h3 className="text-xs font-medium tracking-wide text-muted-foreground uppercase">MCP server</h3>
      <div className="flex flex-col divide-y overflow-hidden rounded-xl border bg-card/60">
        <Row label="Enabled">
          <Toggle label="Enabled" checked={status.enabled} onChange={(enabled) => void save({ enabled })} />
        </Row>
        <Row label="Allow edits">
          <Toggle
            label="Allow edits"
            checked={status.allowEdits}
            onChange={(allowEdits) => void save({ allowEdits })}
          />
        </Row>
        <Row label="Port">
          <PortInput key={status.port} status={status} onSave={(port) => void save({ port })} />
        </Row>
        <Row label="URL">
          <span className="font-mono text-xs text-muted-foreground">{status.url}</span>
          <IconButton label="Copy URL" onClick={() => copyText(status.url, "URL copied")}>
            <Copy />
          </IconButton>
        </Row>
        <Row label="Token">
          <span className="font-mono text-xs text-muted-foreground">••••{status.token.slice(-4)}</span>
          <IconButton label="Copy token" onClick={() => copyText(status.token, "Token copied")}>
            <Copy />
          </IconButton>
          <IconButton label="New token" onClick={() => void regenerate()}>
            <RotateCw />
          </IconButton>
        </Row>
      </div>
      {status.error && <p className="text-xs text-danger">{status.error}</p>}
      <div className="flex gap-1">
        <Button variant="ghost" size="sm" onClick={() => copyText(mcpConfigJson(status), "Config copied")}>
          <Copy /> Copy config
        </Button>
        <Button variant="ghost" size="sm" onClick={() => copyText(claudeCodeCommand(status), "Command copied")}>
          <Terminal /> Copy Claude Code command
        </Button>
      </div>
    </section>
  );
}
