//! Writing fresh YAML in the style Bruno writes: two-space indentation, list
//! items indented under their key, multi-line text as `|` blocks. serde's
//! emitter puts list items level with their key, so it only renders scalars.

use serde_json::{Map, Value};

const INDENT: usize = 2;

pub fn to_yaml(value: &Value) -> Result<String, serde_yaml_ng::Error> {
    let mut out = String::new();
    match value {
        Value::Object(map) if !map.is_empty() => write_mapping(&mut out, map, 0)?,
        Value::Array(items) if !items.is_empty() => write_sequence(&mut out, items, 0)?,
        other => {
            out.push_str(&render_scalar(other, 0)?);
            out.push('\n');
        }
    }
    Ok(out)
}

fn write_mapping(out: &mut String, map: &Map<String, Value>, indent: usize) -> Result<(), serde_yaml_ng::Error> {
    for (key, value) in map {
        pad(out, indent);
        write_entry(out, key, value, indent)?;
    }
    Ok(())
}

/// `key: value` from the current position, whose line is indented `indent`.
fn write_entry(out: &mut String, key: &str, value: &Value, indent: usize) -> Result<(), serde_yaml_ng::Error> {
    out.push_str(&render_scalar(&Value::String(key.to_string()), indent)?);
    out.push(':');
    match value {
        Value::Object(map) if !map.is_empty() => {
            out.push('\n');
            write_mapping(out, map, indent + INDENT)
        }
        Value::Array(items) if !items.is_empty() => {
            out.push('\n');
            write_sequence(out, items, indent + INDENT)
        }
        other => {
            out.push(' ');
            out.push_str(&render_scalar(other, indent + INDENT)?);
            out.push('\n');
            Ok(())
        }
    }
}

fn write_sequence(out: &mut String, items: &[Value], indent: usize) -> Result<(), serde_yaml_ng::Error> {
    for item in items {
        pad(out, indent);
        out.push('-');
        match item {
            Value::Object(map) if !map.is_empty() => {
                // The first key shares the dash's line.
                out.push(' ');
                for (n, (key, value)) in map.iter().enumerate() {
                    if n > 0 {
                        pad(out, indent + INDENT);
                    }
                    write_entry(out, key, value, indent + INDENT)?;
                }
            }
            Value::Array(nested) if !nested.is_empty() => {
                out.push('\n');
                write_sequence(out, nested, indent + INDENT)?;
            }
            other => {
                out.push(' ');
                out.push_str(&render_scalar(other, indent + INDENT)?);
                out.push('\n');
            }
        }
    }
    Ok(())
}

/// A scalar (or empty `{}` / `[]`) as serde quotes it. The lines of a block
/// scalar after the first are indented to `content_indent`.
fn render_scalar(value: &Value, content_indent: usize) -> Result<String, serde_yaml_ng::Error> {
    let rendered = serde_yaml_ng::to_string(value)?;
    let mut lines = rendered.trim_end_matches('\n').split('\n');
    let mut text = lines.next().unwrap_or_default().to_string();
    for line in lines {
        text.push('\n');
        if !line.is_empty() {
            // serde indents block lines by two from column 0; re-base them.
            pad(&mut text, content_indent);
            text.push_str(line.strip_prefix("  ").unwrap_or(line));
        }
    }
    Ok(text)
}

fn pad(out: &mut String, indent: usize) {
    out.extend(std::iter::repeat_n(' ', indent));
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn writes_bruno_style() {
        let value = json!({
            "info": {"name": "Get user", "seq": 1},
            "http": {
                "url": "{{baseUrl}}/users",
                "headers": [{"name": "Accept", "value": "1"}, {"name": "B", "value": ""}],
                "body": {"type": "json", "data": "{\n  \"a\": 1\n}"},
                "auth": "inherit",
            },
            "list": [["x"], "y"],
            "empty": [],
        });
        let yaml = to_yaml(&value).unwrap();
        assert_eq!(
            yaml,
            "info:\n  name: Get user\n  seq: 1\nhttp:\n  url: '{{baseUrl}}/users'\n  headers:\n    - name: Accept\n      value: '1'\n    - name: B\n      value: ''\n  body:\n    type: json\n    data: |-\n      {\n        \"a\": 1\n      }\n  auth: inherit\nlist:\n  -\n    - x\n  - y\nempty: []\n"
        );
        assert_eq!(serde_yaml_ng::from_str::<Value>(&yaml).unwrap(), value);
    }
}
