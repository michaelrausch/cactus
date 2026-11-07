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

| Path                      | What                                                                          | Committed |
| ------------------------- | ----------------------------------------------------------------------------- | --------- |
| `opencollection.yml`      | The collection: its defaults (`request.headers`, `auth`) and shared variables | yes       |
| `environments/<name>.yml` | A named environment                                                           | yes       |
| `<folder>/folder.yml`     | A folder's name, `seq` and defaults                                           | yes       |
| `**/*.yml`                | Requests (every other `.yml`)                                                 | yes       |
| `.cactus/secrets.json`    | Local secrets, the values of `secret: true` variables                         | no        |
| `.cactus/captured.json`   | Values written by set-variable actions                                        | no        |
| `.cactus/state.json`      | Active environment                                                            | no        |
| `.cactus/history/*.json`  | Last responses per request (`a/b.yml` → `a__b.json`)                          | no        |

All path rules live in `core/src/layout.rs`. Request paths are a validated
newtype (`ScenarioPath`) that can never escape the project. Requests are listed
and run in tree order: at each level, folders and requests sorted together by
`seq`, as Bruno does.

**Variable precedence**, lowest first: collection variables → active
environment → folder variables (outermost first) → request `runtime.variables`
→ secrets → captured (see `env::resolve_for_request`). `{{name}}` substitution
is a single pass. Headers and auth are inherited the same way: collection, then
folders, then the request; `auth: inherit` takes the nearest one set.

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
| `env`                                             | `layers` (env layers and merge), `capture`, `template` (`Substituter`)                                                                             |
| `http`                                            | `request` (scenario + inherited defaults → reqwest request), `send`, `response`, `expression` (`res.*`), `assertions` (actions, checks, pass/fail) |
| `import`                                          | Postman and OpenAPI → requests                                                                                                                     |
| `git`                                             | Status, stage/unstage, discard, diffs, commits and local branches in the project's repository (`git` feature)                                      |
| `runner`                                          | Send + persist captures + record history; folder runs                                                                                              |
| `history`, `recent`, `state`, `storage`, `layout` | Persistence                                                                                                                                        |

Errors are typed (`core/src/error.rs`) and reach the UI as their message.

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
- `state.rs`: shared HTTP client and the single cancellable operation.
- `watcher.rs`: emits `FsChanged` when project files change.
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

Nothing is saved automatically. Edits live in `editorStore` buffers (the open
scenario plus any other with unsaved edits), which are dirty until saved with
Ctrl+S (`actions/saving.ts`). If a file changes on disk under a dirty buffer it
is flagged as a conflict, resolved by reloading from disk or overwriting. The editor remounts via `key={path:editorVersion}`
whenever its draft state must reset.

## Common changes

**Add a backend command.** Add the function in `core`, then an adapter in
`src-tauri/src/commands/`, then register it in `bindings.rs`. Run
`cargo test -p cactus` to regenerate the bindings, then call it via
`backend.yourCommand(...)` from an action.

**Edit another OpenCollection field.** Add it to `core/src/scenario/model.rs`
(or `collection/model.rs`) under its spec name, with
`#[serde(default, skip_serializing_if = ...)]` so files without it still load
and stay minimal, and `lenient_string` for strings hand-written YAML may type as
numbers. Until a field is in the model, saves leave it untouched. Regenerate the
bindings, then add an editor tab under `ui/src/features/editor/tabs/` that takes
`ScenarioTabProps`. The UI must hold a request exactly as the backend writes it
(empty lists left out, query params mirroring the URL), or every save looks
like an outside change and reloads the editor.

## Checks

```sh
cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace
cd ui && npm run typecheck && npm run lint && npm run format:check && npm test && npm run build
```

The UI lints with Oxlint because the project uses TypeScript 7, which
typescript-eslint does not support yet.
