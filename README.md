# cactus

A file-based HTTP client. A cactus project is an
[OpenCollection](https://schema.opencollection.com/) v1.0.0 collection: a folder
of YAML files, one per request, in the layout Bruno uses. Projects can be
committed, diffed and reviewed like code, and opened in Bruno.

On top of plain requests, cactus has:

- **Environments and variables** with layered precedence (collection →
  environment → folder → request → secrets → captured values).
- **Assertions and captures** on every response (`res.status eq 200`, `res.body.token` → `{{token}}`).
- **Critters**: personas with their own variables, secrets and captured
  context, and **flows** that chain requests and tool steps (prompt, poll,
  webhook, Slack…), optionally on a schedule with alerts.
- **Git** built into the desktop app: status, diffs, staging, commits, branches.
- **Import** from Postman v2 collections and OpenAPI 3 / Swagger 2 specs.
- **A headless CLI** for CI, with JSON and JUnit output.

## Repository layout

```
crates/core/        cactus-core: all behaviour. Knows nothing about Tauri or the terminal.
crates/cli/         cactus-cli: headless runner for CI (clap).
crates/license-gen/ Issues pro license keys. Never shipped.
apps/desktop/src-tauri/  cactus: desktop backend, thin Tauri commands over core.
apps/desktop/ui/         React + TypeScript front end for the desktop app.
apps/site/          Marketing site and the pro license worker (Astro).
apps/docs/          Documentation site (Starlight), plus how-to guides and design plans.
examples/           A demo project (examples/demo) to open in the app or run with the CLI.
```

## Getting started

### Prerequisites

- Rust (stable, edition 2024).
- Node 24 and npm.
- On Linux, Tauri's system libraries:
  `libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev libxdo-dev`.

The JavaScript apps are one npm workspace, so install once at the root:

```sh
npm ci
```

### Run the desktop app

From the repository root:

```sh
npm run dev:desktop
```

This starts Vite on `localhost:5173` and the Tauri shell around it. Debug
builds regenerate `apps/desktop/ui/src/api/bindings.ts` from the Rust types on start-up.
Open `examples/demo` to look around.

To build installers, run `npm run build:desktop`.

The marketing site and docs run with `npm run dev:site` and `npm run dev:docs`.

### Run the CLI

```sh
cargo run -p cactus-cli -- run examples/demo                     # every request, in tree order
cargo run -p cactus-cli -- send examples/demo get-anything.yml   # one request, body to stdout
cargo run -p cactus-cli -- flow examples/demo admin smoke        # one critter's flow
cargo run -p cactus-cli -- flow examples/demo --all --junit out.xml
cargo run -p cactus-cli -- import examples/demo postman.json
```

Common flags: `-e <env>`, `--as <critter>`, `--json`, `--var name=value` (answers
a flow's prompts). Exit codes: `0` passed, `1` something failed, `2` error.

### Checks

Everything CI runs:

```sh
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace            # also regenerates apps/desktop/ui/src/api/bindings.ts

npm run check                     # UI typecheck, lint, format, tests; site and docs builds
CACTUS_EDITION=pro npm test -w cactus-ui
```

## Documentation

- [ARCHITECTURE.md](ARCHITECTURE.md): how the pieces fit, what's on disk, and the
  rules each layer follows. Read this first.
- [apps/docs/](apps/docs/README.md): the docs index, including
  [how-to guides](apps/docs/README.md#how-to-guides) for extending cactus.
