# MCP server

Status: plan. Nothing here is built yet.

`cactus-cli mcp <project>` serves one cactus project over the
[Model Context Protocol](https://modelcontextprotocol.io) on stdio. A coding
agent (Claude Code, an IDE assistant) can then work in the project the way a
person does in the app. It lists and reads requests, sends them with the
project's environments and secrets, checks their assertions, and writes new
requests that a person reviews in git like any other change.

It's a thin adapter over `core`, like `src-tauri/src/commands/`. Projects and
people that don't use it see no change.

> **Re-validate before building.** This plan was written against commit
> `a43d433` ("Implement UI audit results"), with the critters and
> secrets-encryption plans unbuilt. The names, signatures and behaviour quoted
> here are a snapshot, not a contract. Before each phase, check what it relies
> on (§0) against the current code. Where the code has moved on, follow the
> code and update this plan. §10 says what changes if critters or encryption
> have landed.

## 0. Assumptions to re-check

Run `git log a43d433..HEAD --stat` and read each file below that appears in it.

| Assumption | Where | Phase |
| --- | --- | --- |
| The CLI is clap with `run`, `send` and `import`, `Outcome` sets the exit code, and tokio is `current_thread` with features `rt`, `macros` | `cli/src/args.rs`, `cli/src/main.rs`, `cli/Cargo.toml` | 2 |
| `runner::send_and_record(client, project, path, scenario, env_name)` builds, sends, merges captures and records history; `runner::run` takes an `on_result` callback | `core/src/runner.rs` | 3 |
| `http::send` calls `build_request`, which returns a `PreparedRequest { request, unresolved, warnings }` before anything goes out | `core/src/http/send.rs`, `core/src/http/request.rs` | 1, 3 |
| Attachment and file bodies are read from `project.join(file)`, so an absolute or `..` path leaves the project | `core/src/http/request.rs` (~l. 228, 239) | 1 |
| `resolve_for_request` merges collection → env → folders → request vars → secrets → captured, and is the only merge | `core/src/env/layers.rs` | 1 |
| Secrets live in `.cactus/secrets.json` and captured values in `.cactus/captured.json`, both flat `Env`s read through `env::load` | `core/src/env/layers.rs`, `core/src/layout.rs` | 1 |
| `Subject::parse`/`select` handle `res.*` expressions and are `pub(crate)` | `core/src/http/expression.rs` | 1 |
| `Response` has `url`, `headers`, `body`, `truncated`, `checks`, `captured`, `capture_failures`, `unresolved`, `warnings`; bodies are cut at 5 MiB | `core/src/http/response.rs` | 1, 3 |
| `scenario::store::save` patches in place; `create(project, dir, scenario)` picks the file name from the title and returns the path | `core/src/scenario/store.rs` | 4 |
| `collection::read` parses a request file into `Scenario`; there is no parse-from-string helper yet | `core/src/collection/file.rs` | 4 |
| `import::import(project, file)` reads the file itself | `core/src/import/mod.rs` | 4 |
| `history::load` returns the whole `Response` per entry | `core/src/history.rs` | 2 |
| The desktop watcher reloads files changed on disk and flags conflicts with dirty buffers | `src-tauri/src/watcher.rs`, `ui/src/actions/fsSync.ts` | — |

If an assumption no longer holds:

- **Renamed or moved:** follow it, and update the names in this plan.
- **Changed behaviour** (for example, a new env layer): redesign the affected
  section here before writing code, and say so in the phase's PR.
- **Something this plan adds already exists** (for example,
  `sources_for_request` or `select_values` from the critters plan): reuse it,
  and drop that part of the plan.

## 1. Goals, non-goals and threat model

**Goals**

- An agent can answer "why does this request fail?" without a person copying
  responses into chat. It gets the status, the failed checks, unresolved
  variables, warnings and the relevant part of the body.
- An agent can add and edit requests as Bruno-compatible YAML, saved the way
  the app saves them (in place, comments kept).
- An agent can run a folder against an environment after changing an API, and
  get the same pass/fail as `cactus-cli run`.
- **Values that live only in `.cactus/` never leave the server.** Secrets and
  captured values are substituted into outgoing requests, but never appear in
  a tool result.
- Each tool tells the client whether it reads, writes, or talks to the
  network, so clients can auto-approve the safe ones.

**Non-goals (for now)**

- Deleting or renaming requests, editing environments, or writing secrets.
  A person does that in the app.
- Changing the active environment. That's the app's state (`state.json`), so
  tools take `env` per call instead.
- An HTTP/SSE transport, or MCP built into the desktop app (§11).
- Git tools. The agent already has git, and the requests are files.
- Several projects per server. One server serves one project, fixed at start.

**What it protects against**

The agent is trusted about as much as a colleague with access to the repo but
not to your password manager. What it reads from response bodies is
**untrusted**: an API response can contain text written to steer an agent.
So the server guards against:

- **Secret disclosure in results:** secret and captured values echoed by an
  API (httpbin does this), shown in a URL, or put in a header are redacted
  (§4.3).
- **Secret exfiltration through requests:** an agent, possibly steered by a
  response, writes a request that sends `{{apiKey}}` to a host it controls.
  Sends are allowed only to hosts the project already uses (§4.2).
- **File exfiltration through attachments:** a request whose multipart or file
  body points at `~/.ssh/id_rsa`. In MCP mode, files outside the project are
  refused (§4.2).
- **Accidental side effects:** `DELETE` and `POST` against a real system.
  By default only safe methods are sent (§4.2).
- **Escaping the project:** every path goes through `ScenarioPath`, and the
  import tool reads only files inside the project.

**What it doesn't protect against**

- An agent that also has a shell. It can `cat .cactus/secrets.json` directly.
  This server doesn't make that worse, and encrypted secrets (see
  [that plan](secrets-encryption-plan.md)) are the fix for it. The README
  section says so.
