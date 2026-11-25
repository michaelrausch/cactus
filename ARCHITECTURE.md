# cactus architecture

cactus is a file-based HTTP client. A project is an
[OpenCollection](https://schema.opencollection.com/) v1.0.0 collection stored as
YAML files in the layout Bruno uses, one file per request ("scenario" in the
code), so projects can be versioned and reviewed like code and opened in Bruno.

```
core/       cactus-core: all behaviour. Knows nothing about Tauri or the terminal.
cli/        cactus-cli: headless runner for CI (clap).
src-tauri/  cactus: desktop backend, thin Tauri commands over core.
ui/         React front end for the desktop app.
```

## On disk

| Path                                     | What                                                                          | Committed |
| ---------------------------------------- | ----------------------------------------------------------------------------- | --------- |
| `opencollection.yml`                     | The collection: its defaults (`request.headers`, `auth`) and shared variables | yes       |
| `environments/<name>.yml`                | A named environment                                                           | yes       |
| `<folder>/folder.yml`                    | A folder's name, `seq` and defaults                                           | yes       |
| `critters/<id>/critter.yml`              | A critter (persona): name, species, notes, login flow, variables              | yes       |
| `critters/<id>/flows/<flow>.yml`         | A critter's flow: steps (requests, tools), optionally `triggers` and `alert`  | yes       |
| `**/*.yml`                               | Requests (every other `.yml`)                                                 | yes       |
| `.cactus/secrets.json`                   | Local secrets, the values of `secret: true` variables                         | no        |
| `.cactus/captured.json`                  | Values written by set-variable actions with no critter active                 | no        |
| `.cactus/critters/<id>/secrets.json`     | A critter's own secrets                                                       | no        |
| `.cactus/critters/<id>/context.json`     | What a critter's requests captured, with where and when                       | no        |
| `.cactus/critters/<id>/runs/<flow>.json` | The last 50 scheduled runs of a flow                                          | no        |
| `.cactus/state.json`                     | Active environment and critter                                                | no        |
| `.cactus/history/*.json`                 | Last responses per request (`a/b.yml` → `a%2Fb.json`)                         | no        |

All path rules live in `core/src/layout.rs`. Paths in the project (requests,
attachments, the files git lists) are a validated newtype (`ProjectPath`) that
can never escape the project. Attachments are read through it too, so a
request file from someone else can't upload a file from outside the project. Requests are listed
and run in tree order: at each level, folders and requests sorted together by
`seq`, as Bruno does.

**Variable precedence**, lowest first: collection variables → active
environment → folder variables (outermost first) → request `runtime.variables`
→ secrets → captured. With a critter active, the global captured values are
left out and its own layers come last instead: critter variables → critter
secrets → critter context, then what earlier tool steps of the flow run set,
then a flow step's `with`. One function
(`env::layers`) builds the ordered layers; `resolve_for_request` merges them
and `sources_for_request` says where each value came from, so the two can't
disagree. `{{name}}` substitution is a single pass. Headers and auth are
inherited the same way: collection, then folders, then the request;
`auth: inherit` takes the nearest one set.

**Critters** are personas. Sending as one uses its layers and saves what the
request captures to its context, never to another critter's. A flow runs
existing requests in order, each with its step's overlay applied in memory
(`expect` replaces status assertions, `assert` adds checks, `capture` adds
set-variable actions); the request files are never touched, and a failed step
skips the rest. On a 401, a critter with a `login` flow runs it and sends the
request once more (`runner::send_as`). Steps of the login flow itself never do
this, so it can't loop.

**Flow tools** are steps that aren't requests (`- tool: prompt`): prompt,
pick, display, set, assert, poll, wait, run, open, webhook, slack, notify
(`core/src/tools/`). Tools that send requests or run flows (poll, run) do so
through `ToolCx::runner`, as the run itself would; `run` nests at most 8
deep, so flows that run each other fail rather than loop. A step
is a request unless it has a `tool` key, so older flows read unchanged. Each
tool's templated fields are filled in before it runs; the values it sets
(`ToolCx::set`) last for the rest of the run and are never written to disk.
Tools reach the person through a `ToolHost` the caller passes to `run_flow`:
the desktop asks via the `FlowAsked` event and `answer_flow_ask`, the CLI asks
on the terminal (or takes `--var name=value`), and scheduled runs and
re-logins are `Headless`, so a prompt falls back to its defaults.

**Secrets files** (the project's and each critter's) are read and written
only through `crate::secrets` (`read`, `names`, `write`). They and the context
files (`captured.json`, each critter's `context.json`) are sealed whole by
`secrets::vault` (XChaCha20-Poly1305). With no password the key is built in,
so they're obscured rather than protected. A password (`.cactus/vault`: salt
and check value) switches to an Argon2id key; the app asks for it when a
project opens, the CLI reads `CACTUS_PASSWORD` or prompts. An optional TOTP
secret (`secrets::totp`, RFC 6238) is sealed inside `.cactus/vault`, so
unlocking also takes an authenticator code (`CACTUS_TOTP_CODE` in the CLI);
it's local, a second step rather than a true second factor. Changing the
password re-seals every file. Plain JSON from before is read as-is and
sealed on its next write.

**Saving never rewrites a file.** `collection::write` reads the file into its
model, diffs that against the model being saved and applies only the
differences to the file's lossless syntax tree (`collection/patch.rs`, on
`yaml-edit`). Comments, blank lines, quoting and every key the model doesn't
know (tags, examples, settings, Bruno scripts) survive. A file that doesn't
parse is never overwritten, and an edit that wouldn't read back as saved is
refused. New files are written by `collection/emit.rs` in Bruno's style.

