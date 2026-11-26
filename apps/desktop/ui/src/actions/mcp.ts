import { backend, type McpStatus } from "@/api/client";
import { reportErrors, warnQuietly } from "@/lib/errors";

export function loadMcpStatus(): Promise<McpStatus | undefined> {
  return warnQuietly(() => backend.mcpStatus(), "Couldn't read MCP settings");
}

/** Save and restart the server to match. */
export function saveMcpSettings(enabled: boolean, allowEdits: boolean, port: number): Promise<McpStatus | undefined> {
  return reportErrors(() => backend.setMcpSettings(enabled, allowEdits, port), "Couldn't save");
}

export function regenerateMcpToken(): Promise<McpStatus | undefined> {
  return reportErrors(() => backend.regenerateMcpToken(), "Couldn't make a new token");
}
