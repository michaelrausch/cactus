# Critters: personas, flows and scheduled checks

Status: plan. Nothing here is built yet.

A **critter** is a persona in a project: a name, a pixel-art species and its
own variables. Sending as a critter layers its variables over the environment
and keeps whatever its requests capture (tokens, ids) in its own context.
A critter has **flows**: ordered lists of existing requests with per-step
inputs, captures, checks and watched values. A flow can run on a **timer** and
alert when it starts failing, recovers, or a watched value changes.

Projects that don't use critters behave exactly as they do today.

> **Re-validate before building.** This plan was written against commit
> `a43d433` ("Implement UI audit results"). The code may have changed since
> then, so the names, signatures and behaviour quoted here are a snapshot, not a
> contract. Before starting each phase, check what it relies on (§0) against
> the current code. Where the code has moved on, follow the code and update this
> plan. Never bend the code to match the plan.

## 0. Assumptions to re-check

Before each phase, confirm the assumptions that phase relies on. Run
`git log a43d433..HEAD --stat` to see what has changed, and read each file
listed below that appears in it.

| Assumption | Where | Phase |
| --- | --- | --- |
| Precedence is collection → env → folders → request vars → secrets → captured, in `resolve_for_request(project, named, inherited, request_variables)` | `core/src/env/layers.rs` | 1 |
| `EnvFile` is a `kind`/`name` tagged enum with `Base`, `Named`, `Secrets`, `Captured`; the UI edits layers through it | `core/src/env/layers.rs`, `ui/src/features/env/EnvLayerEditor.tsx`, `ui/src/domain/envFiles.ts` | 1, 7 |
| `merge_captured` does an unlocked read-modify-write of `captured.json` | `core/src/env/capture.rs` | 1 |
| `is_request_file` excludes `environments/` by prefix, and nothing else excludes `critters/` | `core/src/layout.rs`, `core/src/scenario/listing.rs` | 1 |
| `collection::write` patches files in place and refuses edits that wouldn't read back as saved | `core/src/collection/file.rs` | 1 |
| `WorkspaceState` holds only `env` | `core/src/state.rs` | 1 |
| `RequestInfo` has no `tags` field yet | `core/src/scenario/model.rs` | 1 |
| `Response` fields (`captured`, `checks`, `capture_failures`, `warnings`), all `#[serde(default)]`; history stores the whole `Response` | `core/src/http/response.rs`, `core/src/history.rs` | 1 |
| Assertions and set-variable actions are evaluated from `scenario.runtime` by `evaluate`, and pass/fail is `scenario_passed` | `core/src/http/assertions.rs` | 2 |
| `Subject::parse`/`select` handle `res.*` expressions and are `pub(crate)` | `core/src/http/expression.rs` | 2 |
| `runner::run` loops over entries in listing order and `RunResult` has `rel_path`, `title`, `ok`, `response`, `error` | `core/src/runner.rs` | 2 |
| `serde_yaml_ng` is the YAML parser, and its errors expose `location()` | `core/Cargo.toml` | 2 |
| One cancellable operation at a time through `AppState::run_cancellable` | `src-tauri/src/state.rs` | 4 |
| The watcher ignores `.cactus/` and, outside Windows, watches only directories that existed when the project opened | `src-tauri/src/watcher.rs` | 4 |
| Commands are thin adapters registered in `bindings.rs`; bindings regenerate with `cargo test -p cactus` | `src-tauri/src/bindings.rs` | 4 |
| `tokio` in `src-tauri` lacks the `time` feature; there's no notification plugin; `dragDropEnabled` isn't set | `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`, `src-tauri/capabilities/default.json` | 4 |
| `send_request` takes `(project, path, scenario)` and uses the active env; `variable_names` exists | `src-tauri/src/commands/request.rs`, `src-tauri/src/commands/env.rs` | 4 |
| The CLI has `run`, `send` and `import`, and `Outcome` sets the exit code | `cli/src/args.rs`, `cli/src/outcome.rs` | 4 |
| The UI layering rules (component → action → backend → store; stores don't import each other) | `ARCHITECTURE.md`, `ui/.oxlintrc.json` | 5–11 |
| `SHORTCUTS` has Ctrl-only entries and no `Ctrl+B` | `ui/src/app/shortcuts.ts` | 6 |
| `ScenarioSwitcher` is the `Ctrl+P` palette | `ui/src/features/palette/ScenarioSwitcher.tsx` | 6 |
| Theme ids (Phosphor, light themes) and the motion tokens `--dur-*`, `--ease-snappy` with the reduced-motion override | `ui/src/theme/themes.ts`, `ui/src/index.css` | 5 |
| `settingsStore` persists only `themeId` | `ui/src/stores/settingsStore.ts` | 5 |
| `fsSync` routes changed paths, and `isOwnRecentWrite` skips the app's own saves | `ui/src/actions/fsSync.ts`, `ui/src/actions/saving.ts` | 7, 9 |
| `TemplateField`, `templateParts`, `useExpressionSuggester`, `KeyValueTable`, `JsonTree`, `DiffView`, `ConflictBanner`, `StatusCode` exist with the roles assumed here | `ui/src/components/`, `ui/src/domain/template.ts`, `ui/src/hooks/`, `ui/src/features/` | 7–11 |

If an assumption no longer holds:

- **Renamed or moved:** follow it, and update the names in this plan.
- **Changed behaviour** (e.g. precedence reordered, or a new layer added):
  redesign the affected section here before writing code, and say so in the
  phase's PR.
- **Something this plan adds already exists** (e.g. tags, a scheduler, a
  locking helper): reuse it, and drop that part of the plan.

## 1. Goals and non-goals

**Goals**

- Any number of personas per project, each with isolated variables, secrets
  and captured context.
- Flows that chain existing requests, stored as YAML next to the critter.
- Reuse what exists: `Variable`, `Assertion`, set-variable captures, `res.*`
  expressions, the env layer editor, the run pipeline, in-place YAML saves.
- Scheduled flows with alerts on change of state.
- A UI with a bit of charm, from the sprites alone. Everything else stays plain.

**Non-goals (for now)**

- Branching, loops or scripts in flows.
- Sharing context between critters.
- Custom avatars.
- Running steps or flows in parallel.
- Schedules while the app is closed, cron expressions, webhook/Slack alerts.
  The file format leaves room for `trigger.cron` and `alert.webhook` later.

## 2. On disk

| Path | What | Committed |
| --- | --- | --- |
| `critters/<id>/critter.yml` | Name, species, notes, login flow, variables | yes |
| `critters/<id>/flows/<flow>.yml` | Flows | yes |
| `.cactus/critters/<id>/secrets.json` | Values of the critter's `secret: true` variables | no |
| `.cactus/critters/<id>/context.json` | Captured values with where and when they came from | no |
| `.cactus/runs/<id>__<flow>.json` | Last 50 records of each flow's runs | no |
| `.cactus/state.json` | Adds `critter` (active) and `schedules` (enabled on this machine) | no |

`<id>` and `<flow>` follow the env name rule, `[A-Za-z0-9_-]+`, because they
become file names.

### 2.1 `critter.yml`

```yaml
name: Bob
species: roadrunner
notes: Viewer account on staging
login: login                 # optional: critters/bob/flows/login.yml
variables:                   # same shape as an environment's variables
  - name: CLIENT_ID
    value: bob-client
  - name: CLIENT_SECRET
    secret: true             # value lives in .cactus/critters/bob/secrets.json
```

### 2.2 Flow files

```yaml
name: Staging health
trigger:                     # optional
  every: 5m                  # s | m | h, minimum 30s
  env: staging               # optional; default is the active env at run time
alert:                       # optional; default on: [failure]
  on: [failure, change]
steps:
  - request: auth/login.yml
    with:                    # variables for this step only, highest precedence
      username: bob@example.com
    capture:                 # name: res.* expression, saved to Bob's context
      access_token: res.body.access_token
  - request: users/delete.yml
    expect: 403              # replaces the request's own status assertions
  - request: status.yml
    assert:                  # extra checks, same type and operators as runtime.assertions
      - expression: res.body.db
        operator: eq
        value: ok
    watch:                   # alert when these differ from the previous run
      - res.body.version
```

### 2.3 `context.json`

```json
{
  "access_token": { "value": "eyJ…", "from": "auth/login.yml", "at": 1764000000000 }
}
```

The global `.cactus/captured.json` keeps its current flat format.

### 2.4 Auth tags

Requests use the OpenCollection field `info.tags`. A request is tagged auth
with `tags: [auth]`. Bruno already knows the field.

## 3. Variable precedence

Today, lowest first (`env::resolve_for_request`):

```
collection → named env → folders → request vars → secrets → captured
```

With a critter active:

```
collection → named env → folders → request vars → secrets
  → critter vars → critter secrets → critter context → step `with`
```

- The global `captured` layer is **left out** while a critter is active, so
  values never leak from one persona to another.
- Captures go to the active critter's `context.json`, or to `captured.json`
  when no critter is active, as today.
- Substitution stays single-pass.

### 3.1 One list of layers, used for everything

Precedence lives in one function returning ordered layers. Resolving
variables and explaining where they come from both fold over the same list,
so they can't disagree:

```rust
// core/src/env/layers.rs
pub struct Scope {
    pub env: Option<String>,      // resolved env name; None = no env
    pub critter: Option<String>,  // None = no critter
    pub overrides: Env,           // a flow step's `with`
}

impl Scope {
    /// What the app uses for an interactive send: the active env and critter from state.json.
    pub fn active(project: &Path) -> Self;
}

#[derive(Serialize, specta::Type)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum VarSource {
    Collection,
    Environment { name: String },
    Folder { dir: String },
    Request,
    Secret,
    Captured,
    Critter,
    CritterSecret,
    CritterContext { from: String, at: u64 },
    Step,
}

fn layers(project: &Path, scope: &Scope, inherited: &[RequestDefaults], request: &[Variable])
    -> Result<Vec<(VarSource, Env)>>;

pub fn resolve_for_request(..) -> Result<Env>                        // last layer wins
pub fn sources_for_request(..) -> Result<BTreeMap<String, VarSource>> // same fold, keeps the source
```

`resolve(project, named)` and the CLI keep working by building a `Scope` with
no critter. Without a critter the layers are exactly today's, which is what the
"nothing changes" test checks (§10).

### 3.2 `EnvFile` variants

```rust
pub enum EnvFile {
    Base, Named(String), Secrets, Captured,
    Critter(String),        // variables in critter.yml (in-place YAML edit)
    CritterSecrets(String), // .cactus/critters/<id>/secrets.json
    CritterContext(String), // values of context.json, for resolving
}
```

`load`/`save` handle them with the existing helpers (`active_values`,
`merge_values`, `read_json`, `write_json`). So the UI's `EnvLayerEditor` edits
a critter's variables and secrets unchanged. Saving `CritterContext` through
`save` replaces values and stamps `from: "edited"`. Captures use
`merge_context` (§4.3) instead.

## 4. Core (`core/`)

### 4.1 New module `critter/`

```
core/src/critter/
  mod.rs      re-exports
  model.rs    Critter, Species, Flow, FlowStep, Trigger, Interval, AlertOn, ContextEntry
  store.rs    list/create/rename/duplicate/delete, flows CRUD, raw text load/save, parse
  secrets.rs  missing_secrets
```

**Model**

```rust
#[derive(Serialize, Deserialize, Clone, PartialEq, specta::Type)]
pub struct Critter {
    #[serde(default, deserialize_with = "lenient_string")] pub name: String,
    #[serde(default)] pub species: Species,
    #[serde(default, skip_serializing_if = "String::is_empty")] pub notes: String,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub login: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")] pub variables: Vec<Variable>,
}

#[serde(rename_all = "kebab-case")]
pub enum Species { #[default] Roadrunner, Tortoise, Gecko, ElfOwl, Armadillo, Scorpion, Jackalope, Tumbleweed }

pub struct Flow {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub trigger: Option<Trigger>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub alert: Option<AlertRule>,
    #[serde(default)] pub steps: Vec<FlowStep>,
}

pub struct FlowStep {
    pub request: String,                 // validated as ScenarioPath when run, so a bad path is a step error, not a load error
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")] pub with: Env,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")] pub capture: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub expect: Option<u16>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")] pub assert: Vec<Assertion>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")] pub watch: Vec<String>,
}

pub struct Trigger { pub every: Interval, #[serde(default)] pub env: Option<String> }
pub struct Interval(u32);              // seconds; (de)serialised as "30s" / "5m" / "1h", < 30s rejected
pub struct AlertRule { #[serde(default = "failure_only")] pub on: Vec<AlertOn> }
pub enum AlertOn { Failure, Change }
pub struct ContextEntry { pub value: String, pub from: String, #[specta(type = u32)] pub at: u64 }
```

All string fields that hand-written YAML might type as numbers use
`lenient_string`, as in the rest of the model.

**Store** (`store.rs`)

- `list(project) -> Vec<CritterSummary { id, name, species, flows: Vec<FlowSummary>, missing_secrets: Vec<String> }>`.
  One call gives the UI everything it lists.
- `create(project, id, critter)`, `load`, `save` (`collection::write`, so in place),
  `rename(from, to)` (moves the directory and `.cactus/critters/<id>/`, and
  updates `state.critter` and `state.schedules`), `delete` (both directories,
  plus state), `duplicate(from, to, name, species)` (copies `critter.yml` and
  `flows/`, but no secrets, context or run logs).
- Flows: `list_flows`, `load_flow`, `save_flow` (`collection::write`),
  `create_flow`, `rename_flow`, `delete_flow` (also deletes its run log and
  drops it from `schedules`).
- Raw YAML: `load_flow_text`, `save_flow_text(text)`. `save_flow_text` parses
  first and writes the text verbatim with `storage::write_atomic`, so YAML mode
  keeps comments and formatting exactly as typed.
- `parse_flow(text) -> FlowParse`:

  ```rust
  #[serde(tag = "kind")]
  pub enum FlowParse { Ok { flow: Flow }, Invalid { message: String, line: Option<u32>, column: Option<u32> } }
  ```

  The position comes from `serde_yaml_ng::Error::location()`. It's a value,
  not an `Err`, because invalid YAML while typing is normal.

**`secrets.rs`**: `missing_secrets(project, id)`: names of `secret: true`
variables in `critter.yml` with no value in the critter's secrets or the
project's secrets.

### 4.2 `layout.rs`

- `CRITTERS_DIR = "critters"`, plus `critter_dir`, `critter_file`, `flows_dir`,
  `flow_file`, `critter_workspace_dir`, `critter_secrets_file`,
  `critter_context_file`, `runs_dir`, `run_log_file(id, flow)` (`__` separator,
  as history uses).
- `is_request_file`: also false under a top-level `critters/`, the same check
  as `environments/`. Tested.
- `critter_from_path(rel) -> Option<(id, Option<flow>)>`, for the UI's file
  sync and the scheduler's reload.

### 4.3 `env/capture.rs`

- `merge_context(project, id, values: &Env, from: &ScenarioPath)` writes
  `ContextEntry`s with `at = now`.
- `restore_context(project, id, entries)` puts back what "Clear all" or × removed, for Undo.
- A process-wide `static CONTEXT_WRITE: Mutex<()>` guards every
  read-modify-write of `captured.json` and `context.json`. A scheduled run and
  an interactive send can capture at the same moment, and without the lock
  one would lose the other's values. A CLI process at the same time isn't
  covered; that's an accepted limitation.

### 4.4 `scenario/model.rs`, `http/response.rs`, `state.rs`

- `RequestInfo.tags: Vec<String>` (`default`, `skip_serializing_if = "Vec::is_empty"`).
- `Response` gains `critter: Option<String>` and `relogged: bool`, both
  `#[serde(default)]`, so existing history files load. History needs no change
  of its own, since it stores the whole `Response`.
- `WorkspaceState` gains `critter: Option<String>` and
  `schedules: Vec<String>` (`"<id>/<flow>"`), both `#[serde(default, skip_serializing_if)]`.
  `critter::active(project)` mirrors `env::active`: a critter whose directory
  is gone counts as none.

### 4.5 A flow step is a scenario overlay

A step doesn't need its own evaluation code. Before sending, the runner copies
the request and changes the copy in memory:

```rust
fn apply_step(mut scenario: Scenario, step: &FlowStep) -> Scenario {
    if let Some(status) = step.expect {
        scenario.runtime.assertions.retain(|a| Subject::parse(&a.expression) != Some(Subject::Status));
        scenario.runtime.assertions.push(Assertion::new("res.status", "eq", status.to_string()));
    }
    scenario.runtime.assertions.extend(step.assert.iter().cloned());
    scenario.runtime.actions.extend(step.capture.iter().map(|(n, e)| Action::SetVariable(SetVariable::new(n, e))));
    scenario
}
```

`expect`, `assert` and `capture` then go through the existing `evaluate`,
`scenario_passed` and capture code. `with` goes in through `Scope.overrides`.
The file on disk is never touched.

`watch` needs values, not pass/fail:
`http::select_values(response: &Response, expressions: &[String]) -> BTreeMap<String, Option<String>>`
parses `response.body` once and uses `Subject::select`. A truncated or
non-JSON body gives `None` for body paths. That shows as `—` and never counts
as a change.

### 4.6 `runner.rs`

```rust
pub struct SendOptions { pub record_history: bool }

/// Today's function, with a scope. Captures go to the critter's context when
/// one is set. The response is stamped with the critter.
pub async fn send_and_record(client, project, path, scenario, scope: &Scope, opts) -> Result<Response>;

/// What the app and the CLI call: send_and_record, plus automatic re-login.
pub async fn send_as(client, project, path, scenario, scope: &Scope, opts) -> Result<Response>;
```

**Automatic re-login** (`send_as`)

1. Send. If the status isn't 401, or there's no critter, or the critter has no
   `login`, return.
2. Run the login flow's steps with `run_steps(.., relogin: false)`, which calls
   `send_and_record` directly. This is what prevents recursion: no async fn
   calls itself, so there's no boxing and no loop.
3. If the login flow passed, resend once with a freshly resolved scope, since
   the context has the new token. Set `relogged = true` and add a warning
   ("Logged in again after 401 using flow `login`; the request was sent twice").
4. If login failed, return the original 401 with a warning naming the failed
   step.

**Flows**

```rust
pub struct FlowStepResult {
    pub index: u32,
    pub run: RunResult,                              // existing type
    pub watched: BTreeMap<String, Option<String>>,
    pub skipped: bool,
}

pub async fn run_flow(client, project, critter: &str, flow: &str, env: Option<String>,
                      opts: SendOptions, on_step: impl FnMut(&FlowStepResult)) -> Result<FlowRun>;
```

- Steps run in order. Each one loads its request from disk (a missing file is
  a step error), applies the overlay, resolves a `Scope` with that step's
  `with`, and calls `send_as`.
- A failed step stops the flow. The rest are reported as `skipped`, so the UI
  and the run log always have one result per step.
- `FlowRun { at, env, ok, elapsed_ms, steps: Vec<FlowStepResult> }`.
- The existing `run` (folder runs) is refactored to share the step loop. Its
  behaviour and `RunProgress` event stay as they are.

**Matrix**

```rust
pub struct MatrixCell { pub critter: Option<String>, pub response: Option<Response>, pub error: Option<String>, pub skipped: Option<String> }
pub async fn run_matrix(client, project, path, scenario, on_cell) -> Result<Vec<MatrixCell>>;
```

- One send per critter plus one with no critter, one after another. Sending
  them in sequence keeps context writes simple.
- `record_history: false`, but captures do go to each critter's context. These
  are real sends as that persona.
- Critters with missing secrets are skipped with a reason, not sent.

### 4.7 `monitor.rs` (new, pure)

```rust
pub struct RunRecord {
    pub at: u64, pub env: Option<String>, pub ok: bool, pub elapsed_ms: u32,
    pub steps: Vec<StepRecord>,
}
pub struct StepRecord {
    pub request: String, pub ok: bool, pub skipped: bool,
    pub status: Option<u16>, pub failed_checks: Vec<Check>,
    pub error: Option<String>, pub watched: BTreeMap<String, Option<String>>,
}

pub enum AlertEvent {
    Failing { step: u32, request: String, reason: String },
    Recovered,
    Changed { step: u32, expression: String, before: Option<String>, after: Option<String> },
}

pub fn record(run: &FlowRun) -> RunRecord;  // no bodies, so the log stays small
pub fn compare(previous: Option<&RunRecord>, current: &RunRecord, rule: &AlertRule) -> Vec<AlertEvent>;
pub fn append(project, id, flow, record) -> Result<()>;  // keeps the newest 50
pub fn load_log(project, id, flow) -> Vec<RunRecord>;    // lenient: a corrupt log is empty
```

`compare` alerts on changes of state, never on every run:

| Previous | Current | Events (filtered by `alert.on`) |
| --- | --- | --- |
| none | any | none: this run is the baseline |
| ok | failing | `Failing` (first failed step, first failed check or error) |
| failing | failing | none |
| failing | ok | `Recovered` |
| any | any | `Changed` for each watched expression whose value differs, comparing only when both runs reached that step and neither value is `None` |

`Recovered` is sent whenever `failure` is enabled.

### 4.8 Errors

New `Error` variants:

- `InvalidCritterName(String)`
- `InvalidFlowName(String)`
- `CritterNotFound(String)`
- `FlowNotFound { critter, flow }`
- `InvalidInterval(String)`: "every: use e.g. 30s, 5m or 1h; minimum 30s".

## 5. Desktop backend (`src-tauri/`)

### 5.1 Commands (`commands/critter.rs`, `commands/flow.rs`)

Thin adapters, as now:

| Command | Core |
| --- | --- |
| `list_critters` | `critter::list` |
| `load_critter` / `save_critter` / `create_critter` / `rename_critter` / `duplicate_critter` / `delete_critter` | store |
| `active_critter` / `set_active_critter` | state |
| `load_context` / `restore_context` / `clear_context` | capture |
| `list_flows` / `load_flow` / `save_flow` / `create_flow` / `rename_flow` / `delete_flow` | store |
| `load_flow_text` / `save_flow_text` / `parse_flow` | store |
| `run_flow` | `runner::run_flow` in `run_cancellable`, emits `FlowProgress` |
| `run_matrix` | `runner::run_matrix` in `run_cancellable`, emits `MatrixProgress` |
| `variable_sources` | `env::sources_for_request` |
| `set_schedule_enabled` / `set_schedules_paused` / `schedule_states` / `load_run_log` / `run_scheduled_now` | scheduler |

- `send_request` keeps its signature. It builds `Scope::active` and calls
  `runner::send_as`, so the sidebar's critter choice applies to every send.
- `run_folder` uses `Scope::active` too, so "Run all" as Bob does what you'd
  expect.
- `variable_names` is replaced by `variable_sources`, and the UI takes the
  keys. One call replaces two.

New events in `events.rs`, registered in `bindings.rs`:

- `FlowProgress { critter, flow, step: FlowStepResult }`
- `MatrixProgress(MatrixCell)`
- `ScheduledRunFinished { critter, flow, record: RunRecord }`
- `MonitorAlert { critter, flow, event: AlertEvent, at }`
- `ContextChanged { critter }`: emitted after any send, run or clear that
  wrote a critter's context. `.cactus/` writes are filtered out by the
  watcher's `is_own_write`, so the UI needs this to stay current.

### 5.2 Scheduler (`src-tauri/src/scheduler.rs`)

```rust
pub struct Scheduler {
    project: Mutex<Option<PathBuf>>,
    tasks: Mutex<HashMap<String /* id/flow */, Task>>,
    paused: AtomicBool,
}
struct Task { every: Interval, env: Option<String>, handle: tauri::async_runtime::JoinHandle<()> }
```

- `AppState` owns one. `open_project` calls `scheduler.reload(app, project)`,
  and so do the commands that change flows, critters or schedules. The
  watcher's `FsChanged` handler calls it too when a path is under `critters/`.
  Commands call it directly because, on macOS and Linux, the watcher only
  watches directories that existed when the project opened, so a new critter's
  `flows/` directory isn't watched yet.
- `reload` computes the wanted set from `state.schedules` and each listed
  flow's `trigger` (an entry without a trigger is ignored). It aborts tasks
  that are gone or changed and spawns new ones, leaving unchanged tasks alone,
  so editing one flow doesn't reset other timers.
- Each task:

  ```rust
  let mut tick = tokio::time::interval_at(Instant::now() + every, every);
  tick.set_missed_tick_behavior(MissedTickBehavior::Skip); // no backlog after sleep
  loop {
      tick.tick().await;
      if paused { continue }
      let run = runner::run_flow(&client, &project, id, flow, env.clone(), SendOptions { record_history: false }, |_| {}).await;
      // record → load previous → compare → append → emit ScheduledRunFinished, MonitorAlert*, ContextChanged
  }
  ```

  The loop awaits each run, so a flow can't overlap itself. Scheduled runs
  don't use `run_cancellable`, so they never cancel, and are never cancelled
  by, interactive sends. A run that errors before any step (e.g. the flow file
  no longer parses) is recorded as a failing run with that error, which
  produces a single `Failing` alert, not a crash loop.
- Opening another project or quitting aborts every task.
- `run_scheduled_now` runs once through the same code path, outside the timer.
- `tokio` in `src-tauri` gains the `time` feature.

### 5.3 Notifications

- Add `tauri-plugin-notification` and `@tauri-apps/plugin-notification`, with
  `notification:default` in `capabilities/default.json`.
- The UI decides how to show an alert: a toast if the window has focus,
  otherwise an OS notification. One code path, and never both.
- Permission is requested the first time a schedule is enabled. If it's
  denied, alerts are in-app only, and the schedule control says so.
- On desktop the plugin can't route a notification click to a specific view.
  So when the window regains focus with alerts it hasn't shown, an "N alerts"
  banner appears, and View opens the latest flow run.

### 5.4 Other configuration

`tauri.conf.json`: set `"dragDropEnabled": false` on the main window.
Otherwise Tauri takes over drag events for OS file drops and HTML5 drag and
drop doesn't work in the webview on Windows. Import uses a file dialog, so
nothing is lost.

## 6. CLI (`cli/`)

- `--as <critter>` on `run` and `send`, which sets `Scope.critter`.
- `cactus-cli flow <project> <critter> <flow> [-e env] [--json] [--junit FILE]`.
- `cactus-cli flow <project> --all [...]` runs every flow of every critter.
- JUnit: one `<testsuite>` per critter, one `<testcase name="<flow> › <n>. <request>">`
  per step, `<failure>` with the failed checks, `<skipped/>` for skipped steps.
  A small hand-written writer with XML escaping and tests; no new dependency.
- Exit code: non-zero if any step failed (existing `Outcome`).
- No scheduler in the CLI. For headless schedules, run `flow --all` from cron
  or CI.

## 7. UI (`ui/src/`)

### 7.1 Principles

1. Invisible until used. With no critters, the footer gains one small control
   reading "No critter".
2. It's always clear who you're sending as: footer, Send button, response,
   history.
3. Restrained motion: idle bob, a run cycle while working, a hop on pass, a
   shake on fail. Nothing else.
4. Show where values come from, wherever a variable appears.
5. Undo rather than confirm for anything reversible. Only deleting a critter
   asks first.

### 7.2 Structure (following `ARCHITECTURE.md`'s layers)

```
api/client.ts                 + aliases: Critter, CritterSummary, Flow, FlowStep, FlowParse,
                                FlowStepResult, MatrixCell, RunRecord, AlertEvent, VarSource, ContextEntry
domain/
  critterId.ts                name → id, uniqueness            (tested)
  jwt.ts                      decode claims, expiry            (tested)
  interval.ts                 "5m" ⇄ seconds, display          (tested)
  flowEdit.ts                 insert/move/remove/update step   (tested)
  flowDataflow.ts             which captures feed which {{uses}}; problems list (tested)
  dragInsert.ts               pointer Y + card midpoints → insertion index (tested)
  matrixGroups.ts             group identical responses        (tested)
  runTimeline.ts              RunRecord[] → dot states, "failing since" (tested)
stores/
  critterStore.ts             summaries, active id, selected id, context entries of the selected one
  flowStore.ts                open flow: model, text, mode, dirty, undo/redo stacks, last run per step
  monitorStore.ts             schedule states, unseen alerts, paused
  uiStore.ts                  + view: "editor" | "critters", matrixFor: ScenarioPath | null
  settingsStore.ts            + animateCritters: boolean
actions/
  critters.ts                 CRUD, act as, context clear/undo
  flows.ts                    open/save (visual or text), run, parse-as-you-type
  monitors.ts                 enable/disable, pause, alerts → toast or notification
  matrix.ts                   run, pick row, diff, "add as check"
  fsSync.ts                   + critters/** paths → refresh summaries / reload the open flow or flag a conflict
components/critters/
  CritterSprite.tsx           <CritterSprite species size state label />
  species.ts                  pixel maps, palettes, accent, name, one-line description
  critters.css                keyframes
features/critters/
  CrittersView.tsx            list + detail
  CritterList.tsx
  NewCritterDialog.tsx
  context/ContextTab.tsx      missing secrets, variables (EnvLayerEditor), captured
  flows/FlowsTab.tsx          flow list + editor
  flows/RequestPalette.tsx
  flows/StepLane.tsx
  flows/StepCard.tsx
  flows/StepInspector.tsx
  flows/FlowYamlEditor.tsx
  flows/ScheduleControl.tsx
  flows/RunTimeline.tsx
  settings/CritterSettingsTab.tsx
  matrix/MatrixDialog.tsx
features/sidebar/CritterPicker.tsx
```

Stores still never import each other. Cross-store work, like an alert
updating `monitorStore` and the critter list, goes through `actions/`.

### 7.3 Sprites

- `species.ts` defines each species with frames of 16 strings × 16 characters,
  plus a palette (`{ ".": null, "k": "#1b1b1f", "t": "#2bb3a3", … }`) and an
  `accent` colour.
- `CritterSprite` merges horizontal runs of one colour into a single `<rect>`,
  about 40–70 per frame, memoised per species and frame. It renders an SVG with
  `viewBox="0 0 16 16"`, `shape-rendering="crispEdges"`, at 16, 32 or 48px only.
- Frames: `idle` (2 frames, blink), `work` (2), `pass` (1), `fail` (1). Every
  frame of a state is rendered as stacked `<g>`, and CSS switches between them
  with `steps(1)` keyframes. No React timers, so a list of 20 critters doesn't
  re-render.
- Motion: the wrapper animates `transform` only. The blink is given a
  `--seed`-based negative delay from a hash of the id, so critters don't blink
  in unison.
- Idle animation pauses when `:root[data-window-blurred]` is set (from focus
  and blur listeners in `useAppLifecycle`), when `animateCritters` is off, and
  under the existing `prefers-reduced-motion` rule.
- Themes: `--critter-filter` (Phosphor: `grayscale(1) sepia(1) hue-rotate(70deg) saturate(3)`).
  Light themes get `--critter-outline`, drawn as a 1px `drop-shadow` stack. No
  `blur()`, which this codebase notes is slow in WebKitGTK.
- Accessibility: `role="img"`, `aria-label` such as "Bob, roadrunner, sending, token expired".
  States also show as a badge (spinner, ✓, !, z) in the sprite's corner, so
  motion is never the only signal.

### 7.4 Ways in

- **Sidebar footer:** `CritterPicker` sits beside `EnvPicker`, a 16px sprite
  plus name, or "No critter". The dropdown lists critters, "No critter", the
  active critter's flows (▶ runs one) and "Manage critters… (Ctrl+B)".
  A red dot appears when a scheduled flow is failing. Pause/resume schedules is
  also in this dropdown.
- **Shortcuts:**
  - `critters` (`Ctrl+B`) toggles `uiStore.view`.
  - `tryAsEveryone` (`Ctrl+Shift+Enter`). The `SHORTCUTS` table and
    `shortcutForKey` gain an optional `shift: true`.
- **`Ctrl+P`:** `ScenarioSwitcher` treats an `@` prefix as critters (Enter acts
  as one, `@none` clears) and a `>` prefix as flows (Enter runs).

### 7.5 Critters view

- Replaces the editor area when `view === "critters"`. The breadcrumb reads
  `project / Critters`, and Esc returns to the request.
- **Left:** `CritterList` shows a 32px sprite, the name, and a status
  (acting as · token expired · N missing secrets · failing schedule).
  Right-click: Act as / Duplicate / Rename / Delete. ↑/↓ and Enter work.
  A **Scheduled** section at the bottom lists every enabled schedule in the
  project, with its state and next run.
- **Right:** a header with a 48px sprite, the name (click to rename inline),
  species, notes and an **Act as** button. Tabs: Context · Flows · Settings.
- **Empty state:** one sentence and a **New critter** button.

**New critter dialog** (single step)

- Species grid: 4×2 with sprite, name and one line. Arrow keys work.
- Name: shows `critters/<id>/` underneath and checks uniqueness as you type.
- Start from: empty / copy variable names from an environment (values left
  empty) / duplicate a critter.
- Create selects the new critter.

### 7.6 Context tab

- **Missing secrets:** only shown when there are any. One input each. Saving
  writes `CritterSecrets` and refreshes the summary.
- **Variables:** `EnvLayerEditor` with `file = { kind: "critter", name: id }`
  and `{ kind: "critterSecrets", name: id }`, as two sub-tabs. A variable that
  overrides a lower layer gets a muted tag ("overrides staging"), using
  `variable_sources` for that layer.
- **Captured:**
  - One row per value: name in `--color-captured`, the value hidden until
    clicked, copy, a source link (opens the request), and age. Age uses one
    shared 30s ticker for the whole list.
  - Values that `jwt.ts` recognises expand to show their claims, and display
    "expires in 4m" or "expired". When expired, **Log in again** (if a login
    flow is set) runs it.
  - × and **Clear all** keep what they removed in memory and offer Undo in a
    toast for 6s (`restore_context`).
  - Updates on `ContextChanged`.

### 7.7 Showing who you're sending as

- **Send button (`UrlBar`):** a 16px sprite on its left, in the `work` state
  while sending. The tooltip reads "Send as Bob".
- **Editor:** a 2px line in the critter's accent colour along the top of the
  editor area.
- **Response summary:** an "as Bob" chip from `response.critter`, plus
  "· logged in again" when `response.relogged`.
- **History list:** a sprite per entry and a filter by critter.
- **`TemplateField` tooltips:** `{{name}}` shows its source from
  `projectStore.variableSources`, e.g. "Bob's context · auth/login.yml, 2m ago",
  "staging", "not set". `variableNames` becomes `Object.keys(variableSources)`.
- **Request editor:** an Auth toggle (key icon) that edits `scenario.info.tags`.
  The sidebar shows a key on tagged requests (`ScenarioEntry` gains `tags`).
- **Git dialog:** paths under `critters/` show a small sprite icon.

### 7.8 Flows tab

- **Flow list:** name, step count, last-run dot, `⏱ 5m` when scheduled, and ★
  on the login flow. Right-click: Rename / Duplicate / Set as login flow /
  Delete (with Undo, which keeps the text in memory and recreates the file).

**Editor:** three resizable panes.

- **`RequestPalette`:** the project tree from `projectStore.entries`, filtered
  with the sidebar's search, auth-tagged requests first. Rows are
  `draggable`, and `dragstart` sets `application/x-cactus-request` (WebKit
  needs `setData` for the drag to start). Each row also has a `+` button.
- **`StepLane`:** cards (step number, method badge, title, status slot) on a
  vertical track.
  - On `dragover`, `dragInsert.ts` computes the insertion index from card
    midpoints, and the cards at that index slide apart by `--dur-fast`.
  - Dropping a palette row inserts a step. Dropping a card moves it. Dropping
    a card onto the palette removes it.
  - Keyboard: Alt+↑/↓ moves the focused card, Delete removes it, Enter opens
    it in the inspector.
  - Between cards, chips from `flowDataflow.ts` show which values pass along
    (`access_token ↓`). A value is listed when an earlier step captures it (its
    own `capture` or its request's set-variable actions) and a later step uses
    `{{name}}` (from `templateParts` over the request's URL, headers, params,
    body and auth, plus the step's `with`).
- **`StepInspector`:**
  - Request path (with a link to open it).
  - **Expect** (a number field suggesting 200/201/204/401/403/404).
  - **Uses:** each `{{name}}` with its source: step N / critter / env /
    missing. Clicking a missing one adds it to `with`.
  - **With:** `KeyValueTable` with `TemplateField`.
  - **Capture:** name plus expression with `useExpressionSuggester`, and
    "pick from last response", which opens `JsonTree` from the step's last
    result. Clicking a node fills in its expression.
  - **Checks:** the `assert` list, using the `AssertionsTab` table component.
  - **Watch:** a list of expressions, with the same suggester and picker.
- **Problems:** `flowDataflow.ts` reports missing variables and requests that
  no longer exist. They show as a dot on the card and a count in the footer;
  clicking the count selects the first one. Run isn't blocked.

**Editing state** (`flowStore`)

- `model: Flow`, `text: string`, `mode: "visual" | "yaml"`, `savedFingerprint`,
  `undo: Flow[]`, `redo: Flow[]` (capped at 100), `lastRun: (FlowStepResult | null)[]`.
- Visual edits go through the pure functions in `flowEdit.ts` and push to
  `undo`. Ctrl+Z / Ctrl+Shift+Z work while the flow editor has focus.
- YAML mode: `FlowYamlEditor` is a mono textarea with a line-number gutter.
  `parse_flow` runs 200ms after typing stops. `Invalid` underlines that line
  and shows the message. `Ok` updates `model`, so the data-flow chips and
  problems stay live.
- Switching YAML → Visual is blocked while the text is invalid ("Fix line 7
  first"). Switching Visual → YAML loads text from `load_flow_text` if the
  model is saved, or a preview from `parse`/emit if it's dirty.
- Ctrl+S saves with `save_flow_text` in YAML mode and `save_flow` in Visual
  mode. Both record `isOwnRecentWrite`. External changes reload the flow, or
  show `ConflictBanner` when there are unsaved edits, as requests do.

**Running**

- ▶ Run (or Ctrl+Enter in the lane). Unsaved edits get the "Save and run / Run
  saved version" choice, as folder runs do.
- `FlowProgress` fills `lastRun[index]`. Cards show queued (dimmed), running
  (pulsing border, with the critter's 16px sprite in `work`), passed ✓,
  failed (one shake, red) or skipped. A step that logged in again shows a
  "logged in again" note.
- The summary line reads "3/3 passed · 1.24s" or "Failed at step 2: expected
  403, got 200". Cards expand to show status, time, captures, failed checks,
  watched values and "Open in response viewer". Editing the flow marks old
  results as "may be stale".

**Schedule** (`ScheduleControl`, in the flow header)

- Off / Every `[5m]` (validated with `interval.ts`, minimum 30s), an env
  select (default "active environment"), and Alert on ☑ failures ☑ changes.
  These write `trigger` and `alert` into the flow file.
- An **Enabled on this machine** switch calls `set_schedule_enabled`. The
  muted line below reads "Runs while cactus is open · next run in 3:12"
  (`schedule_states` gives `next_at`). **Run now** sits beside it.
- **`RunTimeline`:** above the lane, the last 50 `RunRecord`s as dots: green,
  red, or a blue ring for changed. Hover shows time, duration and env. Clicking
  shows that run's results on the cards, with changes as `1.4.2 → 1.5.0`.

### 7.9 Matrix

- `MatrixDialog` opens from a button next to Send, from right-clicking a
  request, or with `Ctrl+Shift+Enter`. It sends the request as currently
  edited, the same as Send.
- Rows: No critter plus each critter, with sprite, name, `StatusCode`, time
  and size. They fill in from `MatrixProgress`.
- `matrixGroups.ts` groups rows with the same status and body hash, and draws a
  bracket labelled "same response".
- Click a row to show its response in the response pane. Select two to open a
  body diff (reusing `DiffView` with a simple line-diff input).
- "Add as check" adds `{ request, expect: status }` to a flow of that critter
  you choose (new or existing).
- Skipped critters (missing secrets) are greyed out with the reason.

### 7.10 Alerts in the UI

- `MonitorAlert` goes to `actions/monitors.ts`. If the window is focused it
  shows a toast (Failing → `toast.error`, Recovered → `toast.success`,
  Changed → `toast.info`) with a View action. Otherwise it sends an OS
  notification and adds the alert to `monitorStore.unseen`, and on focus an
  "N alerts" banner offers View.
- The critter's sprite plays `fail` once, and the footer dot shows "1 scheduled
  flow failing".

## 8. Compatibility

- **Existing projects:** nothing changes unless a `critters/` directory exists
  or a critter is picked. With no critter, `Scope::active` resolves exactly the
  old layers (§10 has the test).
- **Existing files:** `Response`, `WorkspaceState` and `RequestInfo` fields are
  new and `#[serde(default)]`, so old history, state and requests load. Request
  files without tags stay byte-identical when saved.
- **Bruno:** `critters/**/*.yml` aren't OpenCollection request files. Before
  starting, check that Bruno ignores or tolerates them. If it doesn't, the
  fallback is a single `critters.yml`, or a name Bruno skips.
- **A project with requests in a top-level `critters/` folder** would lose
  them from the list. That's unlikely, and it's mentioned in the release notes.

## 9. Risks

| Risk | Mitigation |
| --- | --- |
| Re-login sends a non-idempotent request twice | Only on 401, at most once, and the response warning says so. Fallback if it's a problem: limit it to GET/HEAD |
| Scheduled runs keep sending real requests | Opt-in per machine, 30s minimum, a global pause, visible in the Scheduled list |
| Scheduled captures change context while you work | Expected for a persona (a fresh token). `ContextChanged` keeps the UI honest |
| Captures from two places at once | `CONTEXT_WRITE` mutex in-process. The CLI at the same time isn't covered (accepted) |
| Watcher misses new directories on macOS/Linux | Commands reload the scheduler and stores directly instead of relying on `FsChanged` |
| HTML5 drag and drop in Tauri | `dragDropEnabled: false`, plus keyboard and `+` alternatives for every drag |
| Notification permission denied | In-app alerts still work, and the schedule control says notifications are off |

## 10. Tests

**Core (Rust)**

- Precedence:
  - With no critter, the result equals today's for the existing
    `layers_override_in_order` fixture.
  - With a critter: vars > secrets > context > step `with`.
  - The global captured layer is left out.
  - `sources_for_request` agrees with `resolve_for_request` for every key.
- Isolation: Alice's capture isn't visible to Bob, and captures with no
  critter still go to `captured.json`.
- Store:
  - Create/rename/duplicate/delete. Rename moves `.cactus/critters/<id>` and
    updates state; duplicate leaves out secrets, context and logs.
  - Invalid ids are rejected, and paths can't escape `critters/`.
  - Saving a flow keeps its comments; `save_flow_text` writes the text as
    typed; `parse_flow` reports line and column.
- `is_request_file` excludes `critters/`, and `list` ignores it.
- Overlay: `expect` replaces status assertions and keeps the others; `assert`
  appends; `capture` becomes set-variable actions.
- `run_flow` (against a local test server, as existing runner tests do): a
  token passes from step 1 to 2, steps after a failure are skipped, and a
  missing request file is a step error.
- Re-login: a 401 runs the login flow and resends once with `relogged` set; a
  401 inside the login flow doesn't trigger it; a failed login returns the
  original 401.
- Matrix: one cell per critter plus none, with missing secrets skipped.
- `Interval` parsing and printing, including the minimum. `missing_secrets`.
- `monitor::compare`: every row of §4.7, `alert.on` filtering, `None` never
  counted as a change, the log capped at 50.
- Serde: old `Response`/history/state JSON still loads.

**Desktop**

- Scheduler `reload` diffing, unit-tested on the computed wanted set (pure fn).
- Watcher filter test extended for `critters/` paths.

**CLI**

- `--as`, `flow`, `--all` argument parsing.
- JUnit output snapshot, including escaping.

**UI (vitest)**

- Every `domain/` module above. `flowDataflow` covers captures from both step
  `capture` and request actions, `{{}}` in each request part, and unknown
  requests.
- `flowStore` undo/redo. `fsSync` routing for `critters/**`.

## 11. Order of work

Each phase **starts** by re-checking the rows of §0 tagged with its number
against the current code, and **ends** green on the checks in
`ARCHITECTURE.md`.

1. **Core: model and precedence.** `critter/` model and store, `layout`,
   `EnvFile` variants, `Scope`, `layers`/`sources_for_request`, context with
   sources and the mutex, `tags`, `Response.critter`, state fields, tests.
2. **Core: running.** Step overlay, `select_values`, `run_flow`, `send_as` with
   re-login, `run_matrix`, `missing_secrets`, `duplicate`, `parse_flow`, tests.
3. **Core: monitoring.** `Trigger`/`Interval`/`AlertRule`, `monitor.rs`, run
   log, tests.
4. **Backend and CLI.** Commands, events, `variable_sources`, scheduler,
   notification plugin, `dragDropEnabled`, bindings regenerated, CLI flags and
   JUnit.
5. **Sprites.** `species.ts` for all eight, `CritterSprite`, `critters.css`, a
   dev-only page with every species × state × size × theme.
   **Checkpoint: review the look before building on it.**
6. **Ways in.** `CritterPicker`, `Ctrl+B`, `@`/`>` in `Ctrl+P`, the empty
   state, `NewCritterDialog`.
7. **Critters view.** List, header, Settings tab, Context tab (missing
   secrets, `EnvLayerEditor`, captured values with JWT and Undo).
8. **Showing who you're sending as.** Send button, accent line, response chip,
   history filter, `TemplateField` sources, Auth toggle and badge, Git icons.
9. **Flow builder.** Palette, lane, drag and drop with keyboard alternatives,
   inspector, data-flow chips, problems, undo/redo, YAML mode, run states,
   inline results.
10. **Schedules in the UI.** `ScheduleControl`, `RunTimeline`, Scheduled list,
    footer dot, alerts (toast, notification, unseen banner).
11. **Matrix.** Dialog, grouping, diff, "Add as check".
12. **Wrap-up.** Accessibility pass, demo critters (an admin and a viewer, a
    flow, a scheduled health check) in `examples/demo`, `ARCHITECTURE.md`
    (on-disk table, precedence, `critter`/`monitor` modules, scheduler).
