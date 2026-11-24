# Encrypted secrets

Status: built, **off by default** behind the `encryption` Cargo feature and
the `CACTUS_ENCRYPTION=1` UI build flag. To try it:
`CACTUS_ENCRYPTION=1 cargo tauri dev --features encryption`, and
`cargo build -p cactus-cli --features encryption`.

**As built, there is no `vault.json`**: every sealed value
carries its own salt (`enc:v1:<salt>:<nonce‖ciphertext‖tag>`), and whether
encryption is on lives in `.cactus/state.json` as a setting with no key
material. Where this plan still describes a vault (§2.1, the KEK/DEK
envelope, re-wrapping weaker KDF parameters, `VaultMissing`,
`UnsupportedVaultVersion`), read it with the changes in
[§13 As built](#13-as-built).

A project can turn on **secret encryption** and set a password. After that,
each secret's value is encrypted in place inside its `secrets.json`, and
cactus asks for the password once per session before it sends a request that
needs one. This covers the project's `.cactus/secrets.json` and, once critters
exist, each critter's `.cactus/critters/<id>/secrets.json`
([critters plan](critters-plan.md)).

```json
{
  "API_KEY": "enc:v1:Jx0c3V…",
  "CLIENT_SECRET": "enc:v1:9aQm1L…"
}
```

Names stay readable; values don't. Projects that don't turn it on behave
exactly as they do today.

> **Re-validate before building.** This plan was written against commit
> `a43d433` ("Implement UI audit results"), with the critters plan unbuilt.
> The names, signatures and behaviour quoted here are a snapshot, not a
> contract. Before each phase, check what it relies on (§0) against the
> current code. Where the code has moved on, follow the code and update this
> plan. In particular, check whether critters (or part of them) have landed:
> §9 says what changes in each case.

## 0. Assumptions to re-check

Run `git log a43d433..HEAD --stat` and read each file below that appears in it.

| Assumption | Where | Phase |
| --- | --- | --- |
| Secrets are a flat `Env` (`BTreeMap<String, String>`) in `.cactus/secrets.json`, read and written only through `env::load`/`env::save` with `EnvFile::Secrets` | `core/src/env/layers.rs`, `core/src/layout.rs` | 1 |
| `secret: true` variables have no value in YAML and are left out by `is_active()` | `core/src/collection/model.rs` | 1 |
| `resolve_for_request` is the only place layers are merged (critters: `layers()`/`Scope`) | `core/src/env/layers.rs` | 2 |
| `storage::write_atomic` writes via temp file and rename | `core/src/storage.rs` | 1 |
| `.cactus/` is git-ignored by its own `.gitignore` and ignored by the watcher | `core/src/layout.rs`, `src-tauri/src/watcher.rs` | 1, 3 |
| Errors reach the UI only as their message (no kind) | `core/src/error.rs`, `ui/src/actions/` | 3, 4 |
| `variable_names` resolves every layer just to get names (critters: replaced by `variable_sources`) | `src-tauri/src/commands/env.rs` | 2 |
| `AppState` holds the client, cancel slot and watcher; `open_project` replaces the watcher | `src-tauri/src/state.rs`, `src-tauri/src/commands/project.rs` | 3 |
| `tokio` in `src-tauri` lacks the `time` feature (critters phase 4 adds it) | `src-tauri/Cargo.toml` | 3 |
| CLI commands are `run`, `send`, `import` (critters adds `flow`, `--as`) | `cli/src/args.rs` | 3 |
| `EnvLayerEditor` saves a layer as a whole `Env`, and hides secrets until revealed | `ui/src/features/env/EnvLayerEditor.tsx` | 4 |
| `SettingsDialog` has Theme and About tabs; `settingsStore` persists only app-wide prefs | `ui/src/features/settings/`, `ui/src/stores/settingsStore.ts` | 4 |
| `Response.url` is the URL after substitution, and history stores the whole `Response` | `core/src/http/response.rs`, `core/src/history.rs` | — (threat model) |

If an assumption no longer holds, handle it as the critters plan says (§0
there): follow renames, redesign on changed behaviour, reuse what exists.

## 1. Goals, non-goals and threat model

**Goals**

- Opt-in per project, per machine: a switch and a password.
- While encryption is on, no secret value stays on disk in plaintext past the
  next unlock.
- Same files, same paths, same shape. An older cactus never loses data.
- One unlock per session. Changing the password is instant and never touches
  `secrets.json`.
- Every secrets file (the project's and every critter's) goes through one
  read/write path, so nothing can bypass encryption by accident.
- Locked is a clear, recoverable state everywhere: sends, runs, flows,
  schedules, matrix, CLI. Anything that only needs names keeps working.

**Non-goals (for now)**

- Encrypting `captured.json`, critter `context.json`, history or run logs (see
  threat model and §10).
- Hiding secret names. They're already in committed YAML as `secret: true`
  variables.
- Storing the password or key in the OS keychain ("remember on this machine").
  The format leaves room for it (§2.1).
- Sharing encrypted secrets through git. `.cactus/` stays uncommitted; this is
  encryption at rest, not a team secret store.
- Password recovery. A forgotten password means resetting the secrets.

**What it protects against**

- The project folder being copied, zipped, synced (Dropbox, iCloud, OneDrive)
  or backed up with `.cactus/` inside it.
- Other tools, scripts and coding agents reading files in the project.
- Casual access to the disk by someone without the password.

**What it doesn't protect against** (stated briefly in the UI's enable dialog)

