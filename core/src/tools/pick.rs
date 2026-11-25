//! `- tool: pick`: choose one item of a list, usually one an earlier step
//! captured (`items: res.body.items`), and set variables from it:
//!
//! ```yaml
//! - tool: pick
//!   list: "{{items}}"
//!   label: <field>              # what the list shows
//!   set: {<variable>: <field>}  # what choosing an item sets
//! ```

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{Answer, Ask, Choice, Tool, ToolCx, ToolOutput, title_or};
use crate::collection::LenientString;
use crate::text::plain_string;
use crate::{Error, Result, jsonpath};

/// The key the chosen item's index is answered under.
pub const CHOICE: &str = "choice";

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Pick {
    #[serde(skip_serializing_if = "LenientString::is_empty")]
    pub title: LenientString,
    /// The items: a JSON array, e.g. `{{items}}`, or lines or commas of text.
    #[serde(skip_serializing_if = "LenientString::is_empty")]
    pub list: LenientString,
    /// Where in each item its label is, e.g. `name`; the item itself when not set.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub label: String,
    /// The variables to set from the chosen item, each to the value at a
    /// path in it (`code`, `user.email`, `tags[0]`); an empty path is the
    /// whole item.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub set: BTreeMap<String, String>,
    /// Older form of `set` with one entry: `name` set to the value at `value`.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub value: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub name: String,
}

/// The items of `list`: a JSON array, or else text split on lines or commas.
fn items(list: &str) -> Vec<Value> {
    if let Ok(Value::Array(items)) = serde_json::from_str(list) {
        return items;
    }
    let separator = if list.contains('\n') { '\n' } else { ',' };
    list.split(separator)
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(|item| Value::String(item.into()))
        .collect()
}

/// The text at `path` in `item` (`code`, `user.name`, `[0]`, and the same
/// written `item.code`), or the whole item for an empty path.
fn at(item: &Value, path: &str) -> Option<String> {
    let path = path.trim();
    let path = path.strip_prefix("item").filter(|rest| rest.is_empty() || rest.starts_with(['.', '['])).unwrap_or(path);
    if path.is_empty() {
        return Some(plain_string(item));
    }
    let path = if path.starts_with(['.', '[', '$']) { path.to_string() } else { format!(".{path}") };
    jsonpath::get(item, &path).map(plain_string)
}

impl Pick {
    /// Variable → path, from `set` and the older `name`/`value`.
    pub fn outputs(&self) -> BTreeMap<String, String> {
        let mut outputs: BTreeMap<String, String> = self
            .set
            .iter()
            .map(|(name, path)| (name.trim().to_string(), path.clone()))
            .filter(|(name, _)| !name.is_empty())
            .collect();
        if !self.name.trim().is_empty() {
            outputs.entry(self.name.trim().to_string()).or_insert_with(|| self.value.clone());
        }
        outputs
    }

    fn choices(&self) -> Vec<Choice> {
        let outputs = self.outputs();
        items(&self.list)
            .iter()
            .map(|item| {
                // A path the item doesn't have sets an empty value.
                let values: BTreeMap<String, String> =
                    outputs.iter().map(|(name, path)| (name.clone(), at(item, path).unwrap_or_default())).collect();
                let label = at(item, &self.label).unwrap_or_else(|| plain_string(item));
                Choice { label, values }
            })
            .collect()
    }
}

impl Tool for Pick {
    fn title(&self) -> String {
        title_or(&self.title, "Pick")
    }

    fn templates(&mut self) -> Vec<&mut String> {
        vec![&mut self.title.0, &mut self.list.0]
    }

    async fn run(&self, cx: &mut ToolCx<'_>) -> Result<ToolOutput> {
        if self.outputs().is_empty() {
            return Err(Error::Tool("No variable to set".into()));
        }
        let mut choices = self.choices();
        let only = match choices.len() {
            0 => return Err(Error::Tool("Nothing to pick from".into())),
            1 => Some(0),
            _ => None,
        };
        let ask = Ask::Choose { title: self.title(), choices: choices.clone() };
        let index = match cx.host.ask(ask).await {
            Answer::Given(answers) => answers.get(CHOICE).and_then(|index| index.parse().ok()).or(only),
            Answer::Stopped => return Err(Error::Tool("Stopped".into())),
            // With one item there's nothing to choose.
            Answer::Unattended => only,
        };
        let chosen = index
            .filter(|index| *index < choices.len())
            .map(|index| choices.swap_remove(index))
            .ok_or_else(|| Error::Tool("Needs a choice".into()))?;
        for (name, value) in chosen.values {
            cx.set(name, value);
        }
        Ok(ToolOutput::text(chosen.label))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn choices_set_values_from_paths_in_each_item() {
        let pick = Pick {
            list: r#"[{"id": 7, "code": "A1", "user": {"name": "Ann"}}, {"id": 8}]"#.into(),
            label: "user.name".into(),
            set: BTreeMap::from([("code".into(), "item.code".into()), ("id".into(), "id".into())]),
            ..Pick::default()
        };
        let choices = pick.choices();
        assert_eq!(choices[0].label, "Ann");
        assert_eq!(choices[0].values, BTreeMap::from([("code".into(), "A1".into()), ("id".into(), "7".into())]));
        assert_eq!(choices[1].label, r#"{"id":8}"#, "no label path: the item");
        assert_eq!(choices[1].values["code"], "", "a missing path sets nothing");

        let text = Pick { list: "eu, us ,".into(), name: "region".into(), ..Pick::default() };
        let values: Vec<_> = text.choices().iter().map(|c| c.values["region"].clone()).collect();
        assert_eq!(values, ["eu", "us"], "the older name/value form, whole item");
        assert_eq!(Pick { list: "a,1\nb,2".into(), ..Pick::default() }.choices().len(), 2);
    }
}
