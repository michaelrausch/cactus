//! Saving by editing a YAML file in place.
//!
//! The model a file was read into is compared with the model being saved,
//! and only the differences are applied to the file's lossless syntax tree.
//! Everything else keeps its bytes: comments, blank lines, quoting, key
//! order, and any keys the model doesn't know.
//!
//! - Mappings are recursed into key by key. A new key goes where the
//!   model's field order puts it.
//! - Lists are matched with a longest common subsequence, so an untouched
//!   item keeps its comments. An item that changed in place is recursed
//!   into rather than rewritten.
//! - A removed item or key takes the comment lines directly above it along.
//! - A changed string keeps its quote style. Strings with line breaks become
//!   `|` blocks, as Bruno writes them.

use std::str::FromStr;

use rowan::NodeOrToken;
use rowan::ast::AstNode;
use serde_json::{Map, Value};
use yaml_edit::{Document, Lang, Mapping, ScalarValue, Sequence, SyntaxKind, YamlFile, YamlNode};

use super::emit::to_yaml;

type SyntaxNode = rowan::SyntaxNode<Lang>;
type SyntaxElement = rowan::SyntaxElement<Lang>;

/// Run `$body` with `$yaml` bound to the [`Fresh`] value as an `AsYaml`.
/// Each variant is a different type and `AsYaml` can't be a trait object, so
/// there is one arm per variant: add a new one here when `Fresh` grows one.
macro_rules! with_yaml {
    ($fresh:expr, |$yaml:ident| $body:expr) => {
        match $fresh {
            Fresh::Auto(text) => {
                let $yaml = text.as_str();
                $body
            }
            Fresh::Styled(scalar) => {
                let $yaml = scalar;
                $body
            }
            Fresh::Int(n) => {
                let $yaml = n;
                $body
            }
            Fresh::Float(n) => {
                let $yaml = n;
                $body
            }
            Fresh::Bool(b) => {
                let $yaml = b;
                $body
            }
            Fresh::Mapping(mapping) => {
                let $yaml = mapping;
                $body
            }
            Fresh::Sequence(sequence) => {
                let $yaml = sequence;
                $body
            }
        }
    };
}

/// Why a file couldn't be patched in place.
#[derive(Debug, thiserror::Error)]
pub enum PatchError {
    #[error("only a mapping at the top of the file can be patched")]
    NotAMapping,
    /// The YAML library refused the file, or a value to write into it.
    #[error("{0}")]
    Yaml(String),
    #[error("unexpected YAML for {0}")]
    UnexpectedShape(Value),
}

fn yaml_error(error: impl std::fmt::Display) -> PatchError {
    PatchError::Yaml(error.to_string())
}

/// Apply the change from `old` to `new` (both JSON objects) to `text`,
/// which `old` was read from.
pub fn patch(text: &str, old: &Value, new: &Value) -> Result<String, PatchError> {
    let (Some(old), Some(new)) = (old.as_object(), new.as_object()) else {
        return Err(PatchError::NotAMapping);
    };
    let file = YamlFile::from_str(text).map_err(yaml_error)?;
    let Some(root) = file.documents().next().and_then(|document| document.as_mapping()) else {
        // Empty or comments only: keep what is there and add the content.
        let fresh = to_yaml(&Value::Object(new.clone())).map_err(yaml_error)?;
        let kept = text.trim_end();
        return Ok(if kept.is_empty() { fresh } else { format!("{kept}\n{fresh}") });
    };
    patch_mapping(&root, old, new)?;
    Ok(file.to_string())
}

fn patch_mapping(mapping: &Mapping, old: &Map<String, Value>, new: &Map<String, Value>) -> Result<(), PatchError> {
    for key in old.keys().filter(|key| !new.contains_key(*key)) {
        remove_key(mapping, key);
    }
    let field_order: Vec<&str> = new.keys().map(String::as_str).collect();
    for (key, new_value) in new {
        let old_value = old.get(key);
        if old_value == Some(new_value) {
            continue;
        }
        match (mapping.get(key.as_str()), new_value) {
            (Some(YamlNode::Mapping(child)), Value::Object(new_child)) => {
                // A key the model didn't read (all its fields unknown) still
                // holds content: patch it as if it were empty.
                let empty = Map::new();
                let old_child = old_value.and_then(Value::as_object).unwrap_or(&empty);
                patch_mapping(&child, old_child, new_child)?;
            }
            (Some(YamlNode::Sequence(child)), Value::Array(new_items)) if old_value.is_some_and(Value::is_array) => {
                let old_items = old_value.and_then(Value::as_array).map_or(&[][..], Vec::as_slice);
                patch_sequence(&child, old_items, new_items)?;
            }
            (existing, _) => {
                let fresh = Fresh::new(new_value, existing.as_ref())?;
                with_yaml!(fresh, |yaml| mapping.set_with_field_order(key.as_str(), yaml, field_order.iter().copied()));
            }
        }
    }
    Ok(())
}

