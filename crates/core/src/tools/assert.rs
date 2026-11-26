//! `- tool: assert`: fail the flow unless values check out. The same
//! operators as a request's checks.
//!
//! ```yaml
//! - tool: assert
//!   checks:
//!     - {expression: "{{count}}", operator: gt, value: "0"}
//!     - {expression: "{{items}}.length", operator: gte, value: "1"}
//!   each:                      # every item of a list
//!     list: "{{items}}"
//!     checks:
//!       - {expression: id, operator: isNotEmpty}
//! ```

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{Tool, ToolCx, ToolOutput};
use crate::collection::LenientString;
use crate::env::{Env, Substituter};
use crate::http::{check_value, parse_value};
use crate::scenario::Assertion;
use crate::{Error, Result, jsonpath};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Assert {
    /// `expression` is a value: `{{name}}`, optionally with a path into
    /// it (`{{items}}.length`, `{{user}}.email`), or any text with `{{names}}`.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub checks: Vec<Assertion>,
    #[serde(skip_serializing_if = "AssertEach::is_empty")]
    pub each: AssertEach,
}

/// Checks every item of a list must pass.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct AssertEach {
    /// The list, as a check's `expression`: `{{items}}`, `{{data}}.items`.
    #[serde(skip_serializing_if = "LenientString::is_empty")]
    pub list: LenientString,
    /// `expression` is a path in the item (`id`, `tags.length`); empty is the item.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub checks: Vec<Assertion>,
}

impl AssertEach {
    fn is_empty(&self) -> bool {
        self.list.is_empty() && self.checks.is_empty()
    }
}

/// The value `text` names: `{{name}}` plus an optional path into it, read
/// as JSON; or else `text` with its `{{names}}` filled in. `None` when the
/// variable or path isn't there.
fn resolve(text: &str, vars: &Env) -> Option<Value> {
    let text = text.trim();
    if let Some((name, path)) = text.strip_prefix("{{").and_then(|rest| rest.split_once("}}"))
        && (path.is_empty() || path.starts_with(['.', '[']))
    {
        return jsonpath::select(&parse_value(vars.get(name.trim())?), &format!("${path}"));
    }
    Some(parse_value(&Substituter::new(vars).apply(text)))
}

/// The value at `path` in an item; the item itself for an empty path.
fn at(item: &Value, path: &str) -> Option<Value> {
    let path = path.trim();
    let path = if path.is_empty() || path.starts_with(['$', '.', '[']) { path.to_string() } else { format!(".{path}") };
    jsonpath::select(item, &format!("${}", path.trim_start_matches('$')))
}

fn active(checks: &[Assertion]) -> impl Iterator<Item = &Assertion> {
    checks.iter().filter(|check| !check.disabled)
}

impl Tool for Assert {
    fn title(&self) -> String {
        "Assert".into()
    }

    fn templates(&mut self) -> Vec<&mut String> {
        // Expressions and the list are read in `run`, so `{{items}}.length` can see the list.
        let values = self.checks.iter_mut().chain(&mut self.each.checks);
        values.filter_map(|check| check.value.as_mut().map(|value| &mut value.0)).collect()
    }

    async fn run(&self, cx: &mut ToolCx<'_>) -> Result<ToolOutput> {
        let mut passed = Vec::new();
        for assertion in active(&self.checks) {
            let check = check_value(assertion, resolve(&assertion.expression, cx.vars()).as_ref());
            if !check.ok {
                return Err(Error::Tool(format!("{}: {}", check.name, check.detail)));
            }
            passed.push(format!("{} ✓", check.name));
        }
        let each = &self.each;
        if !each.list.trim().is_empty() {
            let list = each.list.trim();
            let Some(Value::Array(items)) = resolve(list, cx.vars()) else {
                return Err(Error::Tool(format!("{list} isn't a list")));
            };
            for (index, item) in items.iter().enumerate() {
                for assertion in active(&each.checks) {
                    let check = check_value(assertion, at(item, &assertion.expression).as_ref());
                    if !check.ok {
                        let name = if check.name.is_empty() { "item" } else { &check.name };
                        return Err(Error::Tool(format!("{list} item {}: {name}: {}", index + 1, check.detail)));
                    }
                }
            }
            passed.push(format!("{list}: all {} items ✓", items.len()));
        }
        Ok(ToolOutput::text(passed.join("\n")))
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn values_are_variables_with_paths_or_templates() {
        let vars = Env::from([
            ("items".into(), r#"[{"id": 1}, {"id": 2}]"#.into()),
            ("n".into(), "5".into()),
            ("word".into(), "hi".into()),
        ]);
        assert_eq!(resolve("{{items}}.length", &vars), Some(json!(2)));
        assert_eq!(resolve("{{items}}[1].id", &vars), Some(json!(2)));
        assert_eq!(resolve(" {{n}} ", &vars), Some(json!(5)));
        assert_eq!(resolve("{{word}}.length", &vars), Some(json!(2)));
        assert_eq!(resolve("{{word}} there", &vars), Some(json!("hi there")));
        assert_eq!(resolve("{{missing}}", &vars), None);
        assert_eq!(at(&json!({"tags": ["a"]}), "tags.length"), Some(json!(1)));
        assert_eq!(at(&json!("x"), ""), Some(json!("x")));
    }
}
