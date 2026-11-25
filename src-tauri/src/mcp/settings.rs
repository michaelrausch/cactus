//! The MCP server's settings, kept per machine in the app's config folder
//! (not in the project: the token is a credential for this machine).

use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use cactus_core::Result;
use serde::{Deserialize, Serialize};

pub const DEFAULT_PORT: u16 = 47821;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct McpSettings {
    pub enabled: bool,
    /// Whether agents may create and save requests and flows.
    pub allow_edits: bool,
    pub port: u16,
    pub token: String,
}

impl Default for McpSettings {
    fn default() -> Self {
        Self { enabled: false, allow_edits: false, port: DEFAULT_PORT, token: String::new() }
    }
}

impl McpSettings {
    /// The saved settings, or the defaults; either way with a token.
    pub fn load(file: &Path) -> Self {
        let mut settings: Self =
            fs::read_to_string(file).ok().and_then(|text| serde_json::from_str(&text).ok()).unwrap_or_default();
        if settings.token.is_empty() {
            settings.token = new_token();
        }
        settings
    }

    pub fn save(&self, file: &Path) -> Result<()> {
        if let Some(dir) = file.parent() {
            fs::create_dir_all(dir)?;
        }
        Ok(fs::write(file, serde_json::to_string_pretty(self)?)?)
    }
}

/// 32 random bytes as hex.
pub fn new_token() -> String {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).expect("the OS random source is available");
    bytes.iter().fold(String::with_capacity(64), |mut hex, byte| {
        let _ = write!(hex, "{byte:02x}");
        hex
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_settings_get_defaults_and_a_token_that_is_kept() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("config/mcp.json");
        let settings = McpSettings::load(&file);
        assert!(!settings.enabled && !settings.allow_edits);
        assert_eq!((settings.port, settings.token.len()), (DEFAULT_PORT, 64));
        settings.save(&file).unwrap();
        assert_eq!(McpSettings::load(&file), settings);
        assert_ne!(new_token(), settings.token);
    }
}