- Secrets shorter than the redaction minimum (§4.3), or values transformed
  before being echoed (base64, URL-encoded). Redaction is best effort; the
  host allowlist is the real boundary.
- A person passing `--allow-host '*'` or `--allow-unsafe`.

## 2. Running it

```sh
cactus-cli mcp <project> [--env NAME] [--allow-host HOST]... [--allow-unsafe] [--read-only]
```

| Flag | Effect |
| --- | --- |
| `--env NAME` | Default env for tools that don't pass one. Without it, the active env as the app set it |
| `--allow-host HOST` | Add a host (or `*.example.com`) to the allowlist. Repeatable. `'*'` turns the check off |
| `--allow-unsafe` | Allow every method, not just `GET`, `HEAD` and `OPTIONS` |
| `--read-only` | Don't offer the write tools (§3.3) |

Claude Code, from the repository that holds the project (`.mcp.json`,
committed so the team gets it):

```json
{
  "mcpServers": {
    "cactus": {
      "command": "cactus-cli",
      "args": ["mcp", "api", "--env", "dev", "--allow-unsafe"]
    }
  }
}
```

- stdout carries only protocol messages. Logs go to stderr.
- The server exits when stdin closes.
- `--allow-unsafe` with no `--env` prints a warning to stderr on start,
  because the active env may be production.

## 3. Tools

Every tool returns a short text summary and the same data as
`structuredContent`, with an `outputSchema`. Errors are tool errors with
`isError: true` and the `Error`'s message, never protocol errors, so the agent
can read and act on them.

Annotations: **R** = `readOnlyHint`. **N** = `openWorldHint` (talks to the
network). **W** = writes files, `destructiveHint: false`. A send that can't do
harm under the method policy is still **N**.

### 3.1 Reading (R)

| Tool | Arguments | Returns | Core |
| --- | --- | --- | --- |
| `list_requests` | `folder?` | `[{ path, title, method }]` in tree order | `scenario::list`, filtered by `is_within` |
| `read_request` | `path` | The file's YAML text, verbatim | `fs::read_to_string` on `path.resolve` |
| `list_environments` | — | `{ names, active, default }` (`default` = `--env`) | `env::list_named`, `env::active` |
| `explain_request` | `path`, `env?` | The request as it would be sent, without sending (§3.2) | `build_request` + `variable_sources` |
| `history` | `path`, `limit?` (default 5) | Earlier responses: time, status, checks, `time_ms`, trimmed body | `history::load` |

`read_request` returns the raw text, not the model, because the agent edits
YAML, and comments, scripts and unknown keys are part of what it should see.

### 3.2 `explain_request`

What an agent needs to debug a request that hasn't been sent, or that failed
before getting a response:

