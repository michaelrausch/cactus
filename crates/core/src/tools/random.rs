//! Generated values for `- tool: set`, written as the whole value:
//! `id: $uuid`.

use std::fmt::Write;

use crate::clock::now_unix_millis;

/// Every generator, for the editor's suggestions.
pub const GENERATORS: &[&str] = &["$uuid", "$timestamp", "$isoTime", "$randomInt", "$randomEmail"];

/// The value `name` generates, or `None` if it isn't a generator.
pub fn generate(name: &str) -> Option<String> {
    Some(match name {
        "$uuid" => uuid_v4(),
        "$timestamp" => now_unix_millis().to_string(),
        "$isoTime" => iso_time(now_unix_millis()),
        "$randomInt" => (u64::from_le_bytes(random_bytes()) % 1_000_000).to_string(),
        "$randomEmail" => format!("user-{}@example.com", &hex(&random_bytes::<8>())[..10]),
        _ => return None,
    })
}

fn random_bytes<const N: usize>() -> [u8; N] {
    let mut bytes = [0; N];
    // The OS source only fails on platforms cactus doesn't build for.
    getrandom::fill(&mut bytes).expect("the OS random source is available");
    bytes
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::with_capacity(bytes.len() * 2), |mut text, byte| {
        let _ = write!(text, "{byte:02x}");
        text
    })
}

fn uuid_v4() -> String {
    let mut bytes: [u8; 16] = random_bytes();
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    let h = hex(&bytes);
    format!("{}-{}-{}-{}-{}", &h[..8], &h[8..12], &h[12..16], &h[16..20], &h[20..])
}

/// `2026-11-25T01:02:03.456Z`.
fn iso_time(millis: u64) -> String {
    let (days, ms) = (millis / 86_400_000, millis % 86_400_000);
    let (year, month, day) = civil_from_days(i64::try_from(days).unwrap_or(0));
    let (h, m, s, frac) = (ms / 3_600_000, ms / 60_000 % 60, ms / 1000 % 60, ms % 1000);
    format!("{year:04}-{month:02}-{day:02}T{h:02}:{m:02}:{s:02}.{frac:03}Z")
}

/// Howard Hinnant's days-since-epoch to (year, month, day).
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = u32::try_from(doy - (153 * mp + 2) / 5 + 1).unwrap_or(1);
    let month = u32::try_from(if mp < 10 { mp + 3 } else { mp - 9 }).unwrap_or(1);
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generators() {
        let uuid = generate("$uuid").unwrap();
        assert_eq!((uuid.len(), &uuid[14..15], uuid.matches('-').count()), (36, "4", 4));
        assert_ne!(generate("$uuid"), generate("$uuid"));
        assert!(generate("$randomEmail").unwrap().ends_with("@example.com"));
        assert!(generate("$randomInt").unwrap().parse::<u32>().unwrap() < 1_000_000);
        assert_eq!(generate("uuid"), None);
        for name in GENERATORS {
            assert!(generate(name).is_some(), "{name}");
        }
    }

    #[test]
    fn iso_times() {
        assert_eq!(iso_time(0), "1970-01-01T00:00:00.000Z");
        assert_eq!(iso_time(1_795_568_523_456), "2026-11-25T01:02:03.456Z");
        assert_eq!(iso_time(951_782_400_000), "2000-02-29T00:00:00.000Z");
    }
}