- Malware running as the user while cactus is unlocked.
- Values that leave the secrets layer: a secret substituted into a URL is in
  `Response.url` in history; tokens a login returns are in `captured.json` or
  a critter's `context.json`, in plaintext.
- Old plaintext copies made before encryption was turned on (backups, sync
  history). Deleting or overwriting a file on an SSD or APFS doesn't erase it.
  The enable dialog suggests rotating credentials if that matters.
- Names and approximate value lengths, which are visible.

## 2. On disk

| Path | What | Change |
| --- | --- | --- |
| `.cactus/vault.json` | KDF parameters, the wrapped data key, auto-lock setting | New. **Its presence is what "encryption on" means.** |
| `.cactus/secrets.json` | The project's secrets | Same file. With encryption on, values are sealed strings |
| `.cactus/critters/<id>/secrets.json` | A critter's secrets | Same, once critters exist |

### 2.1 `vault.json`

```json
{
  "version": 1,
  "kdf": { "alg": "argon2id", "memoryKib": 65536, "iterations": 3, "parallelism": 1, "salt": "base64…" },
  "wrappedKey": { "nonce": "base64…", "ciphertext": "base64…" },
  "autoLockMinutes": 15
}
```

- **Envelope encryption.** A random 256-bit data key (DEK) encrypts every
  value. The password, through Argon2id, gives a key-encryption key (KEK)
  that encrypts the DEK. Changing the password rewrites only `wrappedKey` and
  `salt`, so it's atomic, instant, and safe while a scheduled flow is running.
- KDF parameters are stored, so they can be raised later. A vault with weaker
  parameters than the current default is re-wrapped on the next successful
  unlock.
- A wrong password fails the AEAD tag check on `wrappedKey`. No separate
  verifier hash is stored.
- `autoLockMinutes`: `null` means never. It lives here because it's a security
  setting of this vault on this machine, not an app-wide preference.
- Room for later: a `keychain: true` field for "remember on this machine",
  holding the DEK in the OS keychain.

### 2.2 Sealed values

```
enc:v1:<base64url(nonce ‖ ciphertext ‖ tag), no padding>
```

- Cipher: XChaCha20-Poly1305 with the DEK, and a fresh random 24-byte nonce
  for every value on every write (no counter to persist or get wrong).
- **Associated data: `cactus/secrets/v1/<name>`.** Binding the variable name
  means a value can't be moved to another key (e.g. `PROD_KEY`'s value pasted
  under `DEV_KEY`) without failing to open. The critter id isn't included, so
  renaming a critter needs no re-encryption.
- Renaming a secret in the editor re-seals its value under the new name (the
  UI saves the whole layer, and `write` seals every value, so this happens
  anyway).
- A value is sealed if and only if it starts with `enc:v1:`. Interpretation
  depends on whether a vault exists:

| Vault | Value | Meaning |
| --- | --- | --- |
| none | plain | A secret, as today |
| none | `enc:v1:…` | `Error::VaultMissing` naming the variable. Never sent as a literal. The UI offers reset |
| present | `enc:v1:…` | Opened with the DEK; fails as `VaultCorrupt` naming the variable if the tag doesn't verify |
| present | plain | A **plain value**: still works, and is sealed on the next unlock (§3.3) |

### 2.3 Crates

All RustCrypto, pure Rust, no OpenSSL, which keeps the same cross-platform
story as vendored `git2`:

- `chacha20poly1305` (XChaCha20-Poly1305), `argon2`, `getrandom` (or
  `rand_core::OsRng`), `zeroize`, `base64`, in `core`.
- `rpassword` in `cli` for the TTY prompt.

`unsafe_code = "forbid"` applies to our code only, so these are fine.

## 3. Core (`core/`)

### 3.1 New module `secrets/`

```
core/src/secrets/
  mod.rs      re-exports, SecretFile, SecretsStatus
  cipher.rs   derive_kek (Argon2id), seal_value/open_value, DataKey (zeroize on drop)
  vault.rs    VaultFile (de)serialise, enable/disable/change_password/reset/set_auto_lock
  keyring.rs  process-wide unlocked keys, unlock/lock/lock_all
  store.rs    read/names/write: the only code that touches secrets files
```

```rust
/// Every secrets file a project can have. Critter ids are validated as env names.
pub enum SecretFile { Project, Critter(String) }

impl SecretFile {
    fn path(&self, project: &Path) -> PathBuf;
    /// The project file plus every `.cactus/critters/*/secrets.json` found on
    /// disk. Scanning the workspace means this works before critters exist.
    fn all(project: &Path) -> Vec<SecretFile>;
}

#[derive(Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum SecretsStatus { Off, Locked, Unlocked }

pub fn status(project: &Path) -> SecretsStatus;

/// Values, opened. Locked and the file holds a sealed value → Err(SecretsLocked).
/// Locked with only plain values (or an empty file) → Ok, so a project with
/// encryption on but nothing sealed never asks for the password.
pub fn read(project: &Path, file: &SecretFile) -> Result<Env>;

/// Names only. Never needs the key, works while locked.
pub fn names(project: &Path, file: &SecretFile) -> Result<Vec<String>>;

/// Off: plaintext, as today. Unlocked: seals every value. Locked: Err(SecretsLocked).
pub fn write(project: &Path, file: &SecretFile, env: &Env) -> Result<()>;
```

`EnvFile::Secrets` in `env::load`/`env::save` calls `secrets::read`/`write`
with `SecretFile::Project`. Nothing else reads `layout::secrets_file`; the
path function moves into `secrets` and becomes private, so the compiler
enforces this.

Writing seals every value with a fresh nonce, including unchanged ones. The
file is local and uncommitted, so churn doesn't matter, and it keeps `write`
trivially correct.

### 3.2 Keyring

```rust
static KEYRING: Mutex<HashMap<PathBuf /* canonical project */, Arc<DataKey>>>;

pub fn unlock(project: &Path, password: &SecretString) -> Result<UnlockReport>;  // Err(WrongPassword)
pub fn lock(project: &Path);
pub fn lock_all();