```json
{
  "method": "POST",
  "url": "https://staging.example.com/post",
  "headers": [["X-Client", "cactus-demo"], ["Authorization", "Bearer «secret:token»"]],
  "body": "{ \"username\": \"demo\", \"password\": \"«secret:password»\" }",
  "variables": [
    { "name": "baseUrl", "source": "environment:staging" },
    { "name": "password", "source": "secret" },
    { "name": "token", "source": "captured" }
  ],
  "unresolved": [],
  "warnings": [],
  "policy": { "allowed": true }
}
```

- `variables` lists names and the layer each value comes from, **never
  values**. The values of non-local layers can be read in the committed YAML
  anyway.
- `policy` says whether `send_request` would refuse it, and why (§4.2), so the
  agent can tell the person what flag to add instead of trying and failing.
- Multipart and file bodies show the file names, not their contents.

### 3.3 Sending (N)

| Tool | Arguments | Returns |
| --- | --- | --- |
| `send_request` | `path`, `env?`, `select?` | `{ ok, status, statusText, timeMs, url, headers, body, truncated, checks, captured: [names], captureFailures, unresolved, warnings }` |
| `run_folder` | `folder?`, `env?` | `{ passed, failed, results: [{ path, title, ok, status?, failedChecks, error? }] }` |

- Both go through `runner`, so history and captures are recorded exactly as
  when the app sends. The app shows these responses in its history.
- `captured` gives the **names** set, not the values.
- `body` is cut to 16 KiB for the agent (`truncated: true`, and the summary
  says how big it was). `select` is a `res.*` expression
  (`res.body.items[0]`, `res.headers.etag`) evaluated on the full body, for
  getting past the cut without a second send.
- `run_folder` results leave bodies out; the agent calls `history` or
  `send_request` for the one it cares about. If the client sent a
  `progressToken`, each finished request sends a progress notification
  (`n / total`, the title).
- A request the policy refuses is an error for `send_request`, and a result
  with `ok: false, error: "refused: …"` in `run_folder`. The run continues,
  like any other errored request.
- Cancellation (`notifications/cancelled`) drops the send in progress, as
  `AppState::run_cancellable` does in the app.

### 3.4 Writing (W, not offered with `--read-only`)

| Tool | Arguments | Returns |
| --- | --- | --- |
| `create_request` | `folder`, `yaml` | `{ path }`, the file name chosen from the title |
| `update_request` | `path`, `yaml` | `{ path, changed: bool }` |
| `import_spec` | `file` (inside the project) | `ImportSummary` |

- `yaml` is a whole request file. It's parsed into `Scenario` first, so a
  file that doesn't parse, or doesn't read back as written, is refused with
  the parse error and position, and nothing is written.
- `update_request` goes through `store::save`. The existing file's comments,
  quoting and unknown keys (scripts, tags, examples) survive. It follows that
  the agent **can't remove** keys the model doesn't know, and comments in its
  `yaml` aren't added. The tool's description says both.
- `create_request` goes through `store::create`, which writes Bruno's style
  and creates `folder.yml` files as needed.
- `import_spec` takes a path relative to the project and refuses anything that
  resolves outside it (after `canonicalize`). An agent that has a spec
  elsewhere copies it in first, which is visible in git.
- No delete or rename. A person does that in the app or with git.

### 3.5 Resources

| URI | What |
| --- | --- |
| `cactus://collection` | `opencollection.yml` |
| `cactus://environments/{name}` | An environment file |
| `cactus://requests/{path}` | A request file (a resource template, listed from `scenario::list`) |

They're the same files `read_request` returns. They exist so a client can
attach them as context without a tool call. No subscriptions for now (§11).

## 4. Core (`core/`)

Everything here is useful outside MCP, and nothing here depends on it.

### 4.1 `http::select` and `env::variable_sources`

- `pub fn http::select(response: &Response, expression: &str) -> Result<Option<Value>>`:
  a public wrapper over `Subject::parse`/`select` that parses the body once.
  An expression that doesn't parse is `Error::InvalidExpression(String)`.
- `pub fn env::variable_sources(project, named, inherited, request_variables)
  -> Result<BTreeMap<String, VarSource>>`, with
  `VarSource::{Collection, Environment { name }, Folder { dir }, Request, Secret, Captured}`.
  It's the same fold as `resolve_for_request`, keeping the source instead of
  the value. To make sure the two can't disagree, both are rewritten over one
  private `layers(...) -> Vec<(VarSource, Env)>`, as the critters plan's §3.1
  does. If critters have landed, use its `sources_for_request` and drop this.
