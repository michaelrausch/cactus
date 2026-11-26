//! RFC 6238 codes for the vault's optional second step: HMAC-SHA1, 30-second
//! steps, 6 digits, what every authenticator app does by default. The secret
//! lives in the vault file sealed under the password's key, so it adds a
//! step to unlocking on this machine rather than a second factor proper.

use data_encoding::BASE32_NOPAD;
use hmac::{Hmac, KeyInit, Mac};
use qrcode::QrCode;
use qrcode::render::svg;
use serde::Serialize;
use sha1::Sha1;

use crate::{Error, Result};

pub(super) const SECRET_LEN: usize = 20;
const STEP_SECS: u64 = 30;
const DIGITS: u32 = 6;
const ISSUER: &str = "cactus";

/// A new secret to add to an authenticator app, before it's turned on.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct TotpSetup {
    /// Base32, for typing in by hand.
    pub secret: String,
    /// The `otpauth://` link as an SVG QR code.
    pub qr_svg: String,
}

pub(super) fn setup(secret: &[u8], account: &str) -> Result<TotpSetup> {
    let encoded = BASE32_NOPAD.encode(secret);
    let uri = format!("otpauth://totp/{ISSUER}:{}?secret={encoded}&issuer={ISSUER}", percent_encode(account));
    let qr = QrCode::new(uri.as_bytes()).map_err(|e| Error::Tool(format!("QR code: {e}")))?;
    let qr_svg = qr.render::<svg::Color<'_>>().min_dimensions(180, 180).quiet_zone(true).build();
    Ok(TotpSetup { secret: encoded, qr_svg })
}

/// The secret from its base32 form, as [`TotpSetup::secret`] gave it.
pub(super) fn decode(secret: &str) -> Result<Vec<u8>> {
    BASE32_NOPAD.decode(secret.trim().as_bytes()).map_err(|_| Error::InvalidTotpSecret)
}

/// Whether `code` is right now, or one step either side for clock drift.
pub(super) fn verify(secret: &[u8], code: &str, unix_secs: u64) -> bool {
    let code: String = code.chars().filter(|c| !c.is_whitespace()).collect();
    let step = unix_secs / STEP_SECS;
    [step.saturating_sub(1), step, step + 1].into_iter().any(|counter| format_code(hotp(secret, counter)) == code)
}

fn hotp(secret: &[u8], counter: u64) -> u32 {
    let mut mac = <Hmac<Sha1> as KeyInit>::new_from_slice(secret).expect("HMAC takes keys of any length");
    mac.update(&counter.to_be_bytes());
    let hash = mac.finalize().into_bytes();
    let offset = usize::from(hash[hash.len() - 1] & 0x0f);
    let truncated = u32::from_be_bytes([hash[offset] & 0x7f, hash[offset + 1], hash[offset + 2], hash[offset + 3]]);
    truncated % 10u32.pow(DIGITS)
}

/// The code an authenticator app shows at `unix_secs`.
#[cfg(test)]
pub(super) fn code_at(secret: &[u8], unix_secs: u64) -> String {
    format_code(hotp(secret, unix_secs / STEP_SECS))
}

fn format_code(code: u32) -> String {
    format!("{code:0width$}", width = DIGITS as usize)
}

fn percent_encode(text: &str) -> String {
    text.bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || b"-_.".contains(&b) {
                char::from(b).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// RFC 6238, appendix B (SHA-1), last six digits.
    #[test]
    fn matches_the_rfc_test_vectors() {
        let secret = b"12345678901234567890";
        for (time, code) in
            [(59, "287082"), (1_111_111_109, "081804"), (1_234_567_890, "005924"), (2_000_000_000, "279037")]
        {
            assert_eq!(format_code(hotp(secret, time / STEP_SECS)), code);
            assert!(verify(secret, code, time));
        }
        assert!(verify(secret, "287 082", 59 + STEP_SECS), "a step late, with a space");
        assert!(!verify(secret, "287082", 59 + 3 * STEP_SECS));
    }

    #[test]
    fn setup_round_trips_the_secret() {
        let setup = setup(b"12345678901234567890", "my project").unwrap();
        assert_eq!(decode(&setup.secret).unwrap(), b"12345678901234567890");
        assert!(setup.qr_svg.starts_with("<?xml") && setup.qr_svg.contains("<svg"));
    }
}