pub struct UnlockReport { pub sealed_plain_values: u32 }  // see §3.3
```

- **Why a process-wide keyring and not a key passed through every call:** the
  key must reach `env::load`, `resolve_for_request`, the runner, critters'
  `Scope`, flows, the matrix and the scheduler. Passing it explicitly would
  change every one of those signatures. This mirrors the critters plan's
  process-wide `CONTEXT_WRITE` mutex: one piece of process state, owned by
  core, with a small API. Keys are `Arc<DataKey>`, so a send in progress keeps
  working if the user locks mid-request. The key is zeroized when the last
  reference drops.
- Projects are keyed by canonical path so `/p` and `/p/` don't unlock
  separately.
- `unlock` is CPU-bound (Argon2id, about 0.3–0.8 s). Core keeps it sync; the
  desktop runs it in `spawn_blocking`.
- Unlocking with weaker-than-default KDF parameters re-wraps the DEK (§2.1).

### 3.3 Turning it on and off

Every operation below takes a process-wide `SECRETS_WRITE: Mutex<()>`, which
`write` also takes, so a save can't race a migration.

Because plain values are valid while encryption is on, **every intermediate
state is a valid state.** A crash at any point leaves secrets readable and
needs no recovery code. At worst, some values are plain and get sealed at the
next unlock.

**`enable(project, password)`**: fails if a vault exists, or the password is
shorter than 8 characters.

1. Generate the DEK and salt, derive the KEK, and write `vault.json` with
   `write_atomic`. Encryption is now on, with every value plain.
2. Put the DEK in the keyring (enabling leaves you unlocked).
3. For each `SecretFile::all`, seal its values and write it atomically, then
   read it back and compare (a round-trip check).

**`disable(project, password)`**: needs the password even if unlocked, so a
walk-up user can't turn it off.

1. Unlock (verifies the password). Open every file's values and write them
   back as plaintext, atomically.
2. Delete `vault.json` and lock.

A crash between the two leaves a vault with plain values, which the next
unlock would seal again. Disable is simply retried.

**`change_password(project, current, new)`**: verify `current`, re-wrap the
DEK under a new salt, and write `vault.json` atomically. `secrets.json` files
aren't touched.

**`reset(project)`** (forgotten password): remove every sealed value from
every file (plain values stay), then delete `vault.json`. Encryption is off,
and only the sealed secrets are gone. Removing values before deleting the
vault means nothing is ever left as an orphaned `enc:v1:` value. The UI makes
you type the project name to confirm.

**Plain values while on.** These come from a crash during enable, from an
older cactus that saved a secret (§7), or from someone editing the file by
hand. They keep working. `unlock` seals any it finds and reports the count
(`UnlockReport`), and the UI shows a toast: "Encrypted 2 values that were
saved as plain text." `status` doesn't need a separate leftovers state.

### 3.4 Resolving while locked

- `secrets::read` returns `Err(Error::SecretsLocked)` while locked if the file
  holds a sealed value. `resolve_for_request` propagates it, so **a send fails
  before anything goes on the wire.** Sending a literal `{{token}}` to a
  server, or silently sending without auth, is worse than asking for the
  password.
- Anything that only needs names uses `secrets::names` and works while locked:
  `variable_names` (critters: `sources_for_request`), the UI's `{{name}}`
  tooltips ("secret"), and the critters plan's `missing_secrets`.

With critters, this is a parameter on the single `layers()` fold, so the two
uses can't disagree:

```rust
enum Need { Values, Names }  // Names: secret layers come from secrets::names, with empty values
fn layers(project, scope, inherited, request, need: Need) -> Result<Vec<(VarSource, Env)>>;
```

`resolve_for_request` uses `Values` and `sources_for_request` uses `Names`.
Without critters, `resolve_for_request` keeps its signature (always `Values`),
and `variable_names` switches to a small names-only fold.

### 3.5 Errors

New `Error` variants, with messages written for the UI:

- `SecretsLocked`: "Secrets are locked. Unlock them to send."
- `WrongPassword`: "Wrong password."
- `PasswordTooShort`: "Use at least 8 characters."
- `EncryptionAlreadyOn` / `EncryptionOff`
- `VaultMissing { name }`: "`{name}` is encrypted but this project has no
  vault (.cactus/vault.json)."
- `VaultCorrupt(String)`: a vault that doesn't parse, or a value whose tag
  fails with the right key (names the variable). Never "fixed"
  automatically; the UI offers reset.
- `UnsupportedVaultVersion(u32)`: a vault written by a newer cactus.

## 4. Desktop backend (`src-tauri/`)

### 4.1 Commands (`commands/secrets.rs`)

| Command | Core |
| --- | --- |
| `secrets_status` | `secrets::status` → `{ status, autoLockMinutes }` |
| `enable_secrets_encryption(project, password)` | `secrets::enable` |
| `disable_secrets_encryption(project, password)` | `secrets::disable` |
| `change_secrets_password(project, current, new)` | `secrets::change_password` |
| `unlock_secrets(project, password)` | `secrets::unlock` in `spawn_blocking` → `UnlockReport` |
| `lock_secrets(project)` | `secrets::lock` |
| `reset_secrets(project)` | `secrets::reset` |
| `set_auto_lock(project, minutes: Option<u32>)` | `vault::set_auto_lock` |
| `secret_names(project, file: EnvFile)` | `secrets::names`, for the locked editor view |

- Passwords cross IPC once, as `String`, and are moved into `SecretString`
  immediately. They're never logged or put in errors.
- `open_project` calls `secrets::lock_all()` first: switching projects locks
  the previous one.
- New event `SecretsStatusChanged { status }`, emitted on unlock, lock,
  auto-lock, enable, disable and reset.

### 4.2 Auto-lock

- In the backend, because the key lives there and a hidden webview's timers
  can be throttled.
- `AppState` gains `secrets_activity: Mutex<Instant>`. Interactive commands
  that use secret values (`send_request`, `run_folder`, `load_env` of a
  secrets layer, critters' `run_flow`/`run_matrix`) update it. **Scheduled
  runs don't**, so a schedule can't keep the vault open forever.
- A task started on `open_project` checks once a minute. Past
  `autoLockMinutes`, it locks and emits `SecretsStatusChanged`. This needs
  `tokio`'s `time` feature (also added by critters phase 4; whichever lands
  first adds it).
- Default for new vaults: 15 minutes. Options: never, 5, 15, 60, 240.

## 5. CLI (`cli/`)

- `run` and `send` (and critters' `flow`): when the project's vault exists and
  a secrets file holds a sealed value, the CLI gets the password in this
  order:
  1. `--password-stdin` (reads one line), for scripts.
  2. The `CACTUS_SECRETS_PASSWORD` environment variable, for CI.
  3. An interactive prompt (`rpassword`) when stdin is a TTY.
  4. Otherwise it fails before sending, exits non-zero, and says: "Secrets
     are encrypted. Pass --password-stdin or set CACTUS_SECRETS_PASSWORD."
- A wrong password fails before anything is sent.
- The CLI doesn't seal plain values on unlock. It's read-only towards secrets,
  so a CI run never rewrites local files.
- `cactus-cli secrets <project> status|enable|disable|change-password` for
  headless machines. It prompts on a TTY and uses `--password-stdin`
  otherwise.
- Concurrency with the app is as today: the CLI holds its own key in memory.
  A CLI write and an app write at the same moment can still race (accepted, as
  in the critters plan).

## 6. UI (`ui/src/`)

### 6.1 Principles

1. Invisible until turned on. With encryption off, the only new thing is one
   line in the Secrets layer: "Stored as plain text in .cactus/. Encrypt…".
2. Ask for the password when it's needed, not on open. Opening a project
   doesn't prompt; the first send that needs a sealed value does.
3. Locked is never an error toast. It's a dialog, and then the action you
   asked for carries on.
4. Names are always visible. Locked hides values, not the list.
5. No recovery, said once and plainly when enabling. Reset is always reachable
   from the unlock dialog.

### 6.2 Structure

```
stores/secretsStore.ts          status, autoLockMinutes
actions/secrets.ts              refresh, enable, disable, unlock, lock, changePassword, reset,
                                setAutoLock, ensureUnlocked()