- The desktop `variable_names` command can then use `variable_sources`. That's
  optional and not part of this plan.

### 4.2 `SendPolicy`

```rust
// core/src/http/policy.rs
#[derive(Debug, Clone, Default)]
pub struct SendPolicy {
    /// None = any method.
    pub methods: Option<Vec<reqwest::Method>>,
    /// None = any host. Entries are exact hosts or `*.suffix`.
    pub hosts: Option<Vec<String>>,
    /// Refuse attachments and file bodies outside the project.
    pub confine_files: bool,
}

impl SendPolicy {
    /// Why this request may not be sent, or None.
    pub fn refusal(&self, request: &reqwest::Request) -> Option<String>;
}
```

- `SendPolicy::default()` allows everything. That's what the app and the CLI
  pass, so their behaviour is unchanged.
- `http::send` takes `&SendPolicy` and checks `refusal` after `build_request`
  and before anything goes out. A refusal is `Error::Refused(String)`, for
  example: `refused: POST is not allowed (start the server with --allow-unsafe)`
  or `refused: evil.example isn't one of the project's hosts (add --allow-host evil.example)`.
- `confine_files` is checked in `build_request` where it resolves file paths
  (~l. 228, 239): the joined path is canonicalised and must start with the
  canonical project path. That fixes absolute and `..` paths. Symlinks out of
  the project are refused too, because the check runs after canonicalising.
- `runner::send_and_record` and `runner::run` take the policy and pass it on.
  The Tauri commands and CLI pass `&SendPolicy::default()`.

**The MCP server's host allowlist.** At start, the server builds every request
with every env (none, then each named env), with an empty secrets layer, and
collects the hosts of the URLs that resolve. That set, plus `--allow-host`, is
the allowlist.

- It's a snapshot of what the project already talks to. A request the agent
  writes to a new host is refused until a person allows it, and the refusal
  says how.
- Secrets are left out while collecting so a secret value can't put a host on
  the list. A URL that's unresolved without secrets doesn't add a host.
- Requests the person adds in the app later aren't covered until the server
  restarts. That's accepted: the refusal message says to restart or pass
  `--allow-host`.

### 4.3 Redaction (`core/src/redact.rs`)

```rust
pub struct Redactor { /* value → label, longest first */ }

impl Redactor {
    /// Values of the secrets and captured layers, at least MIN_LEN chars.
    pub fn for_project(project: &Path) -> Result<Self>;
    pub fn text(&self, input: &str) -> String;  // "«secret:password»", "«captured:token»"
    pub fn response(&self, response: &Response) -> Response; // url, headers, body, checks' detail
}
```

- Built per call, so values captured by the previous request are covered.
- `MIN_LEN` is 6. Shorter values (`"1"`, `"true"`) would mangle unrelated
  text, so they aren't redacted. Risks (§9) covers that.
- Also matches the URL-encoded form of each value, since that's how secrets
  usually appear in URLs. Base64 and other encodings aren't covered.
- Applied to everything that leaves the server: `explain_request`,
  `send_request`, `run_folder`, `history`. It's applied *after* `select`, so
  selecting a secret's location still returns it redacted.
- History on disk is unchanged. The secrets-encryption plan's §10 already
  suggests redacting history; when that happens, it can use this.

### 4.4 Parsing a request from text

`pub fn scenario::parse(text: &str) -> Result<Scenario>` (the same
deserialisation `collection::read` does, from a string), plus a round-trip
check: emitting the parsed scenario and parsing it again gives the same
scenario. The tools use it before any write, so a refusal never leaves a
partly written file.

### 4.5 `import::import_file` stays

The import tool canonicalises and confines the path (§3.4), then calls the
existing `import::import`. No core change.

### 4.6 Errors

Add `Refused(String)` and `InvalidExpression(String)`. Both reach the agent as
their message, as all errors do.

## 5. MCP server (`cli/src/mcp/`)

```
cli/src/mcp/
  mod.rs        serve(project, options): stdio transport, the server loop
  server.rs     the ServerHandler: tool router, resources, capabilities
  tools/        one file per area: read.rs, send.rs, write.rs
  policy.rs     host snapshot (§4.2), flags → SendPolicy
  output.rs     result shapes (structs with Serialize + JsonSchema), the 16 KiB cut, summaries
```

