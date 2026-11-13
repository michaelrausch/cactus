//! Small string helpers used across core.

use serde_json::Value;

/// Cut `text` to at most `max_bytes` without splitting a character.
/// Returns whether anything was removed.
pub fn truncate_utf8(text: &mut String, max_bytes: usize) -> bool {
    if text.len() <= max_bytes {
        return false;
    }
    let mut cut = max_bytes;
    while !text.is_char_boundary(cut) {
        cut -= 1;
    }
    text.truncate(cut);
    true
}

/// A JSON value as the plain text a user would type: strings unquoted,
/// `null` empty, everything else as JSON.
pub fn plain_string(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

/// Lowercase ASCII alphanumerics joined by single dashes.
pub fn slugify(input: &str) -> String {
    let mut slug = String::with_capacity(input.len());
    for c in input.chars() {
        if c.is_ascii_alphanumeric() {
            slug.push(c.to_ascii_lowercase());
        } else if !slug.is_empty() && !slug.ends_with('-') {
            slug.push('-');
        }
    }
    slug.trim_end_matches('-').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncates_on_char_boundary() {
        let mut text = "aé".to_string();
        assert!(truncate_utf8(&mut text, 2));
        assert_eq!(text, "a");
        assert!(!truncate_utf8(&mut text, 10));
    }

    #[test]
    fn slugifies() {
        assert_eq!(slugify("Get User!"), "get-user");
        assert_eq!(slugify("  --x__y--  "), "x-y");
        assert_eq!(slugify("!!!"), "");
    }
}