cactus checks `runtime.assertions` (`res.status eq 200`, Bruno's operators) and
runs `set-variable` actions; it keeps `runtime.scripts` but can't run them, and
says so in the response's `warnings`.

## core

| Module                                            | Responsibility                                                                                                                                     |
| ------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------- |
| `collection`                                      | `model` (shared OpenCollection types, root/folder/environment files), `patch` (in-place saves), `emit` (new files), defaults chain                 |
| `scenario`                                        | `model` (the request file), `listing` (discovery, tree order), `store` (create/save/rename/delete)                                                 |
| `env`                                             | `layers` (env layers, `Scope`, merge and sources), `capture` (captured values and critter context, behind one lock), `template` (`Substituter`)    |
| `critter`                                         | `model` (critter, flow, `Interval`, alert rule), `store` (critters and flows on disk, schedules enabled), `secrets` (missing secrets)              |
| `http`                                            | `request` (scenario + inherited defaults → reqwest request), `send`, `response`, `expression` (`res.*`), `assertions` (actions, checks, pass/fail) |
| `import`                                          | Postman and OpenAPI → requests                                                                                                                     |
| `git`                                             | Status, stage/unstage, discard, diffs, commits and local branches in the project's repository (`git` feature)                                      |
| `runner`                                          | Send + persist captures + record history, re-login (`send_as`); folder runs; `flow` (step overlay, flow runs); `matrix`                            |
| `secrets`                                         | Secrets files; `vault` seals them and context                                                                                                      |
| `monitor`                                         | Scheduled runs: compact `RunRecord`s, alerts on changes of state (`compare`), the run log                                                          |
| `tools`                                           | Flow tool steps: the `Tool` trait, the `tools!` registry (`ToolStep`), `ToolHost` (asking, notifying, opening URLs), one file per tool             |
| `history`, `recent`, `state`, `storage`, `layout` | Persistence                                                                                                                                        |

Errors are typed (`core/src/error.rs`) and reach the UI as their message. An
I/O failure on a known file is `Error::File`, which names the file; attach the
path with `.map_err(Error::file(&path))`.

**Git** uses libgit2 through `git2`, vendored and without network features,
so it builds the same on Linux and Windows with no OpenSSL. It sits behind the
`git` feature, which only the desktop app enables. The project can be the
repository's root or any folder in it: only the project's files are listed
and staged, but a commit records the whole index, as `git commit` does
(`GitStatus::staged_outside` counts what else is staged). The author comes from
`user.name`/`user.email` in git config, and hooks don't run. `.git/` isn't
watched; the UI refreshes the status on file changes and on window focus.

## Desktop backend (src-tauri)

- `commands/*`: one adapter per command; no logic beyond argument conversion.
- `state.rs`: shared HTTP client, the scheduler and the single cancellable operation.
- `watcher.rs`: emits `FsChanged` when project files change, and reloads the
  scheduler when a file under `critters/` does.
- `scheduler.rs`: runs enabled flows on their timers while the app is open.
  `reload` diffs the wanted tasks against the running ones, so editing one
  flow doesn't reset other timers. Each task awaits its run (a flow never
  overlaps itself) and skips missed ticks after sleep. Scheduled runs don't
  use the cancellable slot, so they and interactive sends never cancel each
  other. Each run is recorded, compared with the last and appended to the log,
  then reported as `ScheduledRunFinished` and `MonitorAlert` events.
- `bindings.rs`: registers commands and events with tauri-specta.

`ui/src/api/bindings.ts` is **generated** from the Rust types. Regenerate with
`cargo test -p cactus` (or by running the app in debug). Never edit it by hand.

## UI (ui/src)

Data flows one way: **component → action → backend → store → component**.