fn patch_sequence(sequence: &Sequence, old: &[Value], new: &[Value]) -> Result<(), PatchError> {
    // Gaps between matched items are edited from the end backwards, so the
    // indices of everything before a gap stay valid.
    for gap in gaps(old, new).into_iter().rev() {
        let paired = gap.old.len().min(gap.new.len());
        for offset in 0..paired {
            let index = gap.old.start + offset;
            replace_item(sequence, index, &old[index], &new[gap.new.start + offset])?;
        }
        for index in (gap.old.start + paired..gap.old.end).rev() {
            remove_item(sequence, index);
        }
        for (offset, value) in new[gap.new.start + paired..gap.new.end].iter().enumerate() {
            insert_item(sequence, gap.old.start + paired + offset, value)?;
        }
    }
    Ok(())
}

fn replace_item(sequence: &Sequence, index: usize, old: &Value, new: &Value) -> Result<(), PatchError> {
    match (sequence.get(index), old.as_object(), new.as_object()) {
        (Some(YamlNode::Mapping(item)), Some(old), Some(new)) => patch_mapping(&item, old, new),
        (existing, ..) => {
            let fresh = Fresh::new(new, existing.as_ref())?;
            with_yaml!(fresh, |yaml| sequence.set(index, yaml));
            Ok(())
        }
    }
}

fn insert_item(sequence: &Sequence, index: usize, value: &Value) -> Result<(), PatchError> {
    let fresh = Fresh::new(value, None)?;
    with_yaml!(fresh, |yaml| sequence.insert(index, yaml));
    keep_comments_with_next_item(sequence, index);
    Ok(())
}

fn remove_item(sequence: &Sequence, index: usize) {
    if let Some(entry) = sequence_entries(sequence).nth(index) {
        detach_all(leading_comment_lines(&entry));
    }
    sequence.remove(index);
    if index == 0 {
        drop_blank_lines_after_key(sequence);
    }
}

/// The blank lines that set a removed first item apart from the next one
/// would otherwise hang directly under the key. They sit after the line
/// break ending the key's line, in front of the sequence.
fn drop_blank_lines_after_key(sequence: &Sequence) {
    let Some(value) = sequence.syntax().parent().filter(|parent| parent.kind() == SyntaxKind::VALUE) else { return };
    let blank_lines: Vec<SyntaxElement> = value
        .children_with_tokens()
        .skip_while(|el| el.kind() != SyntaxKind::NEWLINE)
        .skip(1)
        .take_while(|el| el.kind() == SyntaxKind::NEWLINE)
        .collect();
    detach_all(blank_lines);
}

fn remove_key(mapping: &Mapping, key: &str) {
    if let Some(entry) = mapping.entries().find(|entry| entry.key_matches(key)) {
        detach_all(leading_comment_lines(entry.syntax()));
    }
    mapping.remove(key);
}

// ---------------------------------------------------------------------------
// List matching

/// A run of unmatched items: `old[old]` was replaced by `new[new]`.
#[derive(Debug, PartialEq)]
struct Gap {
    old: std::ops::Range<usize>,
    new: std::ops::Range<usize>,
}

/// The non-empty gaps around a longest common subsequence of equal items.
fn gaps(old: &[Value], new: &[Value]) -> Vec<Gap> {
    // lcs[i][j]: length of the LCS of old[i..] and new[j..].
    let mut lcs = vec![vec![0usize; new.len() + 1]; old.len() + 1];
    for i in (0..old.len()).rev() {
        for j in (0..new.len()).rev() {
            lcs[i][j] = if old[i] == new[j] { lcs[i + 1][j + 1] + 1 } else { lcs[i + 1][j].max(lcs[i][j + 1]) };
        }
    }
    let mut gaps = Vec::new();
    let (mut i, mut j, mut gap_i, mut gap_j) = (0, 0, 0, 0);
    while i < old.len() && j < new.len() {
        if old[i] == new[j] {
            push_gap(&mut gaps, gap_i..i, gap_j..j);
            i += 1;
            j += 1;
            (gap_i, gap_j) = (i, j);
        } else if lcs[i + 1][j] >= lcs[i][j + 1] {
            i += 1;
        } else {
            j += 1;
        }
    }
    push_gap(&mut gaps, gap_i..old.len(), gap_j..new.len());
    gaps
}

