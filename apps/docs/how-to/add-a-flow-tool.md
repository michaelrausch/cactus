# Add a flow tool

A flow tool is a flow step that isn't a request, written as `- tool: <id>` in
`critters/<critter>/flows/<flow>.yml`. The existing tools are in
`crates/core/src/tools/` (Rust: what it does) and
`apps/desktop/ui/src/features/critters/flows/tools/` (TypeScript: how the editor shows it).

This guide adds a `teams` tool that posts a message to a Microsoft Teams
incoming webhook:

```yaml
steps:
  - request: orders/create.yml
  - tool: teams
    webhook: "{{TEAMS_WEBHOOK}}"
    text: "Created order {{order_id}}"
```

`slack.rs` / `slack.tsx` are the smallest real example; `prompt`, `poll` and
`run` show asking the user, sending requests and running other flows.

## 1. Write the tool in core

Create `crates/core/src/tools/teams.rs`:

```rust
//! `- tool: teams`: post a message to a Microsoft Teams incoming webhook.

use serde::{Deserialize, Serialize};

use super::webhook::post_json;
use super::{Tool, ToolCx, ToolOutput};
use crate::Result;
use crate::collection::LenientString;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Teams {
    #[serde(skip_serializing_if = "LenientString::is_empty")]
    pub webhook: LenientString,
    #[serde(skip_serializing_if = "LenientString::is_empty")]
    pub text: LenientString,
}

impl Tool for Teams {
    fn title(&self) -> String {
        "Teams".into()
    }

    fn templates(&mut self) -> Vec<&mut String> {
        vec![&mut self.webhook.0, &mut self.text.0]
    }

    async fn run(&self, cx: &mut ToolCx<'_>) -> Result<ToolOutput> {
        post_json(cx, &self.webhook, &serde_json::json!({ "text": self.text.0 })).await?;
        Ok(ToolOutput::text(self.text.0.clone()))
    }
}
```

The rules every tool follows:

- **The struct name, lowercased, is the id.** `Teams` is `tool: teams`, and the
  module must have the same name (`teams.rs`), because the registry uses the
  module name as the id.
- **`#[serde(default)]` on the struct, and `skip_serializing_if` on every field**
  (`""`, `[]`, `{}`, `false`, `0`), so `- tool: teams` alone reads, and empty
  fields aren't written. A unit test (`each_tool_reads_as_its_id`) enforces this.
- **Use `LenientString` for text** people type in YAML, so `value: 8080` reads
  as a string instead of failing.
- **`templates()` lists every field that may hold `{{name}}`.** They're filled in
  before `run` is called, so `run` sees final values.
- **Fail with `Error::Tool("…")`** and a short message; it's shown on the step
  card and in the CLI. Reuse an existing variant (`MissingUrl`, `InvalidUrl`)
  where one fits.
- **Put limits on anything that can stall a schedule**, as `wait` does with
  `MAX_SECONDS`.

What `run` can use, all through `cx: &mut ToolCx`:

| Need                                     | Use                                                               |
| ---------------------------------------- | ----------------------------------------------------------------- |
| Make an HTTP call                        | `cx.client` (a shared `reqwest::Client`), or `webhook::post_json` |
| Read a variable directly                 | `cx.vars()`                                                       |
| Give later steps `{{name}}`              | `cx.set(name, value)`, or `cx.set_secret` to mask it in results   |
| Ask the person something and wait        | `cx.host.ask(Ask::Fields { … })`; handle `Answer::Unattended`     |
| Show a message / send an OS notification | `cx.host.show(…)` / `cx.host.notify(…)`                           |
| Send a request step, or run another flow | `cx.runner.send(&step)` / `cx.runner.run_flow(critter, flow)`     |
| Send many variants (fuzz, load)          | `cx.runner.probe(&step, &edit)`: no history, captures or re-login |
| Report how far a long run has got        | `cx.progress(done, total)`                                        |

Values set with `cx.set` last for the rest of the run only; they are never
written to disk. Remember a tool also runs **headless** (schedules, re-login
after a 401, CI): `ask` returns `Answer::Unattended` there, so fall back to a
default or fail clearly.

### Security tools

Return what was found in `ToolOutput::findings` (`Some(vec![Finding {
severity, title, rule, location, detail, evidence }])`, `Some(vec![])` when
nothing was found) and numbers in `ToolOutput::metrics`. Don't return `Err`
for findings: implement `Tool::verdict` so the step fails and still shows
them.

| Fails on      | Field                    | `verdict`                                               |
| ------------- | ------------------------ | ------------------------------------------------------- |
| Findings      | `fail_on: FailOn`        | `findings_verdict(self.fail_on, o.findings.as_deref())` |
| Metric limits | `checks: Vec<Assertion>` | `metrics_verdict(&self.checks, &o.metrics)`             |

