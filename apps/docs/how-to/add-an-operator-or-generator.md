# Add an assertion operator or a generator

Both are small lists defined in core that the UI keeps a copy of, so it can
offer completions without a round trip. `crates/core/tests/ui_contract.rs` fails if
the two copies differ, so always change both together.

## Assertion operators

Assertions are `runtime.assertions` in a request (and `assert` / `expect` in
flow steps), written Bruno's way:

```yaml
runtime:
  assertions:
    - expression: res.body.items
      operator: length
      value: "3"
```

To add an operator, for example `isUuid`:

### 1. Core

In `crates/core/src/http/assertions.rs`:

1. Add it to `ASSERTION_OPERATORS` and bump the array length
   (`[&str; 28]` → `[&str; 29]`). The order is the order the editor suggests
   them in.
2. Add an arm to `holds(operator, actual, expected)`:

   ```rust
   "isUuid" => actual.is_some_and(|a| a.as_str().is_some_and(is_uuid)),
   ```

   `actual` is `None` when the expression found nothing; `expected` is the
   raw `value` text. Use the helpers already in the file (`parse_expected`,
   `as_number`, `plain_string`, `loosely_equal`, `length`, `pattern`) so the
   new operator coerces values the same way the others do. Return
   `Err("…")` only for a malformed `expected` (like an invalid regex).

3. Add cases to the `operators` test at the bottom of the file.

If you're adding an operator Bruno has, use Bruno's name, so files stay
portable.

### 2. UI

Add the same string, at the same position, to `ASSERTION_OPERATORS` in
`apps/desktop/ui/src/domain/http.ts`. Operators whose names start with `is` are treated as
taking no value (`UNARY` in `apps/desktop/ui/src/domain/suggestions.ts`), so name a
value-less operator `is…`, or update that rule.

### 3. Check

```sh
cargo test -p cactus-core --test ui_contract
cargo test -p cactus-core http::assertions
npm test --prefix apps/desktop/ui -- assertions suggestions
```

---

## Generators

Generators are values the `set` flow tool fills in when a value is exactly the
generator's name:

```yaml
- tool: set
  values:
    order_ref: $uuid
    created: $isoTime
```

To add one, for example `$randomHex`:

### 1. Core

In `crates/core/src/tools/random.rs`, add it to `GENERATORS` and to the `match` in
`generate`:

```rust
pub const GENERATORS: &[&str] = &["$uuid", "$timestamp", "$isoTime", "$randomInt", "$randomEmail", "$randomHex"];

"$randomHex" => hex(&random_bytes::<8>()),
```

Use `random_bytes` (the OS random source) rather than adding a dependency.
Add a test beside the others in the file.

### 2. UI

Add the same name, in the same position, to `GENERATORS` in
`apps/desktop/ui/src/domain/generators.ts`. The `set` tool's editor offers it as soon as a
value starts with `$`.

### 3. Check

```sh
cargo test -p cactus-core --test ui_contract
cargo test -p cactus-core tools::random
```
