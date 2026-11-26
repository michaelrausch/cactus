# Add a request field and editor tab

Request files follow [OpenCollection v1.0.0](https://schema.opencollection.com/).
cactus models only part of the spec; every key it doesn't model is still
**kept** on save, because saves patch the file in place rather than rewrite it
(see "Saving never rewrites a file" in [ARCHITECTURE.md](../../../ARCHITECTURE.md)).
So adding a field is about letting cactus _read, edit and act on_ it, not about
preserving it.

The worked examples are two fields cactus already has: `docs` (a top-level
string, with its own editor tab) and `settings.timeout` (a typed value that
changes how the request is sent).

## 1. Find the field in the spec

The schema is vendored at `crates/core/tests/fixtures/opencollection-v1.0.0.json`.
Find the field's exact name and shape there, for example under
`HttpRequestSettings`:

```json
"timeout": { "oneOf": [{ "type": "number" }, { "type": "string", "const": "inherit" }] }
```

Use the spec's name in the file even if the Rust field is named differently
(`#[serde(rename = "…")]`). The test `everything_cactus_writes_is_valid`
validates what cactus writes against this schema, so a misspelt key fails it.

## 2. Add it to the model

Request files are `Scenario` in `crates/core/src/scenario/model.rs`. Collection,
folder and environment files are in `crates/core/src/collection/model.rs`.

```rust
pub struct Scenario {
    // …
    #[serde(default, skip_serializing_if = "LenientString::is_empty")]
    pub docs: LenientString,
}
```

Every new field needs:

- **`#[serde(default)]`**, so files without it still load.
- **`skip_serializing_if`**, so it isn't written when empty. If its parent
  struct is skipped when empty (as `Settings` is), update the parent's
  `is_empty()` too:

  ```rust
  impl Settings {
      pub fn is_empty(&self) -> bool {
          self.timeout.is_none() // && self.your_field.is_none()
      }
  }
  ```

- **`LenientString` for free text**, so hand-written YAML like `value: 8080`
  reads as a string instead of failing.
- **The specta derive** on any new type:
  `#[cfg_attr(feature = "specta", derive(specta::Type))]`. A `u64` that the UI
  sees needs `specta(type = specta_typescript::Number)`, as `Timeout::Millis`
  has.
- **Untagged enums for "value or `inherit`"** shapes. `Timeout` is the pattern:

  ```rust
  #[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
  #[serde(untagged)]
  #[cfg_attr(feature = "specta", derive(specta::Type))]
  pub enum Timeout {
      Millis(#[cfg_attr(feature = "specta", specta(type = specta_typescript::Number))] u64),
      Inherit(Inherit),
  }
  ```

No change to `collection/patch.rs` is needed: it diffs whole models generically,
so a new field is saved in place like every other.

## 3. Use it when sending (if it affects the request)

Requests are built in `crates/core/src/http/request.rs` (`build_request`). That's where
`settings.timeout` takes effect:

```rust
if let Some(Timeout::Millis(ms)) = scenario.settings.timeout {
    builder = builder.timeout(Duration::from_millis(ms));
}
```

If the field should inherit from folders and the collection (like headers and
auth do), add it to the defaults chain in `crates/core/src/collection/` and read it
from `inherited` rather than only from the scenario.

If the field is something cactus keeps but can't act on (as with
`runtime.scripts`), add a message to `warnings` so the user knows.

## 4. Regenerate the bindings

```sh
cargo test -p cactus
```

`Scenario` in `apps/desktop/ui/src/api/bindings.ts` now includes the field.

## 5. Add an editor tab

Tabs live in `apps/desktop/ui/src/features/editor/tabs/` and all take `ScenarioTabProps`:
the whole scenario, and an `onChange` that receives the next version.
`DocsTab.tsx` is the whole of the `docs` editor:

```tsx
import { Textarea } from "@/components/ui/textarea";
import type { ScenarioTabProps } from "../types";

export function DocsTab({ scenario, onChange }: ScenarioTabProps) {
  return (
    <Textarea
      value={scenario.docs ?? ""}
      onChange={(e) => {
        const { docs: _previous, ...rest } = scenario;
        onChange(e.target.value ? { ...rest, docs: e.target.value } : rest);
      }}
      placeholder="What does this request do?"
      className="min-h-0 flex-1 resize-none rounded-lg"
    />
  );
}
```

Note how clearing the value **removes the key** rather than setting it to `""`.
This matters: the UI must hold a request exactly as the backend writes it
(empty fields left out). Otherwise the saved file reads back differently from
the buffer, every save looks like an outside change, and the editor reloads.

Then add a trigger and content pair in `apps/desktop/ui/src/features/editor/RequestEditor.tsx`:

```tsx
<TabsTrigger value="docs">Docs</TabsTrigger>
// …
<TabsContent value="docs" className="flex min-h-0 flex-col">
  <DocsTab {...props} />
</TabsContent>
```

For a small setting, consider adding a row to an existing tab rather than a
new tab. Shared inputs are in `apps/desktop/ui/src/components/` (`Field`, `KeyValueTable`)
and `components/ui/` (shadcn); fields that accept `{{variables}}` should use
`components/template/TemplateField.tsx` so they get highlighting and
completion.

Any pure logic (parsing, normalising) goes in `apps/desktop/ui/src/domain/` with a
`*.test.ts` beside it.

## 6. Keep both sides in agreement

If the UI derives the field from something else, or normalises it the way core
does (as query params mirror the URL), put the cases in a shared JSON fixture
under `crates/core/tests/fixtures/` and run it from both test suites. See
`query-params.json`, used by the tests in `crates/core/src/scenario/model.rs` and
`apps/desktop/ui/src/domain/scenario.test.ts`.

## 7. Test

- **Round trip**: a file with the field loads, saves unchanged, and an edit
  changes only that key. The tests in `crates/core/src/collection/patch.rs` and
  `crates/core/tests/opencollection.rs` show how.
- **Schema**: `cargo test -p cactus-core --test opencollection`.
- **Behaviour**: for sending, add a runner test using the local test server in
  `crates/core/src/runner/tests.rs`.
- In the app: edit the field, save, check the file in git's diff view (only
  that key should change), and reopen it.

## Checklist

- [ ] field in `scenario/model.rs` or `collection/model.rs`, spec name, `default` + `skip_serializing_if`
- [ ] parent `is_empty()` updated if needed
- [ ] used in `http/request.rs` (or warned about) if it affects sending
- [ ] `cargo test -p cactus` to regenerate bindings
- [ ] editor tab or row that removes the key when cleared
- [ ] round-trip and schema tests pass
