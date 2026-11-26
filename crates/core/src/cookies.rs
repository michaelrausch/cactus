//! Each critter's cookie jar: `.cactus/critters/<id>/cookies.json`, sealed
//! like its context. Sending as a critter sends the cookies its jar holds
//! for the URL and keeps what the responses set, redirects included.
//! Sending as no one uses no jar.
//!
//! The process shares one HTTP client, so the client's cookie provider
//! ([`ScopedJar`]) holds no cookies itself: it uses the jar of the send it
//! is called from, a task-local set by [`with_jar`]. reqwest calls it from
//! within the request's future, so concurrent sends as different critters
//! each see their own jar.
//!
//! A send works on a copy of the jar loaded when it starts. What responses
//! set is applied to the jar on disk again when it ends, under the context
//! write lock, so two sends as one critter don't lose each other's cookies.

use std::future::Future;
use std::io::Cursor;
use std::path::Path;
use std::sync::{Arc, Mutex, PoisonError};

use cookie_store::{CookieDomain, CookieExpiration, CookieStore, RawCookie};
use reqwest::Url;
use reqwest::header::HeaderValue;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::critter::validate_critter_id;
use crate::env::{ensure_critter_workspace, write_lock};
use crate::secrets::{read_sealed, write_sealed};
use crate::{Error, Result, layout};

tokio::task_local! {
    static ACTIVE: Arc<SendJar>;
}

/// The jar of one send: a working copy, and every `Set-Cookie` it received.
#[derive(Default)]
struct SendJar {
    inner: Mutex<SendJarInner>,
}

#[derive(Default)]
struct SendJarInner {
    store: CookieStore,
    received: Vec<(Vec<String>, Url)>,
}

impl SendJar {
    fn lock(&self) -> std::sync::MutexGuard<'_, SendJarInner> {
        self.inner.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// The shared client's cookie provider: the jar of the send it is called
/// from, or none.
pub(crate) struct ScopedJar;

impl reqwest::cookie::CookieStore for ScopedJar {
    fn set_cookies(&self, cookie_headers: &mut dyn Iterator<Item = &HeaderValue>, url: &Url) {
        let headers: Vec<String> = cookie_headers.filter_map(|value| value.to_str().ok()).map(str::to_string).collect();
        // Outside `with_jar` there is no jar and nothing is kept.
        let _ = ACTIVE.try_with(|jar| {
            let mut jar = jar.lock();
            store_set_cookies(&mut jar.store, &headers, url);
            jar.received.push((headers, url.clone()));
        });
    }

    fn cookies(&self, url: &Url) -> Option<HeaderValue> {
        ACTIVE.try_with(|jar| cookie_header(&jar.lock().store, url)).ok().flatten()
    }
}

fn store_set_cookies(store: &mut CookieStore, headers: &[String], url: &Url) {
    let cookies = headers.iter().filter_map(|header| RawCookie::parse(header.clone()).ok());
    store.store_response_cookies(cookies, url);
}

fn cookie_header(store: &CookieStore, url: &Url) -> Option<HeaderValue> {
    let header =
        store.get_request_values(url).map(|(name, value)| format!("{name}={value}")).collect::<Vec<_>>().join("; ");
    if header.is_empty() { None } else { HeaderValue::from_str(&header).ok() }
}

/// Run `send` with `critter`'s jar, then, if `keep`, keep the cookies its
/// responses set.
pub(crate) async fn with_jar<F: Future>(project: &Path, critter: &str, keep: bool, send: F) -> Result<F::Output> {
    let jar =
        Arc::new(SendJar { inner: Mutex::new(SendJarInner { store: load(project, critter)?, received: vec![] }) });
    let output = ACTIVE.scope(Arc::clone(&jar), send).await;
    let received = std::mem::take(&mut jar.lock().received);
    if keep && !received.is_empty() {
        update(project, critter, |store| {
            for (headers, url) in &received {
                store_set_cookies(store, headers, url);
            }
        })?;
    }
    Ok(output)
}

/// A cookie in a critter's jar, for showing.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct CookieEntry {
    pub name: String,
    pub value: String,
    /// The host it was set by, or the domain it was set for.
    pub domain: String,
    pub path: String,
    /// Unix epoch millis; `None` for a session cookie, which the jar keeps
    /// until it's cleared.
    #[cfg_attr(feature = "specta", specta(type = Option<specta_typescript::Number>))]
    pub expires: Option<i64>,
    pub secure: bool,
    pub http_only: bool,
}

/// Which cookie: they are told apart by domain, path and name.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct CookieKey {
    pub domain: String,
    pub path: String,
    pub name: String,
}