fn push_gap(gaps: &mut Vec<Gap>, old: std::ops::Range<usize>, new: std::ops::Range<usize>) {
    if !old.is_empty() || !new.is_empty() {
        gaps.push(Gap { old, new });
    }
}

// ---------------------------------------------------------------------------
// New values

/// A value ready to be written, in the style the file already uses.
enum Fresh {
    /// yaml-edit picks the style: plain when safe, quoted when not, and a
    /// literal block for text with line breaks.
    Auto(String),
    Styled(ScalarValue),
    Int(i64),
    Float(f64),
    Bool(bool),
    Mapping(Mapping),
    Sequence(Sequence),
}

/// Integers this large or larger can't all be told apart as `f64`.
const MAX_EXACT_INTEGER: f64 = 9_007_199_254_740_992.0;

impl Fresh {
    fn new(value: &Value, existing: Option<&YamlNode>) -> Result<Self, PatchError> {
        Ok(match value {
            Value::String(text) => string_in_style(text, existing),
            Value::Bool(b) => Self::Bool(*b),
            Value::Number(n) => match (n.as_i64(), n.as_f64()) {
                (Some(i), _) => Self::Int(i),
                #[allow(clippy::cast_possible_truncation)]
                (None, Some(f)) if f.fract() == 0.0 && f.abs() < MAX_EXACT_INTEGER => Self::Int(f as i64),
                (None, Some(f)) => Self::Float(f),
                (None, None) => Self::Auto(n.to_string()),
            },
            Value::Null => Self::Styled(ScalarValue::plain("null")),
            Value::Object(_) | Value::Array(_) => Self::tree(value)?,
        })
    }

    /// A nested value, written as fresh YAML and parsed back into a node
    /// that can be spliced in.
    fn tree(value: &Value) -> Result<Self, PatchError> {
        let mut text = to_yaml(value).map_err(yaml_error)?;
        if value.is_array() {
            // yaml-edit keeps a sequence's own column when it becomes the
            // value of a top-level key; indented, it lands under the key.
            text = text
                .lines()
                .map(|line| if line.is_empty() { "\n".to_string() } else { format!("  {line}\n") })
                .collect();
        }
        let document = Document::from_str(&text).map_err(yaml_error)?;
        if let Some(mapping) = document.as_mapping() {
            return Ok(Self::Mapping(mapping));
        }
        document.as_sequence().map(Self::Sequence).ok_or_else(|| PatchError::UnexpectedShape(value.clone()))
    }
}

fn string_in_style(text: &str, existing: Option<&YamlNode>) -> Fresh {
    if text.contains('\n') {
        return Fresh::Auto(text.to_string());
    }
    let raw = existing.and_then(YamlNode::as_scalar).map_or_default(yaml_edit::Scalar::value);
    match raw.chars().next() {
        Some('"') => Fresh::Styled(ScalarValue::double_quoted(text)),
        Some('\'') => Fresh::Styled(ScalarValue::single_quoted(text)),
        _ => Fresh::Auto(text.to_string()),
    }
}

// ---------------------------------------------------------------------------
// Comments around list items and keys

fn sequence_entries(sequence: &Sequence) -> impl Iterator<Item = SyntaxNode> {
    sequence.syntax().children().filter(|node| node.kind() == SyntaxKind::SEQUENCE_ENTRY)
}

fn is_indentation(kind: SyntaxKind) -> bool {
    matches!(kind, SyntaxKind::INDENT | SyntaxKind::WHITESPACE)
}

/// The element before `element`. Before the first entry of a block
/// collection is the line that introduces the collection, so the walk
/// continues in front of it.
fn previous(element: &SyntaxElement) -> Option<SyntaxElement> {
    element.prev_sibling_or_token().or_else(|| {
        let parent = element.parent()?;
        matches!(parent.kind(), SyntaxKind::SEQUENCE | SyntaxKind::MAPPING)
            .then(|| parent.prev_sibling_or_token())
            .flatten()
    })
}

fn ends_a_line(element: Option<&SyntaxElement>) -> bool {
    match element {
        None => true,
        Some(NodeOrToken::Token(token)) => token.kind() == SyntaxKind::NEWLINE,
        Some(NodeOrToken::Node(node)) => node.text().to_string().ends_with('\n'),
    }
}

