---
title: AI agents (MCP)
description: Let an AI agent send requests and run flows through cactus.
---

cactus runs an [MCP](https://modelcontextprotocol.io) server for the project open in the app. Agents send requests with your secrets but never get the values back; secrets are masked in every result.

## Setup

**Settings** (<kbd>Ctrl</kbd>+<kbd>,</kbd>) → **MCP** → **Enabled**. Default URL: `http://127.0.0.1:47821/mcp`.

**Copy Claude Code command**:

```sh
claude mcp add --transport http cactus http://127.0.0.1:47821/mcp \
  --header "Authorization: Bearer <token>"
```

**Copy config**:

```json
{
  "mcpServers": {
    "cactus": {
      "type": "http",
      "url": "http://127.0.0.1:47821/mcp",
      "headers": { "Authorization": "Bearer <token>" }
    }
  }
}
```

The token is stored in the app's config folder, not in the project. **New token** replaces it.

## Tools

| Tool | |
| --- | --- |
| `list_requests` | Requests in tree order |
| `get_request` | One request, as JSON |
| `list_environments` | Environments and which is active |
| `list_critters` | Bots and their flows |
| `get_flow` | A flow's YAML |
| `get_variables` | Variables a request sees and their sources; secret values hidden |
| `get_history` | A request's recent responses |
| `send_request` | Send a request; optional `env`, `critter` |
| `run_folder` | Send every request in a folder |
| `run_flow` | Run a flow; `vars` answers prompts |

With **Allow edits**:

| Tool | |
| --- | --- |
| `create_request` | New request from title, method, URL |
| `save_request` | Save a request; only changed fields are rewritten |
| `save_flow` | Write a flow's YAML; invalid flows are refused |

Sends made by an agent record history and save captures, the same as sends from the app. Prompts that `vars` doesn't answer appear in the app.

## Security

- Listens on `127.0.0.1` only.
- Every request needs the bearer token.
- Requests with an `Origin` header are refused, so web pages can't reach it.
- Editing tools are off until **Allow edits** is on.

## Example prompts

- Run the admin smoke flow on staging and tell me which step fails.
- Create requests for the endpoints in `src/routes/invoices.ts` in an `invoices` folder.
- Send `orders/list.yml` as viewer and as admin and compare the results.