| Folder        | Contains                                                               | May import       |
| ------------- | ---------------------------------------------------------------------- | ---------------- |
| `api/`        | Generated bindings, plus `client.ts` with the type aliases the UI uses | —                |
| `domain/`     | Pure functions with no React or backend imports, unit-tested           | `api` types      |
| `stores/`     | Zustand state, plus synchronous setters only                           | `api` types      |
| `actions/`    | Async workflows: call the backend, update stores, report errors        | everything above |
| `hooks/`      | App lifecycle and keyboard shortcuts                                   | actions, stores  |
| `components/` | Generic presentational pieces (`ui/` is generated by shadcn)           | domain, theme    |
| `features/`   | Screens, one folder per area; read stores, call actions                | all of the above |
| `app/`        | `App.tsx` layout and the `SHORTCUTS` table                             | features, hooks  |

Stores never import each other or actions, which keeps the graph acyclic
(enforced by `import/no-cycle`). Cross-store work belongs in `actions/`.
Store methods are named for what happened (`opened`, `saved`,
`historyLoaded`, `statusFailed`); plain UI flags use setters
(`setEnvDialogOpen`). Components read app state only in `features/`: the
editor hands template fields their variable names through
`KnownNamesContext`.

Errors in actions: user-initiated work goes through `reportErrors(work,
context)`, which shows a toast; background work whose failure only costs a
hint (highlighting, a diff preview) goes through `warnQuietly`. Nothing calls
`toast.error` or `console.warn` directly.

Nothing is saved automatically. Edits live in `editorStore` buffers (the open
scenario plus any other with unsaved edits), which are dirty until saved with
Ctrl+S (`actions/saving.ts`). If a file changes on disk under a dirty buffer it
is flagged as a conflict, resolved by reloading from disk or overwriting. The editor remounts via `key={path:editorVersion}`
whenever its draft state must reset.

**Critters in the UI.** The Critters window (`features/critters/`, Ctrl+B)
lists critters, their context and flows. A flow being edited lives in
`flowStore`: the model, its YAML text, undo/redo and the last run's results.
Visual edits are pure functions (`domain/flowEdit.ts`); `domain/flowDataflow.ts`
works out what each step uses and captures, which drives the chips between
steps and the problems list. Tool steps are described by the registry in
`features/critters/flows/tools/` (palette entry, card summary, dataflow and
editor), and `FlowAskDialog` answers a running flow's prompts. Writes to `.cactus/` aren't watched, so the
backend reports them as events (`ContextChanged`, `FlowProgress`,
`ScheduledRunFinished`, `MonitorAlert`); `actions/alerts.ts` turns alerts into
a toast when the window has focus, or an OS notification and an "N alerts"
banner when it doesn't. Sprites (`components/critters/`) are pixel maps drawn
as merged `<rect>` runs, animated by CSS alone (`critters.css`); the dev-only
page `/#sprites` shows them all.

## Common changes

Step-by-step guides with code for these and more are in
[docs/how-to/](docs/README.md#how-to-guides).

**Add a backend command.** Add the function in `core`, then an adapter in
`src-tauri/src/commands/`, then register it in `bindings.rs`. Run
`cargo test -p cactus` to regenerate the bindings, then call it via
`backend.yourCommand(...)` from an action.

**Add a flow tool.** Write `core/src/tools/<id>.rs`: a struct with its fields
(`#[serde(default)]`, empty fields left out) and `impl Tool` (`title`,
`templates`, `run`), then add `<id>::<Struct>` to `tools!` in
`core/src/tools/mod.rs`. Run `cargo test -p cactus` to regenerate the
bindings, then write `ui/src/features/critters/flows/tools/<id>.tsx` with
`defineTool` (label, icon, `create`, `dataflow`, `Editor` built from
`fields.tsx`) and add it to `TOOLS` in `tools/index.ts`, which won't
typecheck until every tool has one. `slack.rs` and `slack.tsx` are the
smallest example.

**Edit another OpenCollection field.** Add it to `core/src/scenario/model.rs`
(or `collection/model.rs`) under its spec name, with
`#[serde(default, skip_serializing_if = ...)]` so files without it still load
and stay minimal, and `LenientString` for strings hand-written YAML may type
as numbers (`value: 8080`). Until a field is in the model, saves leave it untouched. Regenerate the
bindings, then add an editor tab under `ui/src/features/editor/tabs/` that takes
`ScenarioTabProps`. The UI must hold a request exactly as the backend writes it
(empty lists left out, query params mirroring the URL), or every save looks
like an outside change and reloads the editor. Query-param syncing is
implemented on both sides, so its cases live in one fixture
(`core/tests/fixtures/query-params.json`) that both test suites run. Other
things the UI copies from core (assertion operators, layer file names) are
checked by `core/tests/ui_contract.rs`.

## Checks

```sh
cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace
cd ui && npm run typecheck && npm run lint && npm run format:check && npm test && npm run build
```

The UI lints with Oxlint because the project uses TypeScript 7, which
typescript-eslint does not support yet.