/// The whole-line comments directly above `entry`, up to a blank line or
/// anything else, with their indentation and line breaks.
fn leading_comment_lines(entry: &SyntaxNode) -> Vec<SyntaxElement> {
    let mut cursor = previous(&NodeOrToken::Node(entry.clone()));
    // The entry's own indentation stays.
    if cursor.as_ref().is_some_and(|el| is_indentation(el.kind())) {
        cursor = cursor.as_ref().and_then(previous);
    }
    let mut lines = Vec::new();
    loop {
        let Some(line_break) = cursor.clone().filter(|el| el.kind() == SyntaxKind::NEWLINE) else { break };
        let Some(comment) = previous(&line_break).filter(|el| el.kind() == SyntaxKind::COMMENT) else { break };
        let mut line = vec![line_break, comment.clone()];
        let mut before = previous(&comment);
        if let Some(indent) = before.clone().filter(|el| is_indentation(el.kind())) {
            before = previous(&indent);
            line.push(indent);
        }
        // A comment after a value on the same line belongs to that value.
        if !ends_a_line(before.as_ref()) {
            break;
        }
        lines.extend(line);
        cursor = before;
    }
    lines
}

fn detach_all(elements: impl IntoIterator<Item = SyntaxElement>) {
    for element in elements {
        match element {
            NodeOrToken::Node(node) => node.detach(),
            NodeOrToken::Token(token) => token.detach(),
        }
    }
}

