# cactus docs

This folder holds two things: these contributor notes, and the user docs
site (Astro Starlight; pages in `src/content/docs/`). To work on the site, from
the repository root:

```sh
npm ci && npm run dev:docs         # http://localhost:4321
npm run build -w cactus-docs       # static output in apps/docs/dist
```

Start with [ARCHITECTURE.md](../../ARCHITECTURE.md): the crates, what lives on disk,
variable precedence, how saving works, and the UI's layering rules. The guides
below assume it.

## How-to guides

Each guide lists every file to touch, in order, and the check that tells you
you've missed one.

| I want to…                                              | Guide                                                                            |
| ------------------------------------------------------- | -------------------------------------------------------------------------------- |
| Add a flow step type (`- tool: …`)                      | [Add a flow tool](how-to/add-a-flow-tool.md)                                     |
| Expose new core behaviour to the desktop UI             | [Add a backend command or event](how-to/add-a-backend-command.md)                |
| Add a `cactus-cli` subcommand or flag                   | [Add a CLI command](how-to/add-a-cli-command.md)                                 |
| Support another field of the request file               | [Add a request field and editor tab](how-to/add-a-request-field.md)              |
| Add an assertion operator or a `$generator`             | [Add an assertion operator or generator](how-to/add-an-operator-or-generator.md) |
| Import another collection format                        | [Add an importer](how-to/add-an-importer.md)                                     |
| Add a screen, dialog, settings tab or keyboard shortcut | [Add a UI feature](how-to/add-a-ui-feature.md)                                   |
| Add a colour theme                                      | [Add a theme](how-to/add-a-theme.md)                                             |
| Add a critter species                                   | [Add a critter species](how-to/add-a-critter-species.md)                         |

## Conventions that apply everywhere

- **core owns behaviour.** `apps/desktop/src-tauri` and `crates/cli` only convert arguments and
  present results. If you're writing an `if` in a Tauri command, it probably
  belongs in core.
- **Errors are user-facing.** Every `cactus_core::Error` message is shown as-is
  in the app and the CLI. Add a variant to `crates/core/src/error.rs` with a short
  message rather than formatting strings at the call site. Attach paths to I/O
  errors with `.map_err(Error::file(&path))`.
- **Files stay minimal and round-trip.** Model fields use
  `#[serde(default, skip_serializing_if = …)]` so absent stays absent. Saves
  patch files in place; never write a request file with `serde_yaml` directly.
- **Never hand-edit `apps/desktop/ui/src/api/bindings.ts`.** Regenerate with
  `cargo test -p cactus`. CI fails if it is stale.
- **Things the UI mirrors are tested.** When the UI keeps a copy of a core list
  (assertion operators, generators, file names), `crates/core/tests/ui_contract.rs`
  checks it. Update both sides together.
- **UI copy is terse.** Labels and titles only: no helper sentences, caveats or
  taglines. Error strings are short, with no trailing advice.
- **Colours come from theme tokens**, never Tailwind's fixed palette
  (`text-red-500`); a test fails if a component uses one.
- **Lints are strict.** Clippy pedantic with `-D warnings`, `unsafe` forbidden;
  Oxlint with `--deny-warnings` and `import/no-cycle`.

## Design plans

Longer write-ups of features as they were designed. Useful for the "why", but
the code is the source of truth where they differ.

- [critters-plan.md](critters-plan.md): critters, flows, schedules and alerts.
- [mcp-plan.md](mcp-plan.md): a planned MCP server.
