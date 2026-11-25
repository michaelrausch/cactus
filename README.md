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
core/       cactus-core: all behaviour. Knows nothing about Tauri or the terminal.
cli/        cactus-cli: headless runner for CI (clap).
src-tauri/  cactus: desktop backend, thin Tauri commands over core.
ui/         React + TypeScript front end for the desktop app.
examples/   A demo project (examples/demo) to open in the app or run with the CLI.
docs/       Architecture notes, how-to guides and design plans.
```

## Getting started

### Prerequisites

- Rust (stable, edition 2024).
- Node 24 and npm.
- On Linux, Tauri's system libraries:
  `libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev libxdo-dev`.

```sh
cd ui && npm ci
```

### Run the desktop app

From the repository root (the Tauri CLI finds `src-tauri/` and `ui/package.json`
by searching below the current folder):

```sh
node ui/node_modules/@tauri-apps/cli/tauri.js dev
```

This starts Vite on `localhost:5173` and the Tauri shell around it. Debug
builds regenerate `ui/src/api/bindings.ts` from the Rust types on start-up.
Open `examples/demo` to look around.

To build installers, run the same command with `build` instead of `dev`.

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
cargo test --workspace            # also regenerates ui/src/api/bindings.ts

cd ui
npm run typecheck && npm run lint && npm run format:check && npm test && npm run build
```

## Documentation

- [ARCHITECTURE.md](ARCHITECTURE.md): how the pieces fit, what's on disk, and the
  rules each layer follows. Read this first.
- [docs/](docs/README.md): the docs index, including
  [how-to guides](docs/README.md#how-to-guides) for extending cactus.
