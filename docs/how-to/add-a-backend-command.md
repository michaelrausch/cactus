# Add a backend command or event

The desktop UI talks to Rust through Tauri commands (UI → backend, request and
response) and events (backend → UI, pushed). Both are typed end to end:
tauri-specta generates `ui/src/api/bindings.ts` from the Rust signatures.

The path a new command takes:

```
core function  →  src-tauri adapter  →  bindings.rs  →  (regenerate)  →  UI action  →  component
```

This guide adds a `delete_folder` command, then an event.

## 1. Write the behaviour in core

All logic goes in `cactus-core`, so the CLI (and tests) can use it too. Put it
in the module that owns the data, here `core/src/scenario/store.rs`, next to
`create_folder`:

```rust
/// Delete a request folder and everything in it.
pub fn delete_folder(project: &Path, dir: &str) -> Result<()> {
    let path = ProjectPath::parse(dir.trim().trim_matches('/'))?;
    if path.as_str().is_empty() || !is_request_folder(path.as_str()) {
        return Err(Error::InvalidPath(dir.to_string()));
    }
    let full = path.resolve(project);
    fs::remove_dir_all(&full).map_err(Error::file(&full))
}
```

Rules of thumb:

- **Take paths inside the project as `ProjectPath`** (or parse a string into
  one). It is validated so it can never point outside the project.
- **Return `cactus_core::Result`.** If no existing `Error` variant fits, add one
  to `core/src/error.rs`. Its `#[error("…")]` message is what the user sees,
  so keep it short and plain.
- **Paths of files on disk come from `core/src/layout.rs`**, never hand-built.
- **Re-export** from the module's `mod.rs` if the store function isn't already
  public through it.
- Add a unit test beside the function (`tempfile::tempdir()` for a project).

## 2. Add the Tauri adapter

In the matching file under `src-tauri/src/commands/`, here `scenario.rs`:

```rust
#[tauri::command(async)]
#[specta::specta]
pub fn delete_folder(project: PathBuf, dir: String) -> Result<()> {
    scenario::delete_folder(&project, &dir)
}
```

- `#[tauri::command(async)]` runs it off the main thread; use it for anything
  touching disk or network. Write `pub async fn` when you need to `.await`.
- **Arguments are owned** (`PathBuf`, `String`, `ProjectPath`): Tauri
  deserialises them per call. Every argument type and the return type must
  implement `specta::Type`; core types get it from the `specta` feature
  (`#[cfg_attr(feature = "specta", derive(specta::Type))]`).
- **No logic here** beyond converting arguments. A doc comment becomes the
  JSDoc on the generated TypeScript function.
- Need shared state? Add `state: State<'_, AppState>` (the HTTP client, the
  scheduler, the cancellable operation). Need to emit events? Add
  `app: AppHandle`. Neither appears in the TypeScript signature.

A new file under `commands/` also needs a `pub mod` line in `commands/mod.rs`.

## 3. Register it

Add it to `collect_commands![…]` in `src-tauri/src/bindings.rs`:

```rust
scenario::create_folder,
scenario::delete_folder,
scenario::list_folders,
```

## 4. Regenerate the bindings

```sh
cargo test -p cactus
```

`ui/src/api/bindings.ts` now has `commands.deleteFolder(project, dir)`
(camelCase). Commit the regenerated file; CI fails if it differs from what
the Rust code produces.

If the command returns a new type, add a short alias in `ui/src/api/client.ts`
(`export type Foo = Bindings.Foo_Serialize;`) so the rest of the UI doesn't
import from `bindings.ts` directly.

## 5. Call it from an action

Components never call the backend. Write (or extend) a function in
`ui/src/actions/` that calls it, updates stores, and reports errors:

```ts
// ui/src/actions/scenario.ts
export async function deleteFolder(dir: string): Promise<void> {
  const { project } = useProjectStore.getState();
  if (!project) return;
  const deleted = await succeeded(
    () => backend.deleteFolder(project, dir),
    "Couldn't delete the folder",
  );
  if (deleted) await refreshEntries();
}
```

Error helpers (`ui/src/lib/errors.ts`):

| Helper                    | Use for                                                                                             |
| ------------------------- | --------------------------------------------------------------------------------------------------- |
| `reportErrors(work, ctx)` | User-initiated work. Shows a toast on failure, resolves to `undefined`.                             |
| `succeeded(work, ctx)`    | Same, when all you need back is "did it work".                                                      |
| `warnQuietly(work, ctx)`  | Background work whose failure only costs a hint (highlighting, previews). Logs instead of toasting. |

Never call `toast.error` or `console.warn` directly.

Then call the action from a component in `features/`:

```tsx
<DropdownMenuItem onSelect={() => void deleteFolder(dir)}>
  Delete
</DropdownMenuItem>
```

Destructive actions should confirm first: see `features/dialogs/ConfirmDialog.tsx`
and how existing deletes use `useUiStore.getState().askConfirm(…)`.

---

## Adding an event

Use an event when the backend needs to tell the UI something it didn't ask
for: progress of a long run, a scheduled run finishing, or a write the file
watcher can't see (it ignores `.cactus/`).

### 1. Define it

In `src-tauri/src/events.rs`:

```rust
/// A folder was deleted from outside the editor.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct FolderDeleted {
    pub dir: String,
}
```

### 2. Register it

Add it to the `use crate::events::{…}` list and to `collect_events![…]` in
`src-tauri/src/bindings.rs`.

### 3. Emit it

From a command (take `app: AppHandle`) or anywhere with an app handle:

```rust
use crate::events::{FolderDeleted, notify};

notify(&FolderDeleted { dir: dir.clone() }, &app);
```

`notify` logs and carries on if emitting fails, so an event never fails the
command.

### 4. Listen in the UI

Regenerate bindings (`cargo test -p cactus`), then subscribe in
`ui/src/hooks/useAppLifecycle.ts`, alongside the other listeners, and hand
the payload to an action or store method:

```ts
backendEvents.folderDeleted.listen(({ payload }) => folderDeleted(payload.dir)),
```

The listeners there are set up once for the app's lifetime and torn down on
unmount. For an event that matters only during one operation (like
`runProgress` during a folder run), `listen` inside the action and call the
returned `unlisten` when it finishes; see `actions/request.ts`.

## Checklist

- [ ] core function, with a test, returning `cactus_core::Result`
- [ ] new `Error` variant if needed, with a short message
- [ ] adapter in `src-tauri/src/commands/<area>.rs`
- [ ] entry in `collect_commands!` (and `collect_events!` for events)
- [ ] `cargo test -p cactus`, commit `bindings.ts`
- [ ] type alias in `api/client.ts` for new types
- [ ] action in `ui/src/actions/`, called from a component