/// `Sequence::insert` puts the new item directly in front of the item that
/// was at `index`, after that item's comments. Move whatever lies between
/// the previous item and the new one (those comments and blank lines) back
/// in front of the item they belong to.
fn keep_comments_with_next_item(sequence: &Sequence, index: usize) {
    let entries: Vec<SyntaxNode> = sequence_entries(sequence).collect();
    let (Some(previous_entry), Some(inserted), Some(next)) =
        (index.checked_sub(1).and_then(|i| entries.get(i)), entries.get(index), entries.get(index + 1))
    else {
        return;
    };
    let mut cursor = inserted.prev_sibling_or_token();
    if cursor.as_ref().is_some_and(|el| el.kind() == SyntaxKind::INDENT) {
        cursor = cursor.and_then(|el| el.prev_sibling_or_token());
    }
    let mut between = Vec::new();
    while let Some(element) = cursor {
        if element.as_node() == Some(previous_entry) {
            break;
        }
        cursor = element.prev_sibling_or_token();
        between.push(element);
    }
    between.reverse();
    // The line break that ends the previous item's line stays with it.
    let previous_ends_line = previous_entry.text().to_string().ends_with('\n');
    if !previous_ends_line && between.first().is_some_and(|el| el.kind() == SyntaxKind::NEWLINE) {
        between.remove(0);
    }
    if between.is_empty() {
        return;
    }
    detach_all(between.clone());
    let anchor = match next.prev_sibling_or_token() {
        Some(element) if element.kind() == SyntaxKind::INDENT => element.index(),
        _ => next.index(),
    };
    sequence.syntax().splice_children(anchor..anchor, between);
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    /// Parse `text` as plain data, apply `edit` to a copy, patch, and check
    /// the result against `expected` byte for byte.
    fn check(text: &str, edit: impl FnOnce(&mut Value), expected: &str) {
        let old: Value = serde_yaml_ng::from_str(text).unwrap();
        let mut new = old.clone();
        edit(&mut new);
        let patched = patch(text, &old, &new).unwrap();
        assert_eq!(patched, expected, "\n--- got ---\n{patched}\n--- want ---\n{expected}");
        let reparsed: Value = serde_yaml_ng::from_str(&patched).unwrap();
        assert_eq!(reparsed, new);
    }

    const REQUEST: &str = r#"# Sign in and keep the token.
info:
  name: Login # shown in the sidebar
  type: http

http:
  method: POST
  url: "{{baseUrl}}/login"
  # Sent with every login.
  headers:
    # Ask for JSON.
    - name: Accept
      value: application/json

    # Trace id, keep it.
    - name: X-Trace
      value: "1"
    - name: X-Last
      value: z
  body:
    type: json
    data: |-
      {"user": "{{user}}"}
"#;

    #[test]
    fn no_change_is_byte_identical() {
        check(REQUEST, |_| {}, REQUEST);
    }

    #[test]
    fn scalar_edits_keep_comments_and_quotes() {
        check(
            REQUEST,
            |v| {
                v["info"]["name"] = json!("Sign in");
                v["http"]["url"] = json!("{{baseUrl}}/signin");
                v["http"]["headers"][1]["value"] = json!("2");
            },
            &REQUEST
                .replace("name: Login #", "name: Sign in #")
                .replace(r#""{{baseUrl}}/login""#, r#""{{baseUrl}}/signin""#)
                .replace(r#"value: "1""#, r#"value: "2""#),
        );
    }

    #[test]
    fn multiline_body_becomes_a_block() {
        check(
            REQUEST,
            |v| v["http"]["body"]["data"] = json!("{\n  \"user\": \"{{user}}\"\n}"),
            &REQUEST.replace(
                "    data: |-\n      {\"user\": \"{{user}}\"}\n",
                "    data: |-\n      {\n        \"user\": \"{{user}}\"\n      }\n",
            ),
        );
    }

    #[test]
    fn removing_an_item_takes_its_comment() {
        check(
            REQUEST,
            |v| {
                v["http"]["headers"].as_array_mut().unwrap().remove(1);
            },
            &REQUEST.replace("\n    # Trace id, keep it.\n    - name: X-Trace\n      value: \"1\"", ""),
        );
        check(
            REQUEST,
            |v| {
                v["http"]["headers"].as_array_mut().unwrap().remove(0);
            },
            &REQUEST.replace("    # Ask for JSON.\n    - name: Accept\n      value: application/json\n\n", ""),
        );
    }

    #[test]
    fn inserting_an_item_leaves_comments_on_their_item() {
        check(
            REQUEST,
            |v| {
                v["http"]["headers"].as_array_mut().unwrap().insert(1, json!({"name": "X-New", "value": "n"}));
            },
            &REQUEST.replace(
                "      value: application/json\n",
                "      value: application/json\n    - name: X-New\n      value: n\n",
            ),
        );
        check(
            REQUEST,
            |v| v["http"]["headers"].as_array_mut().unwrap().push(json!({"name": "X-End", "value": "e"})),
            &REQUEST.replace("      value: z\n", "      value: z\n    - name: X-End\n      value: e\n"),
        );
    }

    #[test]
    fn new_keys_follow_field_order() {
        check(
            "http:\n  method: GET # verb\n  url: x\n",
            |v| {
                v["http"] =
                    json!({"method": "GET", "url": "x", "headers": [{"name": "A", "value": "1"}], "auth": "inherit"});
            },
            "http:\n  method: GET # verb\n  url: x\n  headers:\n    - name: A\n      value: '1'\n  auth: inherit\n",
        );
    }

    #[test]
    fn removing_a_key_takes_its_comment() {
        check(
            "a: 1\n# about b\nb: 2\nc: 3\n",
            |v| {
                v.as_object_mut().unwrap().remove("b");
            },
            "a: 1\nc: 3\n",
        );
    }

    #[test]
    fn unknown_keys_survive() {
        let text = "info:\n  name: A\n  tags: [x] # mine\nruntime:\n  scripts:\n    - type: tests\n      code: ok\n";
        let old = json!({"info": {"name": "A"}});
        let new = json!({"info": {"name": "B"}, "runtime": {"assertions": [{"expression": "res.status", "operator": "eq", "value": "200"}]}});
        let patched = patch(text, &old, &new).unwrap();
        assert!(patched.contains("name: B\n  tags: [x] # mine"), "{patched}");
        assert!(patched.contains("code: ok"), "{patched}");
        assert!(patched.contains("expression: res.status"), "{patched}");
    }

    #[test]
    fn comment_only_file_gets_content() {
        let patched = patch("# notes\n", &json!({}), &json!({"a": 1})).unwrap();
        assert_eq!(patched, "# notes\na: 1\n");
    }

    #[test]
    fn gaps_pair_changes() {
        let v = |items: &[i64]| items.iter().map(|n| json!(n)).collect::<Vec<_>>();
        assert_eq!(gaps(&v(&[1, 2, 3]), &v(&[1, 2, 3])), []);
        assert_eq!(gaps(&v(&[1, 2, 3]), &v(&[1, 9, 3])), [Gap { old: 1..2, new: 1..2 }]);
        assert_eq!(gaps(&v(&[1, 2]), &v(&[1, 2, 3])), [Gap { old: 2..2, new: 2..3 }]);
        assert_eq!(gaps(&v(&[1, 2, 3]), &v(&[3])), [Gap { old: 0..2, new: 0..0 }]);
    }
}
