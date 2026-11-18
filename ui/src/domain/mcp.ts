/** What an MCP client needs to reach the app's server. */
export interface McpEndpoint {
  url: string;
  token: string;
}

/** The `mcpServers` entry most MCP clients read from their JSON config. */
export function mcpConfigJson({ url, token }: McpEndpoint): string {
  const config = { mcpServers: { cactus: { type: "http", url, headers: { Authorization: `Bearer ${token}` } } } };
  return JSON.stringify(config, null, 2);
}

/** Adds the server to Claude Code. */
export function claudeCodeCommand({ url, token }: McpEndpoint): string {
  return `claude mcp add --transport http cactus ${url} --header "Authorization: Bearer ${token}"`;
}

/** A port typed into settings, or null if it isn't one an unprivileged server can use. */
export function parsePort(text: string): number | null {
  const trimmed = text.trim();
  if (!/^\d+$/.test(trimmed)) return null;
  const port = Number(trimmed);
  return port >= 1024 && port <= 65535 ? port : null;
}
