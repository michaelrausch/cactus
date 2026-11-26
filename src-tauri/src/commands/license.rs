//! The edition, and the Pro edition's license key. Both builds have these
//! commands, so the bindings are the same; the free edition is always
//! licensed and has no key to enter.

use cactus_core::{Error, Result};
use serde::Serialize;
use tauri::AppHandle;

/// Which edition this is, and who it's licensed to.
#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Edition {
    pub pro: bool,
    /// The Pro license's holder, once a key is entered.
    pub licensee: Option<String>,
}

/// Which edition this is, and who it's licensed to.
#[tauri::command]
#[specta::specta]
pub fn edition() -> Edition {
    Edition { pro: cfg!(feature = "pro"), licensee: licensee() }
}

/// Check the key, keep it for next time and start what waited for it.
#[tauri::command]
#[specta::specta]
pub async fn activate_license(app: AppHandle, key: String) -> Result<Edition> {
    #[cfg(feature = "pro")]
    {
        use tauri::Manager;

        let dir = app.path().app_config_dir().map_err(|e| Error::Io(std::io::Error::other(e.to_string())))?;
        cactus_core::license::activate(&dir, &key)?;
        crate::start_services(&app).await;
        Ok(edition())
    }
    #[cfg(not(feature = "pro"))]
    {
        let _ = (app, key);
        Err(Error::InvalidLicense)
    }
}

/// Whether the app may be used: the free edition always, Pro once a key is
/// entered.
pub fn is_licensed() -> bool {
    !cfg!(feature = "pro") || licensee().is_some()
}

/// Fail unless the app may be used ([`is_licensed`]).
pub fn require_license() -> Result<()> {
    if is_licensed() { Ok(()) } else { Err(Error::Unlicensed) }
}

#[cfg(feature = "pro")]
fn licensee() -> Option<String> {
    cactus_core::license::active().map(|license| if license.name.is_empty() { license.email } else { license.name })
}

#[cfg(not(feature = "pro"))]
fn licensee() -> Option<String> {
    None
}
