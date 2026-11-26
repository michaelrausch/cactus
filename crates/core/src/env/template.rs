//! `{{name}}` placeholder substitution.

use super::layers::Env;

const OPEN: &str = "{{";
const CLOSE: &str = "}}";

/// Replaces placeholders with env values and remembers the names it could
/// not resolve, across every string it is applied to.
pub struct Substituter<'a> {
    env: &'a Env,
    unresolved: Vec<String>,
}

impl<'a> Substituter<'a> {
    pub fn new(env: &'a Env) -> Self {
        Self { env, unresolved: Vec::new() }
    }

    /// Single pass: substituted values are not themselves expanded. Unknown
    /// placeholders are left intact.
    pub fn apply(&mut self, input: &str) -> String {
        let mut output = String::with_capacity(input.len());
        let mut rest = input;
        while let Some(placeholder) = Placeholder::find(rest) {
            output.push_str(&rest[..placeholder.start]);
            if let Some(value) = self.env.get(placeholder.name) {
                output.push_str(value);
            } else {
                self.record_unresolved(placeholder.name);
                output.push_str(placeholder.raw);
            }
            rest = &rest[placeholder.end..];
        }
        output.push_str(rest);
        output
    }

    /// Names left unresolved so far, in first-seen order.
    pub fn unresolved(&self) -> &[String] {
        &self.unresolved
    }

    /// Unresolved names in first-seen order, without duplicates.
    pub fn into_unresolved(self) -> Vec<String> {
        self.unresolved
    }

    fn record_unresolved(&mut self, name: &str) {
        if !self.unresolved.iter().any(|seen| seen == name) {
            self.unresolved.push(name.to_string());
        }
    }
}

struct Placeholder<'a> {
    start: usize,
    end: usize,
    /// The trimmed variable name.
    name: &'a str,
    /// The placeholder as written, braces included.
    raw: &'a str,
}

impl<'a> Placeholder<'a> {
    fn find(text: &'a str) -> Option<Self> {
        let start = text.find(OPEN)?;
        let name_start = start + OPEN.len();
        let name_len = text[name_start..].find(CLOSE)?;
        let end = name_start + name_len + CLOSE.len();
        Some(Self { start, end, name: text[name_start..name_start + name_len].trim(), raw: &text[start..end] })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn substitutes_and_reports_unknown() {
        let env = Env::from([("base".into(), "http://h".into()), ("t".into(), "{{base}}".into())]);
        let mut vars = Substituter::new(&env);
        let out = vars.apply("{{base}}/x?a={{ t }}&b={{nope}}&c={{nope}}{{open");
        // Single pass: values are not re-expanded.
        assert_eq!(out, "http://h/x?a={{base}}&b={{nope}}&c={{nope}}{{open");
        assert_eq!(vars.into_unresolved(), ["nope"]);
    }

    #[test]
    fn accumulates_across_calls() {
        let env = Env::new();
        let mut vars = Substituter::new(&env);
        vars.apply("{{a}}");
        vars.apply("{{b}} {{a}}");
        assert_eq!(vars.into_unresolved(), ["a", "b"]);
    }
}
