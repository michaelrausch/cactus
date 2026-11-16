# Add a CLI command

`cactus-cli` (`cli/`) is the headless runner used in CI. Like the desktop
backend, it holds no behaviour of its own: each subcommand parses arguments,
calls `cactus-core`, and prints the result.

```
cli/src/main.rs          parse, execute, map the result to an exit code
cli/src/args.rs          the clap definitions; doc comments are the --help text
cli/src/commands/mod.rs  `execute`: builds the HTTP client, then dispatches
cli/src/commands/<x>.rs  one module per subcommand
cli/src/outcome.rs       Passed / Failed → exit codes 0 / 1 (errors are 2)
cli/src/output.rs        shared report formatting
cli/src/tool_host.rs     how flow tools ask questions on a terminal
```

This guide adds `cactus-cli list <project>`, which prints every request path
in tree order.

## 1. Declare the arguments

Add a variant to `Command` in `cli/src/args.rs`. The doc comments are the
`--help` text, so write them for users:

```rust
/// List every request in the project, in the order `run` sends them.
List {
    project: PathBuf,
    /// Print as JSON instead of one path per line.
    #[arg(long)]
    json: bool,
},
```

Match the flags the other commands use where they mean the same thing:
`-e/--env NAME`, `--as CRITTER`, `--json`. Add a parsing test to the `tests` module at the bottom
of `args.rs`.

## 2. Write the command

Create `cli/src/commands/list.rs`:

```rust
//! `cactus-cli list`: request paths, one per line.

use std::path::Path;

use cactus_core::scenario;

use crate::outcome::Outcome;

pub fn list(project: &Path, json: bool) -> cactus_core::Result<Outcome> {
    let entries = scenario::list(project)?;
    if json {
        let paths: Vec<_> = entries.iter().map(|entry| entry.rel_path.as_str()).collect();
        println!("{}", serde_json::to_string_pretty(&paths)?);
    } else {
        for entry in &entries {
            println!("{}", entry.rel_path);
        }
    }
    Ok(Outcome::Passed)
}
```

Conventions:

- **stdout is for the result, stderr for everything else** (status lines,
  warnings), so output can be piped into `jq`. See `send.rs`.
- **Return `Outcome`.** `Passed` exits 0, `Failed` exits 1. Return `Err` for
  anything that stopped the command from reaching a verdict; `main` prints it
  and exits 2.
- **Make it `async`** if it sends requests; the runtime is a single-threaded
  tokio runtime, and `execute` builds one shared HTTP client for you.
- For flows, pass a `CliHost` so `prompt` steps can ask on the terminal or
  take `--var` answers.

## 3. Dispatch to it

In `cli/src/commands/mod.rs`, add `mod list;` and a match arm in `execute`:

```rust
Command::List { project, json } => list::list(&project, json),
```

## 4. Try it

```sh
cargo run -p cactus-cli -- list examples/demo
cargo run -p cactus-cli -- list --help
```

## Checklist

- [ ] variant in `Command` (`args.rs`) with doc comments, plus a parse test
- [ ] `cli/src/commands/<name>.rs` returning `Result<Outcome>`
- [ ] `mod` + match arm in `commands/mod.rs`
- [ ] behaviour you needed that wasn't in core is added to core, not here