- **SDK:** `rmcp`, the official Rust SDK, with the `server`, `transport-io` and
  `macros` features. Tool argument and result types derive
  `schemars::JsonSchema` through the version `rmcp` re-exports, so the
  versions can't drift. Check its current API and version at phase 2;
  this plan doesn't depend on its macro details.
- **Feature-gated:** `cli` gets an `mcp` feature, on by default. Builds that
  only run requests in CI can turn it off with `--no-default-features` to skip
  `rmcp` and `schemars`. The `mcp` subcommand exists only with the feature.
- **Tokio:** `rmcp`'s stdio transport needs tokio's `io-std` (and probably
  `io-util`). `current_thread` stays. Sends are already async, and file I/O is
  small.
- **State:** the project path, one `reqwest::Client` (`http::build_client`),
  the `SendPolicy`, the default env and the flags. No caching of files:
  every tool reads from disk, so it sees edits made in the app.
- **Adapters only.** Each tool converts arguments, calls core, redacts, cuts
  and shapes. Anything with a decision in it belongs in core.
- **One send at a time**, like the app: a second `send_request` or
  `run_folder` while one is running waits for it. Captures from one request
  are then always visible to the next, which is what an agent expects.
- **Instructions:** the server's `instructions` say, briefly, that requests are
  OpenCollection YAML in Bruno's layout, that tools take `env` per call, that
  secrets are redacted as `«secret:name»` and can't be read, and that refused
  sends name the flag a person must add.

## 6. CLI (`cli/`)

- `Command::Mcp { project, env, allow_host: Vec<String>, allow_unsafe, read_only }`.
- `commands::execute` runs the server and returns `Outcome::Passed` when stdin
  closes. A startup failure (not a folder, unreadable collection) is an error
  and exit code 2, as today, printed to stderr before any protocol traffic.
- The existing commands are unchanged except for passing
  `&SendPolicy::default()`.

## 7. Working alongside the app

- The agent's writes reach the app through the watcher, like any outside
  edit. If the person has unsaved edits to the same request, the app shows
  its conflict banner. Nothing new is needed.
- Sends from the server record history and captures. The app shows them on
  the next refresh. The two processes can race on `captured.json`, as the CLI
  and app already can (accepted in the critters plan, §4.3).
- The server never writes `state.json`.

## 8. Compatibility

- **App and CLI:** `SendPolicy::default()` and an unchanged
  `resolve_for_request` result keep behaviour identical. The existing tests
  (`layers_override_in_order`, the runner and CLI tests) must pass unchanged.
- **Projects:** nothing on disk changes. Bruno is unaffected.
- **`confine_files`** is off for the app and CLI, so a project that uses an
  attachment outside the folder keeps working there. The MCP server refuses it
  and says why. If that proves common, add `--allow-files DIR`.

## 9. Risks

| Risk | Mitigation |
| --- | --- |
| A response steers the agent into sending secrets elsewhere | Host allowlist from the project's own URLs (§4.2); secrets can't add hosts; refusals need a person to change flags |
| A secret is echoed in a form redaction misses | Redaction is defence in depth; the allowlist keeps secrets on hosts that already get them. Stated in the README |
| Short secrets aren't redacted | `MIN_LEN` 6 is stated in the README; short secrets are rare and weak anyway |
| Agent runs `DELETE` on production | Safe methods only by default; `--allow-unsafe` with no `--env` warns; per-call `env` shows in the client's approval prompt |
| Agent writes break a file | Parse and round-trip check before writing; `collection::write` refuses edits that wouldn't read back |
| Large bodies flood the agent's context | 16 KiB cut, `select`, bodies left out of `run_folder` |
| `rmcp` API churn | Isolated in `cli/src/mcp/`; core has no MCP dependency |
| Snapshot allowlist goes stale as the project grows | The refusal message says to restart or add `--allow-host` |

## 10. Other plans

**Critters.** If critters have landed:

- `send_request`, `explain_request` and `run_folder` take `as?: critter`, which
  sets `Scope.critter`. Add `list_critters` (R) and `run_flow(critter, flow,
  env?)` (N), returning the flow run record, redacted.
