//! Inputs made from a value: the edge cases a fuzzer tries beyond its fixed
//! payloads. Seeded, so a run can be repeated exactly.

use serde_json::{Number, Value};

/// Characters that end strings, quote, escape or separate.
const SPECIAL: &[char] =
    &['\'', '"', '<', '>', '%', ';', '\\', '/', '&', '|', '`', '$', '\0', '\n', '{', '}', '[', ']', '#', '=', '?'];
/// Characters that trip up encoders, normalisers and length checks.
const ODD_UNICODE: &[char] =
    &['\u{0}', '\u{200b}', '\u{202e}', '\u{feff}', '\u{fffd}', '\u{1f600}', '\u{301}', '\u{10ffff}', 'İ', 'ß'];
/// Long enough to hit most length limits.
const LONG_CHARS: usize = 4096;

/// splitmix64: small, fast and good enough to pick mutations.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u32) -> Self {
        Self(u64::from(seed))
    }

    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    /// A number below `n`, which must not be 0.
    #[allow(clippy::cast_possible_truncation)] // The result is below `n`, a usize.
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }

    fn pick<T: Copy>(&mut self, items: &[T]) -> T {
        items[self.below(items.len())]
    }
}

/// A seed to use when none is given.
pub fn random_seed() -> u32 {
    let mut bytes = [0; 4];
    // Without randomness a fixed seed still fuzzes, just the same way each time.
    let _ = getrandom::fill(&mut bytes);
    u32::from_le_bytes(bytes).max(1)
}

/// Up to `count` different inputs made from `original`, none equal to it.
/// Numbers stay numbers where they can, so JSON bodies are tried with both.
pub fn mutations(original: &Value, count: usize, rng: &mut Rng) -> Vec<Value> {
    let text = match original {
        Value::String(text) => text.clone(),
        other => other.to_string(),
    };
    let mut made: Vec<Value> = Vec::new();
    for _ in 0..count * 10 {
        if made.len() == count {
            break;
        }
        let value = sanitise(mutate(original, &text, rng));
        if value != *original && !made.contains(&value) {
            made.push(value);
        }
    }
    made
}

fn mutate(original: &Value, text: &str, rng: &mut Rng) -> Value {
    let number = text.trim().parse::<f64>().ok().filter(|_| !text.trim().is_empty());
    match (number, rng.below(14)) {
        (Some(number), 11..) => numeric(original, number, rng),
        (_, 13) if original.is_boolean() => Value::Bool(!original.as_bool().unwrap_or_default()),
        (_, 12..) => typed(rng),
        (_, op) => Value::String(textual(op, text, rng)),
    }
}

/// Text with one thing changed.
fn textual(op: usize, text: &str, rng: &mut Rng) -> String {
    let mut chars: Vec<char> = text.chars().collect();
    let at = |rng: &mut Rng, len: usize| if len == 0 { 0 } else { rng.below(len + 1) };
    match op {
        0 => String::new(),
        1 => {
            let unit = if text.is_empty() { "A" } else { text };
            unit.repeat(LONG_CHARS / unit.chars().count().max(1) + 1)
        }
        2 if !chars.is_empty() => chars[..rng.below(chars.len())].iter().collect(),
        3 if !chars.is_empty() => {
            chars.remove(rng.below(chars.len()));
            chars.into_iter().collect()
        }
        4 => {
            let i = at(rng, chars.len());
            chars.insert(i, rng.pick(SPECIAL));
            chars.into_iter().collect()
        }
        5 if !chars.is_empty() => {
            let i = rng.below(chars.len());
            chars[i] = rng.pick(SPECIAL);
            chars.into_iter().collect()
        }
        6 if !chars.is_empty() => {
            let start = rng.below(chars.len());
            let end = start + 1 + rng.below(chars.len() - start);
            let slice: Vec<char> = chars[start..end].to_vec();
            chars.splice(end..end, slice.iter().copied().cycle().take(slice.len() * (1 + rng.below(8))));
            chars.into_iter().collect()
        }
        7 if !chars.is_empty() => {
            let i = rng.below(chars.len());
            chars[i] = flip(chars[i], rng);
            chars.into_iter().collect()
        }
        8 => (0..=rng.below(32)).map(|_| char::from(b' ' + u8::try_from(rng.below(95)).unwrap_or(0))).collect(),
        9 => {
            let i = at(rng, chars.len());
            chars.insert(i, rng.pick(ODD_UNICODE));
            chars.into_iter().collect()
        }
        _ => format!("{text}{}", rng.pick(&["%00", "%n", "\\", "../", "${", "\r\n", "' --", "\u{0}"])),
    }
}