/// The unexpired cookies in `critter`'s jar, by domain, path and name.
pub fn list(project: &Path, critter: &str) -> Result<Vec<CookieEntry>> {
    validate_critter_id(critter)?;
    let store = load(project, critter)?;
    let mut entries: Vec<CookieEntry> = store
        .iter_unexpired()
        .map(|cookie| CookieEntry {
            name: cookie.name().to_string(),
            value: cookie.value().to_string(),
            domain: domain_text(&cookie.domain),
            path: cookie.path.to_string(),
            expires: match &cookie.expires {
                CookieExpiration::AtUtc(at) => Some(at.unix_timestamp().saturating_mul(1000)),
                CookieExpiration::SessionEnd => None,
            },
            secure: cookie.secure().unwrap_or(false),
            http_only: cookie.http_only().unwrap_or(false),
        })
        .collect();
    entries.sort_by(|a, b| (&a.domain, &a.path, &a.name).cmp(&(&b.domain, &b.path, &b.name)));
    Ok(entries)
}

/// Remove the given cookies, or with `None` empty the jar.
pub fn remove(project: &Path, critter: &str, cookies: Option<&[CookieKey]>) -> Result<()> {
    validate_critter_id(critter)?;
    update(project, critter, |store| match cookies {
        None => store.clear(),
        Some(keys) => {
            for key in keys {
                store.remove(&key.domain, &key.path, &key.name);
            }
        }
    })
}

fn domain_text(domain: &CookieDomain) -> String {
    match domain {
        CookieDomain::HostOnly(host) | CookieDomain::Suffix(host) => host.clone(),
        CookieDomain::NotPresent | CookieDomain::Empty => String::new(),
    }
}

/// The jar without expired cookies; an empty one when there is no file.
fn load(project: &Path, critter: &str) -> Result<CookieStore> {
    let json: Value = read_sealed(project, &layout::critter_cookies_file(project, critter))?;
    if json.is_null() {
        return Ok(CookieStore::default());
    }
    let bytes = serde_json::to_vec(&json)?;
    cookie_store::serde::json::load(Cursor::new(bytes)).map_err(|e| unreadable(project, critter, &*e))
}

fn update(project: &Path, critter: &str, change: impl FnOnce(&mut CookieStore)) -> Result<()> {
    let _guard = write_lock();
    let mut store = load(project, critter)?;
    change(&mut store);
    let mut bytes = Vec::new();
    // Session cookies are kept too: a critter's session lasts until its jar
    // is cleared, not until the app quits.
    cookie_store::serde::json::save_incl_expired_and_nonpersistent(&store, &mut bytes)
        .map_err(|e| unreadable(project, critter, &*e))?;
    let json: Value = serde_json::from_slice(&bytes)?;
    ensure_critter_workspace(project, critter)?;
    write_sealed(project, &layout::critter_cookies_file(project, critter), &json)
}

fn unreadable(project: &Path, critter: &str, error: &dyn std::fmt::Display) -> Error {
    let path = layout::critter_cookies_file(project, critter);
    Error::file(&path)(std::io::Error::new(std::io::ErrorKind::InvalidData, error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn url(text: &str) -> Url {
        Url::parse(text).unwrap()
    }

    #[test]
    fn a_jar_round_trips_and_keeps_session_cookies() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path();
        assert_eq!(list(p, "bob").unwrap(), []);
        update(p, "bob", |store| {
            store_set_cookies(
                store,
                &["sid=abc; Path=/; HttpOnly".into(), "pref=dark; Max-Age=3600".into()],
                &url("http://api.test/login"),
            );
        })
        .unwrap();
        let entries = list(p, "bob").unwrap();
        assert_eq!(entries.len(), 2);
        let sid = entries.iter().find(|c| c.name == "sid").unwrap();
        assert_eq!((sid.domain.as_str(), sid.path.as_str(), sid.expires, sid.http_only), ("api.test", "/", None, true));
        assert!(entries.iter().find(|c| c.name == "pref").unwrap().expires.is_some());
        assert_eq!(list(p, "alice").unwrap(), []);

        let store = load(p, "bob").unwrap();
        // `pref` has no Path, so it takes /login's default path, `/`.
        let header = cookie_header(&store, &url("http://api.test/me")).unwrap();
        let mut sent: Vec<&str> = header.to_str().unwrap().split("; ").collect();
        sent.sort_unstable();
        assert_eq!(sent, ["pref=dark", "sid=abc"]);
        assert!(cookie_header(&store, &url("http://other.test/")).is_none());

        let key = CookieKey { domain: "api.test".into(), path: "/".into(), name: "sid".into() };
        remove(p, "bob", Some(&[key])).unwrap();
        assert_eq!(list(p, "bob").unwrap().len(), 1);
        remove(p, "bob", None).unwrap();
        assert_eq!(list(p, "bob").unwrap(), []);
    }

    #[test]
    fn expired_cookies_are_dropped() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path();
        update(p, "bob", |store| {
            store_set_cookies(store, &["sid=abc".into()], &url("http://api.test/"));
            store_set_cookies(store, &["sid=; Max-Age=0".into()], &url("http://api.test/"));
        })
        .unwrap();
        assert_eq!(list(p, "bob").unwrap(), []);
    }
}