- Redaction also covers the critter's secrets and `context.json` values.
- `SendPolicy` goes into the critters plan's `SendOptions` instead of being its
  own argument.
- Use `sources_for_request` and `select_values` instead of §4.1.
- The host snapshot builds with each critter too.

**Encrypted secrets.** If encryption has landed:

- The server is a process like the CLI. It gets the password the same way
  (`CACTUS_SECRETS_PASSWORD`, `--password-stdin`), and never prompts, since
  stdin is the protocol. Without a password, tools that only need names
  (`list_*`, `read_request`, `explain_request`'s variable sources) work and
  sends fail with the CLI's message.
- `Redactor::for_project` needs values, so it unlocks with the same key. With
  the vault locked, nothing can be sent, so nothing needs redacting.
- Update that plan's threat model: "coding agents reading files" is reduced
  further, because the server is the agent's only route to the values and it
  doesn't return them.

## 11. Later

- **MCP in the desktop app**, over local HTTP: the agent works on the project
  that's open, with its unsaved buffers, and the person sees sends happen
  live. Needs auth on the local port, so it's a separate plan.
- **Resource subscriptions**: notify on file changes (the CLI would need its
  own watcher), and for critters, on watched-value changes.
- **Prompts**: "debug this request", "write tests for this folder", as MCP
  prompts that pull in the right resources.
- **Per-environment policy** in a committed file (for example `prod` is always
  read-only), instead of flags only.
- **A send from unsaved YAML** (`send_request` with `yaml` instead of `path`),
  for trying a change before writing it. Held back because it makes the host
  allowlist the only thing between the agent and an arbitrary request.

## 12. Tests

**Core (Rust)**

- `SendPolicy::refusal`: methods, exact and wildcard hosts, default allows all.
- `confine_files`: an absolute path, a `..` path and a symlink out of the
  project are refused; a file inside is allowed; with the flag off, all are
  allowed (today's behaviour).
- `http::send` with a refusing policy sends nothing (a local test server sees
  no request).
- `variable_sources` agrees with `resolve_for_request` on which layer wins,
  over the `layers_override_in_order` fixture.
- `Redactor`: longest match first (a secret containing another), URL-encoded
  form, short values left alone, labels name the layer, `response()` covers
  url, headers, body and checks.
- `http::select`: status, header, body path, invalid expression.
- `scenario::parse`: a good file, a bad one with position, round trip.

**MCP server**

- Host snapshot: built from `examples/demo` with and without an env; a secret
  value that looks like a URL adds no host.
- Each tool against `examples/demo` with a local test server in place of
  httpbin (`baseUrl` overridden by an env): `list_requests` order,
  `explain_request` with `password` redacted, `send_request` with checks and
  captured names but no values, `run_folder` continuing past a refusal,
  `create_request` then `update_request` keeping a comment, `import_spec`
  refusing `../x.json`.
- `--read-only` lists no write tools.
- A protocol smoke test: spawn `cactus-cli mcp`, send `initialize`,
  `tools/list` and one `tools/call` over stdio, and check nothing but JSON-RPC
  goes to stdout.

**CLI**

- Argument parsing for `mcp` and its flags.

## 13. Order of work

Each phase starts by re-checking its §0 rows and ends green on the checks in
`ARCHITECTURE.md`.

1. **Core.** `SendPolicy` threaded through `http::send` and `runner` (app and
   CLI pass the default), `confine_files`, `http::select`, `layers` with
   `variable_sources`, `Redactor`, `scenario::parse`, new errors, tests.
   Useful and shippable on its own: it closes the attachment path escape.
2. **Server, read-only.** `mcp` feature and subcommand, `rmcp` stdio server,
   instructions, `list_requests`, `read_request`, `list_environments`,
   `explain_request`, `history`, resources, the protocol smoke test.
3. **Sending.** Host snapshot, flags → policy, `send_request` and
   `run_folder` with redaction, the cut, `select`, progress and cancellation.
4. **Writing.** `create_request`, `update_request`, `import_spec`,
   `--read-only`.
5. **Wrap-up.** `ARCHITECTURE.md` (a `cli/src/mcp` row, `SendPolicy`,
   redaction), a README section with the `.mcp.json` example and what's
   redacted and what isn't, and amendments to the critters and encryption
   plans per §10 for whichever hasn't landed.
