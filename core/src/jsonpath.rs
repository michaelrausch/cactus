//! Tiny JSONPath subset: `$`, `.key`, `[0]`, `["key"]`.

use serde_json::Value;

const ROOT: char = '$';

/// The value at `path`, or `None` if any step is missing or malformed.
pub fn get<'a>(root: &'a Value, path: &str) -> Option<&'a Value> {
    let path = path.trim();
    let mut rest = path.strip_prefix(ROOT).unwrap_or(path);
    let mut current = root;
    while !rest.is_empty() {
        let (next, remaining) = step(current, rest)?;
        current = next;
        rest = remaining;
    }
    Some(current)
}

/// [`get`], plus a final `.length`: how many items an array has, keys an
/// object has or characters a string has. A real `length` key wins.
pub fn select(root: &Value, path: &str) -> Option<Value> {
    if let Some(value) = get(root, path) {
        return Some(value.clone());
    }
    let count = match get(root, path.trim().strip_suffix(".length")?)? {
        Value::Array(items) => items.len(),
        Value::Object(map) => map.len(),
        Value::String(text) => text.chars().count(),
        _ => return None,
    };
    Some(Value::from(count))
}

/// Apply the first step of `path` to `value`, returning the remainder.
fn step<'a, 'p>(value: &'a Value, path: &'p str) -> Option<(&'a Value, &'p str)> {
    if let Some(tail) = path.strip_prefix('.') {
        let end = tail.find(['.', '[']).unwrap_or(tail.len());
        Some((value.get(&tail[..end])?, &tail[end..]))
    } else if let Some(tail) = path.strip_prefix('[') {
        let end = tail.find(']')?;
        Some((bracket_lookup(value, &tail[..end])?, &tail[end + 1..]))
    } else {
        None
    }
}

/// `"key"` looks up an object key; anything else must be an array index.
fn bracket_lookup<'a>(value: &'a Value, selector: &str) -> Option<&'a Value> {
    match selector.strip_prefix('"').and_then(|s| s.strip_suffix('"')) {
        Some(key) => value.get(key),
        None => value.get(selector.trim().parse::<usize>().ok()?),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn paths() {
        let v = json!({"a": {"b": [10, {"c": "x"}]}, "d.e": 1});
        assert_eq!(get(&v, "$"), Some(&v));
        assert_eq!(get(&v, "$.a.b[0]"), Some(&json!(10)));
        assert_eq!(get(&v, "$.a.b[1].c"), Some(&json!("x")));
        assert_eq!(get(&v, r#"$["d.e"]"#), Some(&json!(1)));
        assert_eq!(get(&v, "$.a.zzz"), None);
        assert_eq!(get(&v, "$.a.b[9]"), None);
        assert_eq!(get(&v, "$é"), None);
    }

    #[test]
    fn length() {
        let v = json!({"items": [1, 2, 3], "name": "héllo", "obj": {"length": 9}, "n": 5});
        assert_eq!(select(&v, "$.items.length"), Some(json!(3)));
        assert_eq!(select(&v, "$.name.length"), Some(json!(5)));
        assert_eq!(select(&v, "$.obj.length"), Some(json!(9)), "a real key wins");
        assert_eq!(select(&v, "$.n.length"), None);
        assert_eq!(select(&v, "$.missing.length"), None);
        assert_eq!(select(&json!([1, 2]), "$.length"), Some(json!(2)));
    }
}
