# Add an importer

Importers turn another tool's collection into cactus request files. They live
in `crates/core/src/import/`: `mod.rs` detects the format and writes the result, and
each format is one file (`postman.rs`, `openapi.rs`) that only _parses_.

Both the desktop app (sidebar import button → `import_file` command) and the
CLI (`cactus-cli import <project> <file>`) call `cactus_core::import::import`,
so a new format works in both once core knows it.

This guide adds an Insomnia v4 export as the example.

## How an import runs

`import(project, file)` in `crates/core/src/import/mod.rs`:

1. `parse_document`: reads the file as JSON, or YAML by extension (or when
   JSON fails).
2. `detect_format`: decides which format it is, from the document's shape.
3. The format's `parse(&Value) -> ParsedCollection`: a list of
   `ImportedScenario { dir, scenario }` plus collection variables.
4. `write_scenarios`: creates every request file. **All or nothing**: if one
   fails, those already written are removed.
5. `add_missing_env_vars`: adds the variables to the collection, never
   overwriting an existing one.

Your parser only does step 3. It doesn't touch the disk.

## 1. Add the format

In `crates/core/src/import/mod.rs`:

```rust
mod insomnia;

pub enum ImportFormat {
    Postman,
    OpenApi,
    Insomnia,
}

impl ImportFormat {
    pub fn as_str(self) -> &'static str {
        match self {
            // …
            Self::Insomnia => "insomnia",
        }
    }
}
```

Then teach `detect_format` to recognise it. Check for something specific to
the format, and keep the existing checks first so a document that's ambiguous
keeps importing as it does today:

```rust
let is_insomnia = document["_type"] == "export" && document["__export_format"] == 4;
```

and dispatch in `import`:

```rust
ImportFormat::Insomnia => insomnia::parse(&document),
```

## 2. Write the parser

Create `crates/core/src/import/insomnia.rs`:

```rust
//! Insomnia v4 exports.

use serde_json::Value;

use super::{ImportedScenario, ParsedCollection, child_dir, text_value};
use crate::scenario::Scenario;

pub(super) fn parse(document: &Value) -> ParsedCollection {
    let mut parsed = ParsedCollection::default();
    for resource in document["resources"].as_array().into_iter().flatten() {
        if resource["_type"] == "request" {
            let mut scenario = Scenario::titled(text_value(&resource["name"]));
            scenario.http.method = text_value(&resource["method"]).into();
            scenario.http.url = text_value(&resource["url"]).into();
            parsed.scenarios.push(ImportedScenario { dir: String::new(), scenario });
        }
    }
    parsed
}
```

Guidelines, from the two existing parsers:

- **Start from `Scenario::titled(name)`**, which sets the title and makes
  auth inherit from the folder, like a request created in the app.
- **Be lenient.** Read with `Value` indexing (`doc["a"]["b"]`), which yields
  `Null` instead of panicking, and skip what you don't understand rather than
  failing the whole import.
- **Folders**: build `dir` with `child_dir(parent, name)`, which slugifies the
  name. `write_scenarios` creates the folders.
- **Variables**: convert the format's variable syntax to `{{name}}`, and put
  collection-level values in `parsed.variables`. They're only added if the
  project doesn't already have them.
- **Scripts** the source format has go into `scenario.runtime.scripts`; cactus
  keeps them in the file and warns that it can't run them.
- Map to existing model types (`Body`, `TypedBody`, `Auth`, `Param`,
  `MultipartField`) rather than adding new ones; see `postman.rs` for bodies,
  auth and form data.

## 3. Test

Add tests in `mod.rs`'s `tests` module, modelled on the existing ones: write a
small sample document to a temp dir, run `import`, and check the
`ImportSummary` and the files created. Cover detection (it's this format, and
Postman / OpenAPI files still detect as before).

To check imported files against the OpenCollection schema, import a sample
into a temp project in `crates/core/tests/opencollection.rs` and run its
`assert_valid` over the result, as `everything_cactus_writes_is_valid` does
for files the app writes.

## 4. Update the UI's labels

The file picker's filter and the sidebar button name the formats:

- `IMPORT_FILTERS` in `apps/desktop/ui/src/actions/project.ts` (add extensions if the new
  format uses others).
- The import button's label in `apps/desktop/ui/src/features/sidebar/Sidebar.tsx`.
- The guide text in `apps/desktop/ui/src/features/settings/guide/pages.tsx`.

`ImportFormat` is in the generated bindings, so regenerate them
(`cargo test -p cactus`) after step 1.

## Checklist

- [ ] `ImportFormat` variant and `as_str`
- [ ] detection in `detect_format`, dispatch in `import`
- [ ] `crates/core/src/import/<format>.rs` with `pub(super) fn parse`
- [ ] tests for detection and parsing
- [ ] `cargo test -p cactus` to regenerate bindings
- [ ] UI labels and file filter