features/secrets/
  EnableEncryptionDialog.tsx    password, confirm, warning, what it does and doesn't cover
  UnlockDialog.tsx              password, error shake, "Forgot password?" → reset
  ChangePasswordDialog.tsx
  ResetSecretsDialog.tsx        type the project name to confirm
  LockButton.tsx                sidebar footer, beside EnvPicker
  SecurityTab.tsx               in SettingsDialog, shown when a project is open
domain/password.ts              length and match validation (tested)
```

### 6.3 `ensureUnlocked()`

- `actions/secrets.ts` exports `ensureUnlocked(): Promise<boolean>`. If
  `status` is `locked`, it opens `UnlockDialog` and resolves when the dialog
  closes (`true` if unlocked, `false` if cancelled). Otherwise it resolves
  `true` straight away.
- It's called before `send`, `run folder`, and critters' run flow, run matrix,
  "Log in again" and "Run now". Cancelling cancels the action quietly.
- After unlocking, if `UnlockReport.sealedPlainValues > 0`, the UI shows
  `toast.info("Encrypted N values that were saved as plain text")`.
- Since errors reach the UI only as messages, the UI doesn't parse
  `SecretsLocked`. It relies on the pre-check, and the backend refusal is the
  safety net. If the refusal ever happens (e.g. auto-lock in the gap between
  the check and the send), it shows as a normal error.

### 6.4 Where it shows

- **Sidebar footer:** `LockButton`, only when encryption is on. An open
  padlock when unlocked (click to lock now), a closed padlock when locked
  (click to unlock). Tooltip: "Secrets unlocked · locks after 15 min idle".
- **Env dialog, Secrets layer (`EnvLayerEditor`):**
  - Off: a muted line, "Stored as plain text in .cactus/secrets.json", and an
    **Encrypt…** link opening `EnableEncryptionDialog`.
  - Locked: the names from `secret_names`, each value shown as "encrypted",
    read-only, with an **Unlock** button above. Editing needs the key, because
    new values must be sealed.
  - Unlocked: today's table, with the existing reveal toggle, and a small
    "encrypted" note under the layer title.
- **Settings → Security** (project section): status, Encrypt / Turn off
  encryption, Change password, Auto-lock select, Lock now, Reset secrets.
- **`{{name}}` tooltips:** a secret shows its source as "secret" whether
  locked or not, since names come from `secrets::names`.

### 6.5 Enable dialog

- Password and confirm, minimum 8 characters, no other rules or meter.
- Three short lines: what's encrypted (values, not names); that there's no
  recovery; that captured tokens and history aren't encrypted and old copies
  may exist in backups.
- Auto-lock select (default 15 minutes).
- **Encrypt** runs `enable`, closes the dialog, and toasts "Secrets
  encrypted".

## 7. Compatibility

- **No vault:** every code path is today's. `secrets::read` with status `Off`
  reads `secrets.json` exactly as `read_json` does. The existing
  `layers_override_in_order` test must pass unchanged.
- **Older cactus on an encrypted project:** it parses `secrets.json` fine and
  sees `enc:v1:…` strings as values.
  - Sending substitutes the ciphertext, so auth fails. No secret leaks.
  - Saving the layer writes every untouched `enc:v1:` string back unchanged,
    so nothing is lost. A value edited there is saved plain and gets sealed at
    the next unlock in a current build.
- **Newer vault version:** `UnsupportedVaultVersion`; the UI says to update
  cactus. Never overwritten.
- **Bruno, git:** unaffected; both ignore `.cactus/`.

## 8. Risks

| Risk | Mitigation |
| --- | --- |
| Forgotten password | Stated when enabling; Reset is one click from the unlock dialog and keeps plain values; secrets are local values you can re-enter |
| Crash mid-migration | Every intermediate state is valid (plain values allowed while on); atomic writes per file; round-trip check after sealing |
| Auto-lock interrupts scheduled flows | Schedules wait rather than fail (§9.2); the footer says why; auto-lock can be set to never |
| Users assume tokens and history are encrypted too | Said in the enable dialog and here; §10 lists it as the next step |
| Weak passwords | Argon2id at 64 MiB / 3 iterations makes guessing slow; parameters can be raised later, and old vaults upgrade on unlock |
| Password in IPC or logs | Moved into `SecretString` at the boundary; no `Debug` on types holding it; errors never include it |
| A lock mid-send | The send in progress holds an `Arc` to the key and finishes; the next one asks |
| Ciphertext sent by an older build | Only auth fails; the ciphertext reveals nothing without the vault and password |

## 9. Critters

This plan works whichever lands first. The rule is the same either way:
**every critter secrets file goes through `secrets::read`/`names`/`write`
with `SecretFile::Critter(id)`**, and everything that resolves values handles
`SecretsLocked`.

### 9.1 Amendments to the critters plan

If encryption lands first, amend the critters plan as below before its phase
1. If critters land first, make these changes as part of this plan's phase 5.

| Critters plan | Change |
| --- | --- |
| §0 | Add rows: "secrets files are read only via `secrets::read`/`names`/`write`"; "`resolve_for_request` fails with `SecretsLocked`" |
| §2 table | Note that values in `.cactus/critters/<id>/secrets.json` are sealed when encryption is on |
| §3.1 `layers()` | Takes `Need::{Values, Names}` (§3.4 here). `resolve_for_request` uses `Values`, `sources_for_request` uses `Names` |
| §3.2 `EnvFile::CritterSecrets` | `load`/`save` call `secrets::read`/`write(SecretFile::Critter(id))`, not `read_json`/`write_json` |
| §4.1 store | `rename` and `delete` already move or remove all of `.cactus/critters/<id>/`; no re-encryption is needed, because the critter id isn't in the AAD. `duplicate` already copies no secrets. `SecretFile::all` finds critter files by scanning, so `enable`/`disable` cover critters created at any time |
| §4.1 `missing_secrets` | Uses `secrets::names`, so it works while locked. A name with a sealed value counts as present |
| §4.3 locking | Secret writes use their own `SECRETS_WRITE` mutex, separate from `CONTEXT_WRITE` |
| §4.6 `send_as` | A locked vault fails at resolve, before the first send, so re-login is never attempted with missing secrets |
| §4.6 `run_matrix` | Refuses as a whole while locked if any critter or project secret is sealed (the UI unlocks first), instead of failing each critter one by one |
| §5.1 | `run_flow`, `run_matrix` and `run_scheduled_now` update `secrets_activity`. `variable_sources` works while locked |
| §5.2 scheduler | See §9.2 |
| §6 CLI | `flow` and `--as` use the same password sources (§5 here) |
| §7.6 Context tab | The missing-secrets block works while locked; entering a value needs **Unlock** first. The `critterSecrets` sub-tab of `EnvLayerEditor` uses the locked view (names, "encrypted", Unlock) |
| §7.9 Matrix, §7.6 "Log in again", §7.8 Run / Run now | Call `ensureUnlocked()` first |
| §9 Risks | Add "auto-lock pauses schedules" |
| §10 Tests | Add the critters rows from §11 here |

### 9.2 Scheduled flows while locked

- At each tick, if the vault is locked and the flow's resolution would need a
  sealed value, the task **doesn't run the flow**. Nothing is sent, nothing is
  appended to the run log, and no `MonitorAlert` fires. A locked vault isn't
  the API failing, and recording it as a failure would page you for walking
  away.
  - In practice: the tick tries to resolve the first step and gets
    `SecretsLocked` before anything is sent.
  - A flow whose critter and project have no sealed values keeps running while
    locked.
- `schedule_states` reports `waiting: "secretsLocked"` per task. The footer's
  scheduled indicator reads "Schedules waiting: secrets locked", and the
  Scheduled list shows a padlock instead of the next-run countdown.
- On `SecretsStatusChanged` to unlocked, tasks pick up at their next tick. No
  catch-up runs (consistent with `MissedTickBehavior::Skip`).
- Scheduled runs don't reset the auto-lock timer (§4.2). Anyone who wants
  monitoring to run unattended sets auto-lock to never; the Security tab says
  so when schedules are enabled.

## 10. Later

- **Ask only for requests that use a secret:** today a locked vault blocks any
  send whose layers hold a sealed value. With names visible, substitution
  could open values lazily, and only prompt when a request actually uses
  `{{SOME_SECRET}}`.
- **Encrypt captured values:** `captured.json` and critter `context.json`, with
  the same sealed-value format. Deferred because scheduled flows write
  context, and while locked they'd have to write plain values. The
  plain-values-allowed rule makes that workable, but it needs its own design.
- **History and run logs:** at minimum, redact known secret values from
  `Response.url` and headers before recording history. That's cheaper than
  encryption and fixes the main leak.
- **Remember on this machine:** keep the DEK in the OS keychain (`keyring`
  crate) behind a `keychain: true` vault flag.
- **Typed errors to the UI:** a `kind` on serialised errors would let the UI
  react to `SecretsLocked` directly instead of pre-checking.

## 11. Tests

**Core (Rust)**

- Cipher:
  - Seal/open round trip.
  - A flipped byte or a wrong key fails.
  - A value moved to a different name fails (AAD).
  - Two seals of the same value differ (fresh nonce).
- Value interpretation: every row of the §2.2 table, including
  `VaultMissing` naming the variable.
- Vault:
  - Enable leaves status `Unlocked`; lock gives `Locked`.
  - Unlock with the right password works; a wrong password is
    `WrongPassword`.
  - Vault JSON round-trips; an unknown version is `UnsupportedVaultVersion`.
- Change password: the old password fails, the new one works, and
  `secrets.json` bytes are unchanged.
- Enable/disable: values are sealed after enable and plain after disable, with
  the same contents; `disable` with a wrong password changes nothing.
- Crash safety: inject a failure after each file write in enable, disable and
  reset, and check every secret is still readable (sealed or plain) and no
  orphaned `enc:v1:` value exists without a vault.
- Plain values while on: they resolve, and unlock seals them and reports the
  count.
- Reset: sealed values gone, plain ones kept, vault gone, status `Off`.
- Locked:
  - `resolve_for_request` is `SecretsLocked` when a sealed value exists, and
    works when the file holds only plain values.
  - `names` works while locked.
  - No vault means today's results exactly (`layers_override_in_order`
    untouched).
- Keyring: canonical path keys (`/p` vs `/p/.`); an `Arc` held across `lock`
  still decrypts.
- Older-build round trip: parse a sealed file as a plain `Env`, save it back
  unchanged, and check it still opens.

**Critters (when both exist)**

- `enable` seals critter secrets created before it; a critter created after
  enabling gets sealed values.
- Rename a critter while encrypted: its secrets still open.
- `missing_secrets` is correct while locked.
- Scheduler: a tick while locked sends nothing, appends nothing, alerts
  nothing, and reports `secretsLocked`; a flow with no sealed values still
  runs.

**Desktop**

- Auto-lock: a pure function `should_lock(last_activity, now, minutes)`.

**CLI**

- Argument parsing for `--password-stdin` and the `secrets` subcommands.
- An encrypted project with no password source fails without sending, and
  never rewrites `secrets.json`.

**UI (vitest)**

- `domain/password.ts`.
- `ensureUnlocked` resolves `true` immediately when unlocked or off, and waits
  for the dialog when locked.

## 12. Order of work

Each phase starts by re-checking its §0 rows and ends green on the checks in
`ARCHITECTURE.md`.

1. **Core: crypto and storage.** Crates, `cipher`, `vault` format, `keyring`,
   `store` (`read`/`names`/`write`) with `SecretFile::Project`,
   `EnvFile::Secrets` routed through it, errors, tests.
2. **Core: lifecycle and resolving.** `enable`/`disable`/`change_password`/
   `reset` and sealing plain values on unlock, with crash-safety tests; locked
   behaviour in `resolve_for_request`; names-only `variable_names`.
3. **Backend and CLI.** Commands, `SecretsStatusChanged`, lock on project
   switch, auto-lock task, bindings regenerated; CLI password sources and the
   `secrets` subcommand.
4. **UI.** Store, actions, `ensureUnlocked` wired into send and run, dialogs,
   `LockButton`, Secrets layer states (including the locked names view),
   Security tab, tooltips.
5. **Critters.** If critters have landed, do §9.1 and §9.2 now. If not, fold
   §9.1 into the critters plan's text so its phases build it in.
6. **Wrap-up.** `ARCHITECTURE.md` (on-disk table, `secrets` module, locked
   behaviour), and a release note on what is and isn't encrypted.

## 13. As built

What changed from the plan above, and why.

**No vault file.** Encrypted values stay inline in the same
`secrets.json` files, and each one is self-describing:

```
enc:v1:<base64url(salt)>:<base64url(nonce ‖ ciphertext ‖ tag)>
```

- The key is Argon2id(password, salt) with the parameters fixed by `v1`
  (64 MiB, 3 passes, 1 lane). A project uses one salt for all its values, so
  unlocking derives one key. The associated data is still
  `cactus/secrets/v1/<name>`.
- "On" is `.cactus/state.json` → `secrets: { autoLockMinutes }` being set,
  **or** any sealed value existing. A lost or reset state file never exposes
  new values as plain text.
- A wrong password is detected by trying to open a sealed value. With
  nothing sealed yet there is nothing to check against, so unlocking sets the
  password; the UI asks for it twice then (`SecretsInfo.hasSealedValues`).
- Changing the password re-seals every value under a fresh salt (the files
  are small and local). If that's interrupted, values sit under two salts:
  the keyring holds keys per salt, the project stays `Locked` until both
  passwords have been given, and the next full unlock brings every value
  under the password given last. Every intermediate state stays readable.
- `enc:` with an unknown version is `UnsupportedSecretFormat`; a malformed
  value, or one that doesn't open with a key that opens its salt, is
  `SecretCorrupt`. `VaultMissing` and `UnsupportedVaultVersion` don't exist.
  Raising KDF cost later means a `v2` format, sealed on the next unlock.

**Core API** (`core/src/secrets/`): `cipher`, `keyring`, `store`
(`read`/`names`/`write`/`locked_for`, the only code that opens secrets
files), `lifecycle` (`info`, `status`, `enable`, `disable`,
`change_password`, `reset`, `set_auto_lock`, `unlock`, `unlock_read_only`,
`has_sealed_values`). `runner::ensure_unlocked_for` refuses folder runs and
flows before anything is sent; the matrix refuses as a whole.

**Desktop:** commands as §4.1 (`secrets_status` returns `SecretsInfo`;
`change_secrets_password` takes `current`, `next`). Auto-lock is a task
started with the app that checks the open project once a minute
(`auto_lock::should_lock`). Scheduled ticks skip while
`scheduler::waiting_for_secrets`, and `schedule_states` reports
`waiting: "secretsLocked"`. `run_scheduled_now` refuses while locked.

**CLI:** `--password-stdin` on `run`, `send` and `flow`; then
`CACTUS_SECRETS_PASSWORD`; then a TTY prompt. `secrets <project>
status|enable|disable|change-password`. `change-password` reads current and
new from two stdin lines, and refuses the env var (one variable can't hold
two passwords).