/// A letter in the other case, or another character one bit away.
fn flip(c: char, rng: &mut Rng) -> char {
    if c.is_ascii_alphabetic() {
        return if c.is_ascii_lowercase() { c.to_ascii_uppercase() } else { c.to_ascii_lowercase() };
    }
    let flipped = u32::from(c) ^ (1 << rng.below(7));
    char::from_u32(flipped).unwrap_or('\u{fffd}')
}

/// A number at or past a boundary, near `number`.
#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
fn numeric(original: &Value, number: f64, rng: &mut Rng) -> Value {
    let whole = number as i64;
    let picks: [Value; 12] = [
        Value::from(whole.saturating_add(1)),
        Value::from(whole.saturating_sub(1)),
        Value::from(whole.saturating_neg()),
        Value::from(0),
        Value::from(-1),
        Value::from(2_147_483_648_i64),
        Value::from(i64::MAX),
        Value::from(i64::MIN),
        Value::from(u64::MAX),
        Number::from_f64(1e308).map_or(Value::Null, Value::Number),
        Number::from_f64(number + 0.5).map_or(Value::Null, Value::Number),
        Value::from(whole.saturating_mul(1_000_000)),
    ];
    let value = picks[rng.below(picks.len())].clone();
    // A number written as text stays text, as it would be sent.
    if original.is_number() { value } else { Value::String(value.to_string()) }
}

/// A value of another type: what a loosely typed handler may not expect.
fn typed(rng: &mut Rng) -> Value {
    match rng.below(6) {
        0 => Value::Null,
        1 => Value::Bool(true),
        2 => Value::Array(Vec::new()),
        3 => serde_json::json!({"$ne": null}),
        4 => serde_json::json!({"$gt": ""}),
        _ => Value::Array(vec![Value::from(1), Value::from("a")]),
    }
}

/// No `{{`: it would be read as a variable when the request is sent.
fn sanitise(value: Value) -> Value {
    match value {
        Value::String(mut text) => {
            // Again until none are left: `{{{` gives `{ {{` the first time.
            while text.contains("{{") {
                text = text.replace("{{", "{ {");
            }
            Value::String(text)
        }
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_same_seed_gives_the_same_inputs() {
        let original = Value::from("shoes");
        let first = mutations(&original, 20, &mut Rng::new(7));
        assert_eq!(first, mutations(&original, 20, &mut Rng::new(7)));
        assert_ne!(first, mutations(&original, 20, &mut Rng::new(8)));
        assert_eq!(first.len(), 20);
        assert!(!first.contains(&original), "never the original");
    }

    #[test]
    fn numbers_get_boundaries_and_keep_their_type() {
        let made = mutations(&Value::from(42), 60, &mut Rng::new(1));
        assert!(made.contains(&Value::from(43)) || made.contains(&Value::from(41)), "{made:?}");
        assert!(made.contains(&Value::from(i64::MAX)) || made.contains(&Value::from(u64::MAX)), "{made:?}");
        let text = mutations(&Value::from("42"), 60, &mut Rng::new(1));
        assert!(text.iter().all(|value| !value.is_number()), "a number in text stays text: {text:?}");
    }

    #[test]
    fn nothing_made_reads_as_a_variable() {
        for value in mutations(&Value::from("{{a"), 200, &mut Rng::new(3)) {
            assert!(!value.to_string().contains("{{"), "{value}");
        }
    }

    #[test]
    fn an_empty_value_still_mutates() {
        assert!(mutations(&Value::from(""), 10, &mut Rng::new(5)).len() >= 5);
    }
}