`rule` must be stable between runs: findings are matched by `rule` and
`location`.

## 2. Register it

In `crates/core/src/tools/mod.rs`, add the module and list it in `tools!`:

```rust
mod teams;
// …
tools! {
    prompt::Prompt,
    // …
    slack::Slack,
    teams::Teams,
    // …
}
```

The macro generates the `ToolStep` enum (`#[serde(tag = "tool")]`), its
dispatch, and `ToolStep::IDS`. Nothing else in core, the CLI or the desktop
backend needs to change: they all run tools through `ToolStep::run`.

## 3. Regenerate the bindings

```sh
cargo test -p cactus
```

`apps/desktop/ui/src/api/bindings.ts` now has a `{ tool: "teams"; webhook?: string; text?: string }`
variant of `ToolStep`. Until the UI has an editor for it, `npm run typecheck --prefix apps/desktop/ui`
fails in `tools/index.ts`. That is deliberate.

## 4. Write the editor

Create `apps/desktop/ui/src/features/critters/flows/tools/teams.tsx`:

```tsx
import { MessageSquare } from "lucide-react";
import { bind, Fields, TextInput } from "./fields";
import { defineTool } from "./types";

export const teams = defineTool<"teams">({
  label: "Teams",
  icon: MessageSquare,
  create: () => ({ tool: "teams", webhook: "{{TEAMS_WEBHOOK}}" }),
  summary: (step) => step.text ?? "",
  dataflow: (step) => ({
    templates: [step.webhook ?? "", step.text ?? ""],
    problems: step.webhook?.trim() ? [] : ["No webhook URL"],
  }),
  Editor: (props) => (
    <Fields>
      <TextInput label="Webhook" mono {...bind(props, "webhook")} />
      <TextInput label="Message" multiline {...bind(props, "text")} />
    </Fields>
  ),
});
```

| Field      | What it's for                                                                                                                                           |
| ---------- | ------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `label`    | Name in the palette and on the step card.                                                                                                               |
| `icon`     | A `lucide-react` icon.                                                                                                                                  |
| `create`   | The step added when it's dropped into a flow. Leave out empty fields, as the backend would.                                                             |
| `summary`  | One line on the step card.                                                                                                                              |
| `dataflow` | What the step reads (`templates`), what it `sets` for later steps, and `problems` to list. Drives the chips between steps and the flow's problems list. |
| `Editor`   | The step's settings pane. Build it from `fields.tsx`.                                                                                                   |

`fields.tsx` has `TextInput` (templated, with `{{name}}` completion), `NumberInput`,
`Checkbox`, `ChoiceInput`, `ChecksInput` and `MapInput`. `bind(props, "field")`
wires one of them to a step field as an undoable edit, and drops the field when
it's emptied.

If the tool mirrors a limit from Rust (like `wait`'s `MAX_SECONDS`), copy the
constant with a comment naming the Rust one.

## 5. Add it to the registry

In `apps/desktop/ui/src/features/critters/flows/tools/index.ts`, import it and add it to
`TOOLS`. The order there is the order of the palette.

```ts
import { teams } from "./teams";

export const TOOLS = {
  // …
  slack,
  teams,
  notify,
};
```

## 6. Test

- **Rust:** `each_tool_reads_as_its_id` already covers reading and writing.
  Test `run` end to end in `crates/core/src/runner/tests.rs`, which runs real flows
  in a temp project: `tool("tool: teams\n…")` builds a step, `serve(|request| …)`
  stands up a local HTTP server, and `run_flow(…, &Headless, …)` runs it.
  `webhook_and_slack_post_and_fail_on_errors` is the model to copy. Pure
  logic with no I/O can be unit-tested in the tool's own file, as `assert.rs`
  and `pick.rs` do.
- **UI:** `tools.test.ts` checks every `create()` is written the way the backend
  writes it. Add a `toolDataflow({ tool: "teams", … })` case for its problems.
- Run the full [checks](../../../README.md#checks), then try it in the app: open a
  flow, drag the tool in from the palette, run it, and check the step card
  and the YAML tab.

## Checklist

- [ ] `crates/core/src/tools/<id>.rs`: struct + `impl Tool`
- [ ] `crates/core/src/tools/mod.rs`: `mod <id>;` and an entry in `tools!`
- [ ] `cargo test -p cactus` to regenerate bindings
- [ ] `apps/desktop/ui/src/features/critters/flows/tools/<id>.tsx` with `defineTool`
- [ ] `apps/desktop/ui/src/features/critters/flows/tools/index.ts`: entry in `TOOLS`
- [ ] Tests on both sides
