//! OAuth2 tokens for requests whose auth is `oauth2`.
//!
//! Tokens are cached per sender: `.cactus/critters/<id>/tokens.json` for a
//! critter, `.cactus/tokens.json` for no one, sealed like the context.
//! Each is keyed by `tokenConfig.id`, or else by the grant, URL, client,
//! scope and user, so requests that ask for the same token share it.
//!
//! A send ([`token_for_send`]) uses the cached token, refreshes it when it
//! has expired, or gets a new one with the client credentials or password
//! grant. The authorization code grant needs a browser, so only
//! [`get_token`] runs it: it opens the authorization page and takes the
//! code from the redirect to the callback URL, which must be a loopback
//! `http://` URL cactus can listen on.

use std::collections::BTreeMap;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::path::{Path, PathBuf};
use std::time::Duration;

use data_encoding::BASE64URL_NOPAD;
use reqwest::header::{ACCEPT, CONTENT_TYPE, HeaderName, HeaderValue};
use reqwest::{RequestBuilder, Url};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

use crate::collection::{
    OAuth2, OAuth2CredentialsPlacement, OAuth2Flow, OAuth2Parameter, OAuth2ParameterPlacement, PkceMethod,
};
use crate::env::{self, Env, Scope, Substituter, ensure_critter_workspace, write_lock};
use crate::http::{BearerToken, effective_oauth2};
use crate::scenario::Scenario;
use crate::secrets::{read_sealed, write_sealed};
use crate::{Error, ProjectPath, Result, layout, now_unix_millis};

/// A token this close to expiring counts as expired, so it doesn't run out
/// on its way to the server.
const EXPIRY_MARGIN_MS: u64 = 30_000;
/// How long the authorization page has to redirect back.
const AUTHORIZE_TIMEOUT: Duration = Duration::from_mins(5);
/// More than any redirect's request line and headers.
const MAX_CALLBACK_REQUEST: usize = 16 * 1024;
const PKCE_VERIFIER_BYTES: usize = 32;
const STATE_BYTES: usize = 16;

/// A token as the cache keeps it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StoredToken {
    pub access_token: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id_token: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
    /// Unix epoch millis; `None` when the server didn't say.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<u64>,
    pub obtained_at: u64,
}

impl StoredToken {
    fn expired(&self, now: u64) -> bool {
        self.expires_at.is_some_and(|at| at <= now.saturating_add(EXPIRY_MARGIN_MS))
    }

    pub fn bearer(&self) -> BearerToken<'_> {
        BearerToken { access_token: &self.access_token, id_token: self.id_token.as_deref() }
    }
}

type Tokens = BTreeMap<String, StoredToken>;

/// What the UI shows about a request's token.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct TokenStatus {
    pub flow: OAuth2Flow,
    /// Whether a token is cached, expired or not.
    pub has_token: bool,
    #[cfg_attr(feature = "specta", specta(type = Option<specta_typescript::Number>))]
    pub expires_at: Option<u64>,
    #[cfg_attr(feature = "specta", specta(type = Option<specta_typescript::Number>))]
    pub obtained_at: Option<u64>,
    pub refreshable: bool,
    pub scope: Option<String>,
}

/// A token for one send, and whether it was got just now rather than
/// read from the cache.
pub(crate) struct SendToken {
    pub token: StoredToken,
    pub fresh: bool,
}

/// The token a send of `oauth2` as `critter` goes with: the cached one,
/// refreshed if it has expired, or else a new one.
pub(crate) async fn token_for_send(
    client: &reqwest::Client,
    project: &Path,
    critter: Option<&str>,
    oauth2: &OAuth2,
    vars: &Env,
) -> Result<SendToken> {
    let grant = Grant::resolve(oauth2, vars);
    let cached = load(project, critter)?.remove(&grant.key);
    let now = now_unix_millis();
    let mut refresh_failure = None;
    if let Some(token) = &cached {
        if !token.expired(now) {
            return Ok(SendToken { token: token.clone(), fresh: false });
        }
        if oauth2.auto_refresh()
            && let Some(refresh_token) = &token.refresh_token
        {
            match grant.refresh(client, refresh_token).await {
                Ok(token) => return store(project, critter, &grant.key, token).map(fresh),
                Err(error) => refresh_failure = Some(error),
            }
        }
    }
    if oauth2.auto_fetch() && grant.flow != OAuth2Flow::AuthorizationCode {
        return store(project, critter, &grant.key, grant.request_new(client).await?).map(fresh);
    }
    let reason = match (cached, refresh_failure) {
        (_, Some(error)) => format!("the token has expired and refreshing it failed ({error})"),
        (Some(_), None) => "the token has expired".into(),
        (None, None) => "no token yet".into(),
    };
    Err(Error::OAuth2(format!("{reason}; get one in the request's Auth tab")))
}

fn fresh(token: StoredToken) -> SendToken {
    SendToken { token, fresh: true }
}

/// Mark the cached token of `oauth2` expired, after the server refused it.
/// Its refresh token is kept.
pub(crate) fn expire(project: &Path, critter: Option<&str>, oauth2: &OAuth2, vars: &Env) -> Result<()> {
    let key = Grant::resolve(oauth2, vars).key;
    update(project, critter, |tokens| {
        if let Some(token) = tokens.get_mut(&key) {
            token.expires_at = Some(0);
        }
    })
}

/// The request's OAuth2 settings with what it would send them with.
struct RequestOAuth2 {
    oauth2: OAuth2,
    vars: Env,
}

fn request_oauth2(project: &Path, scope: &Scope, path: &ProjectPath, scenario: &Scenario) -> Result<RequestOAuth2> {
    let resolved = env::resolve_for_request(project, scope, path.dir(), &scenario.runtime.variables)?;
    let oauth2 = effective_oauth2(scenario, &resolved.inherited)
        .cloned()
        .ok_or_else(|| Error::OAuth2("this request doesn't use OAuth2".into()))?;
    Ok(RequestOAuth2 { oauth2, vars: resolved.vars })
}

/// The cached token of the request (as edited) when sent as `scope`.
pub fn token_status(project: &Path, scope: &Scope, path: &ProjectPath, scenario: &Scenario) -> Result<TokenStatus> {
    let RequestOAuth2 { oauth2, vars } = request_oauth2(project, scope, path, scenario)?;
    let grant = Grant::resolve(&oauth2, &vars);
    let token = load(project, scope.critter.as_deref())?.remove(&grant.key);
    Ok(status(oauth2.flow, token.as_ref()))
}

fn status(flow: OAuth2Flow, token: Option<&StoredToken>) -> TokenStatus {
    TokenStatus {
        flow,
        has_token: token.is_some(),
        expires_at: token.and_then(|t| t.expires_at),
        obtained_at: token.map(|t| t.obtained_at),
        refreshable: token.is_some_and(|t| t.refresh_token.is_some()),
        scope: token.and_then(|t| t.scope.clone()),
    }
}

/// Get a new token for the request, replacing the cached one. The
/// authorization code grant opens its authorization page with `open_url`
/// and waits for the browser to come back to the callback URL.
pub async fn get_token(
    client: &reqwest::Client,
    project: &Path,
    scope: &Scope,
    path: &ProjectPath,
    scenario: &Scenario,
    open_url: impl FnOnce(&str) -> Result<()>,
) -> Result<TokenStatus> {
    let RequestOAuth2 { oauth2, vars } = request_oauth2(project, scope, path, scenario)?;
    let grant = Grant::resolve(&oauth2, &vars);
    let token = match grant.flow {
        OAuth2Flow::AuthorizationCode => grant.authorize(client, open_url).await?,
        OAuth2Flow::ClientCredentials | OAuth2Flow::ResourceOwnerPasswordCredentials => {
            grant.request_new(client).await?
        }
    };
    let token = store(project, scope.critter.as_deref(), &grant.key, token)?;
    Ok(status(oauth2.flow, Some(&token)))
}

/// Forget the request's cached token.
pub fn clear_token(project: &Path, scope: &Scope, path: &ProjectPath, scenario: &Scenario) -> Result<()> {
    let RequestOAuth2 { oauth2, vars } = request_oauth2(project, scope, path, scenario)?;
    let key = Grant::resolve(&oauth2, &vars).key;
    update(project, scope.critter.as_deref(), |tokens| {
        tokens.remove(&key);
    })
}

fn tokens_file(project: &Path, critter: Option<&str>) -> PathBuf {
    match critter {
        Some(id) => layout::critter_tokens_file(project, id),
        None => layout::tokens_file(project),
    }
}

fn load(project: &Path, critter: Option<&str>) -> Result<Tokens> {
    read_sealed(project, &tokens_file(project, critter))
}

fn update(project: &Path, critter: Option<&str>, change: impl FnOnce(&mut Tokens)) -> Result<()> {
    let _guard = write_lock();
    let mut tokens = load(project, critter)?;
    change(&mut tokens);
    match critter {
        Some(id) => ensure_critter_workspace(project, id)?,
        None => {
            layout::ensure_workspace(project)?;
        }
    }
    write_sealed(project, &tokens_file(project, critter), &tokens)
}

fn store(project: &Path, critter: Option<&str>, key: &str, token: StoredToken) -> Result<StoredToken> {
    update(project, critter, |tokens| {
        tokens.insert(key.to_string(), token.clone());
    })?;
    Ok(token)
}

/// An `oauth2` auth with its `{{variables}}` filled in.
struct Grant {
    flow: OAuth2Flow,
    authorization_url: String,
    access_token_url: String,
    refresh_token_url: String,
    callback_url: String,
    client_id: String,
    client_secret: String,
    credentials_in_body: bool,
    username: String,
    password: String,
    scope: String,
    state: String,
    pkce: Option<PkceMethod>,
    authorization_params: Vec<Param>,
    access_token_params: Vec<Param>,
    refresh_token_params: Vec<Param>,
    key: String,
}

struct Param {
    name: String,
    value: String,
    placement: Option<OAuth2ParameterPlacement>,
}

impl Grant {
    fn resolve(oauth2: &OAuth2, vars: &Env) -> Self {
        let mut vars = Substituter::new(vars);
        let mut fill = |text: &str| vars.apply(text.trim());
        let credentials = oauth2.credentials.clone().unwrap_or_default();
        let owner = oauth2.resource_owner.clone().unwrap_or_default();
        let extra = oauth2.additional_parameters.clone().unwrap_or_default();
        let mut params = |list: &[OAuth2Parameter]| -> Vec<Param> {
            list.iter()
                .filter(|p| !p.name.trim().is_empty())
                .map(|p| Param { name: fill(&p.name), value: fill(&p.value), placement: p.placement })
                .collect()
        };
        let authorization_params = params(&extra.authorization_request);
        let access_token_params = params(&extra.access_token_request);
        let refresh_token_params = params(&extra.refresh_token_request);
        let mut grant = Self {
            flow: oauth2.flow,
            authorization_url: fill(&oauth2.authorization_url),
            access_token_url: fill(&oauth2.access_token_url),
            refresh_token_url: fill(&oauth2.refresh_token_url),
            callback_url: fill(&oauth2.callback_url),
            client_id: fill(&credentials.client_id),
            client_secret: fill(&credentials.client_secret),
            credentials_in_body: credentials.placement == Some(OAuth2CredentialsPlacement::Body),
            username: fill(&owner.username),
            password: fill(&owner.password),
            scope: fill(&oauth2.scope),
            state: fill(&oauth2.state),
            pkce: oauth2.uses_pkce().then(|| oauth2.pkce.as_ref().and_then(|p| p.method).unwrap_or(PkceMethod::S256)),
            authorization_params,
            access_token_params,
            refresh_token_params,
            key: String::new(),
        };
        let id = oauth2.token_config.as_ref().map_or_default(|config| fill(&config.id));
        grant.key = if id.is_empty() { grant.derived_key() } else { id };
        grant
    }

    /// Requests that would get the same token share it.
    fn derived_key(&self) -> String {
        let flow = match self.flow {
            OAuth2Flow::ClientCredentials => "client_credentials",
            OAuth2Flow::ResourceOwnerPasswordCredentials => "password",
            OAuth2Flow::AuthorizationCode => "authorization_code",
        };
        let user = if self.flow == OAuth2Flow::ResourceOwnerPasswordCredentials { self.username.as_str() } else { "" };
        // Newlines can't be in a URL, client id, scope or user name, so fields can't run together.
        [flow, &self.access_token_url, &self.client_id, &self.scope, user].join("\n")
    }

    /// A new token with the client credentials or password grant.
    async fn request_new(&self, client: &reqwest::Client) -> Result<StoredToken> {
        let mut form = Vec::new();
        match self.flow {
            OAuth2Flow::ClientCredentials => form.push(("grant_type", "client_credentials".into())),
            OAuth2Flow::ResourceOwnerPasswordCredentials => {
                form.push(("grant_type", "password".into()));
                form.push(("username", self.username.clone()));
                form.push(("password", self.password.clone()));
            }
            OAuth2Flow::AuthorizationCode => {
                return Err(Error::OAuth2("the authorization code grant needs a browser".into()));
            }
        }
        if !self.scope.is_empty() {
            form.push(("scope", self.scope.clone()));
        }
        self.token_request(client, &self.access_token_url, form, &self.access_token_params).await
    }

    async fn refresh(&self, client: &reqwest::Client, refresh_token: &str) -> Result<StoredToken> {
        let url = if self.refresh_token_url.is_empty() { &self.access_token_url } else { &self.refresh_token_url };
        let form = vec![("grant_type", "refresh_token".into()), ("refresh_token", refresh_token.to_string())];
        let mut token = self.token_request(client, url, form, &self.refresh_token_params).await?;
        // A server may keep the refresh token and not send it again.
        token.refresh_token.get_or_insert_with(|| refresh_token.to_string());
        Ok(token)
    }

    /// POST `form` to the token endpoint with the client's credentials, and
    /// read the token from the reply.
    async fn token_request(
        &self,
        client: &reqwest::Client,
        url: &str,
        mut form: Vec<(&str, String)>,
        extra: &[Param],
    ) -> Result<StoredToken> {
        if url.is_empty() {
            return Err(Error::OAuth2("no access token URL".into()));
        }
        let url = Url::parse(url).map_err(|e| Error::OAuth2(format!("invalid token URL {url:?}: {e}")))?;
        let mut builder = client.post(url.clone()).header(ACCEPT, "application/json");
        if self.credentials_in_body || self.client_secret.is_empty() {
            form.push(("client_id", self.client_id.clone()));
            if !self.client_secret.is_empty() {
                form.push(("client_secret", self.client_secret.clone()));
            }
        } else {
            builder = builder.basic_auth(&self.client_id, Some(&self.client_secret));
        }
        let mut body: Vec<(String, String)> = form.into_iter().map(|(k, v)| (k.to_string(), v)).collect();
        builder = add_params(builder, &mut body, extra, OAuth2ParameterPlacement::Body)?;
        let reply = builder.form(&body).send().await.map_err(|e| Error::OAuth2(format!("{url}: {e}")))?;
        let status = reply.status();
        let content_type = reply.headers().get(CONTENT_TYPE).and_then(|v| v.to_str().ok()).unwrap_or("").to_string();
        let text = reply.text().await.map_err(|e| Error::OAuth2(format!("{url}: {e}")))?;
        let fields = parse_token_reply(&text, &content_type);
        if !status.is_success() {
            return Err(Error::OAuth2(format!("{url} answered {status}{}", error_text(&fields, &text))));
        }
        token_from(&fields).ok_or_else(|| Error::OAuth2(format!("{url} sent no access_token")))
    }

    /// The authorization code grant: open the authorization page, take the
    /// code from the redirect, then exchange it for a token.
    async fn authorize(
        &self,
        client: &reqwest::Client,
        open_url: impl FnOnce(&str) -> Result<()>,
    ) -> Result<StoredToken> {
        let callback = loopback_callback(&self.callback_url)?;
        let listener = TcpListener::bind(callback.address)
            .await
            .map_err(|e| Error::OAuth2(format!("couldn't listen on {}: {e}", callback.address)))?;
        let state = if self.state.is_empty() { random_text(STATE_BYTES)? } else { self.state.clone() };
        let verifier = random_text(PKCE_VERIFIER_BYTES)?;
        let page = self.authorization_page(&state, &verifier)?;
        open_url(page.as_str())?;
        let code = tokio::time::timeout(AUTHORIZE_TIMEOUT, wait_for_code(&listener, &callback.path, &state))
            .await
            .map_err(|_| Error::OAuth2("the browser didn't come back within 5 minutes".into()))??;
        let mut form = vec![
            ("grant_type", "authorization_code".into()),
            ("code", code),
            ("redirect_uri", self.callback_url.clone()),
        ];
        if self.pkce.is_some() {
            form.push(("code_verifier", verifier));
        }
        self.token_request(client, &self.access_token_url, form, &self.access_token_params).await
    }

    fn authorization_page(&self, state: &str, verifier: &str) -> Result<Url> {
        if self.authorization_url.is_empty() {
            return Err(Error::OAuth2("no authorization URL".into()));
        }
        let mut url = Url::parse(&self.authorization_url)
            .map_err(|e| Error::OAuth2(format!("invalid authorization URL {:?}: {e}", self.authorization_url)))?;
        // It's opened by the OS, so a project file mustn't make it a local file or app.
        if !matches!(url.scheme(), "http" | "https") {
            return Err(Error::OAuth2(format!("the authorization URL must be http(s), not {:?}", url.scheme())));
        }
        {
            let mut query = url.query_pairs_mut();
            query.append_pair("response_type", "code");
            query.append_pair("client_id", &self.client_id);
            query.append_pair("redirect_uri", &self.callback_url);
            if !self.scope.is_empty() {
                query.append_pair("scope", &self.scope);
            }
            query.append_pair("state", state);
            if let Some(method) = self.pkce {
                let (challenge, name) = match method {
                    PkceMethod::S256 => (BASE64URL_NOPAD.encode(&Sha256::digest(verifier.as_bytes())), "S256"),
                    PkceMethod::Plain => (verifier.to_string(), "plain"),
                };
                query.append_pair("code_challenge", &challenge);
                query.append_pair("code_challenge_method", name);
            }
            // The authorization request is a page the browser opens, so
            // every extra parameter goes in its query.
            for param in &self.authorization_params {
                query.append_pair(&param.name, &param.value);
            }
        }
        Ok(url)
    }
}

/// Put each extra parameter where it says, or `default` when it doesn't.
fn add_params(
    mut builder: RequestBuilder,
    body: &mut Vec<(String, String)>,
    params: &[Param],
    default: OAuth2ParameterPlacement,
) -> Result<RequestBuilder> {
    for param in params {
        match param.placement.unwrap_or(default) {
            OAuth2ParameterPlacement::Body => body.push((param.name.clone(), param.value.clone())),
            OAuth2ParameterPlacement::Query => builder = builder.query(&[(&param.name, &param.value)]),
            OAuth2ParameterPlacement::Header => {
                let name = HeaderName::from_bytes(param.name.as_bytes())
                    .map_err(|_| Error::InvalidHeaderName(param.name.clone()))?;
                let value =
                    HeaderValue::from_str(&param.value).map_err(|_| Error::InvalidHeaderValue(param.name.clone()))?;
                builder = builder.header(name, value);
            }
        }
    }
    Ok(builder)
}

/// A token endpoint's reply: JSON, or the form encoding some servers
/// (GitHub's, without `Accept`) still use.
fn parse_token_reply(text: &str, content_type: &str) -> BTreeMap<String, Value> {
    if let Ok(Value::Object(object)) = serde_json::from_str(text) {
        return object.into_iter().collect();
    }
    if content_type.contains("x-www-form-urlencoded") || !text.trim_start().starts_with(['{', '<']) {
        let parsed = Url::parse(&format!("http://reply/?{}", text.trim())).ok();
        if let Some(url) = parsed {
            return url.query_pairs().map(|(k, v)| (k.into_owned(), Value::String(v.into_owned()))).collect();
        }
    }
    BTreeMap::new()
}

fn text_field(fields: &BTreeMap<String, Value>, name: &str) -> Option<String> {
    match fields.get(name)? {
        Value::String(text) if !text.is_empty() => Some(text.clone()),
        Value::Number(number) => Some(number.to_string()),
        _ => None,
    }
}

fn token_from(fields: &BTreeMap<String, Value>) -> Option<StoredToken> {
    let now = now_unix_millis();
    let expires_in = text_field(fields, "expires_in").and_then(|text| text.parse::<f64>().ok());
    Some(StoredToken {
        access_token: text_field(fields, "access_token")?,
        token_type: text_field(fields, "token_type"),
        refresh_token: text_field(fields, "refresh_token"),
        id_token: text_field(fields, "id_token"),
        scope: text_field(fields, "scope"),
        // Whole seconds from the server; a negative or absurd one is ignored.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        expires_at: expires_in
            .filter(|secs| secs.is_finite() && *secs >= 0.0 && *secs < 1e12)
            .map(|secs| now.saturating_add((secs * 1000.0) as u64)),
        obtained_at: now,
    })
}

/// `: invalid_client (Client authentication failed)`, or the start of the body.
fn error_text(fields: &BTreeMap<String, Value>, text: &str) -> String {
    match (text_field(fields, "error"), text_field(fields, "error_description")) {
        (Some(error), Some(description)) => format!(": {error} ({description})"),
        (Some(error), None) => format!(": {error}"),
        _ if text.trim().is_empty() => String::new(),
        _ => format!(": {}", text.trim().chars().take(200).collect::<String>()),
    }
}

fn random_text(bytes: usize) -> Result<String> {
    let mut buffer = vec![0; bytes];
    getrandom::fill(&mut buffer).map_err(|e| Error::OAuth2(format!("no random source: {e}")))?;
    Ok(BASE64URL_NOPAD.encode(&buffer))
}

struct Callback {
    address: SocketAddr,
    path: String,
}

/// Where to listen for the redirect: the callback URL must be `http://` on
/// a loopback host, with a port.
fn loopback_callback(callback_url: &str) -> Result<Callback> {
    let not_loopback = || {
        Error::OAuth2(format!(
            "the callback URL must be http://localhost:<port>/... for cactus to catch the code, not {callback_url:?}"
        ))
    };
    let url = Url::parse(callback_url).map_err(|_| not_loopback())?;
    if url.scheme() != "http" {
        return Err(not_loopback());
    }
    let ip = match url.host() {
        Some(url::Host::Domain(host)) if host.eq_ignore_ascii_case("localhost") => IpAddr::V4(Ipv4Addr::LOCALHOST),
        Some(url::Host::Ipv4(ip)) if ip.is_loopback() => IpAddr::V4(ip),
        Some(url::Host::Ipv6(ip)) if ip.is_loopback() => IpAddr::V6(Ipv6Addr::LOCALHOST),
        _ => return Err(not_loopback()),
    };
    let port = url.port_or_known_default().ok_or_else(not_loopback)?;
    Ok(Callback { address: SocketAddr::new(ip, port), path: url.path().to_string() })
}

/// Accept connections until the browser comes back to `path` with a code
/// (or an error). Anything else, such as a favicon, gets a 404.
async fn wait_for_code(listener: &TcpListener, path: &str, state: &str) -> Result<String> {
    loop {
        let (mut stream, _) =
            listener.accept().await.map_err(|e| Error::OAuth2(format!("waiting for the browser: {e}")))?;
        let Some(target) = read_request_target(&mut stream).await else { continue };
        let Ok(url) = Url::parse(&format!("http://callback{target}")) else { continue };
        if url.path() != path {
            respond(&mut stream, "404 Not Found", "Not found").await;
            continue;
        }
        let query: BTreeMap<String, String> = url.query_pairs().into_owned().collect();
        let outcome = if let Some(error) = query.get("error") {
            let description = query.get("error_description").map_or_default(|d| format!(" ({d})"));
            Err(Error::OAuth2(format!("authorization was refused: {error}{description}")))
        } else if query.get("state").map(String::as_str) != Some(state) {
            Err(Error::OAuth2("the browser came back with the wrong state".into()))
        } else if let Some(code) = query.get("code") {
            Ok(code.clone())
        } else {
            Err(Error::OAuth2("the browser came back without a code".into()))
        };
        let page = if outcome.is_ok() { "Done. You can close this tab." } else { "cactus couldn't get a token." };
        respond(&mut stream, "200 OK", page).await;
        return outcome;
    }
}

/// The request target of the request line, `/callback?code=...`.
async fn read_request_target(stream: &mut TcpStream) -> Option<String> {
    let mut buffer = Vec::new();
    let mut chunk = [0; 1024];
    while !buffer.windows(4).any(|w| w == b"\r\n\r\n") && buffer.len() < MAX_CALLBACK_REQUEST {
        let read = stream.read(&mut chunk).await.ok()?;
        if read == 0 {
            break;
        }
        buffer.extend_from_slice(&chunk[..read]);
    }
    let text = String::from_utf8_lossy(&buffer);
    let line = text.lines().next()?;
    let mut parts = line.split(' ');
    let (_method, target) = (parts.next()?, parts.next()?);
    target.starts_with('/').then(|| target.to_string())
}

async fn respond(stream: &mut TcpStream, status: &str, text: &str) {
    let body = format!("<!doctype html><meta charset=utf-8><title>cactus</title><p>{text}</p>");
    let reply = format!(
        "HTTP/1.1 {status}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    // The browser may already have gone; the code is what matters.
    let _ = stream.write_all(reply.as_bytes()).await;
    let _ = stream.shutdown().await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_replies_in_json_and_form_encoding() {
        let fields = parse_token_reply(r#"{"access_token":"a","expires_in":"60","refresh_token":"r"}"#, "");
        let token = token_from(&fields).unwrap();
        assert_eq!((token.access_token.as_str(), token.refresh_token.as_deref()), ("a", Some("r")));
        assert!(token.expires_at.unwrap() > token.obtained_at);

        let fields =
            parse_token_reply("access_token=b&token_type=bearer&scope=repo", "application/x-www-form-urlencoded");
        let token = token_from(&fields).unwrap();
        assert_eq!((token.access_token.as_str(), token.scope.as_deref(), token.expires_at), ("b", Some("repo"), None));

        let fields = parse_token_reply(r#"{"error":"invalid_client","error_description":"bad secret"}"#, "");
        assert!(token_from(&fields).is_none());
        assert_eq!(error_text(&fields, ""), ": invalid_client (bad secret)");
    }

    #[test]
    fn only_loopback_http_callbacks_are_caught() {
        let callback = loopback_callback("http://localhost:8765/cb").unwrap();
        assert_eq!((callback.address.to_string().as_str(), callback.path.as_str()), ("127.0.0.1:8765", "/cb"));
        assert_eq!(loopback_callback("http://[::1]:9/").unwrap().address.to_string(), "[::1]:9");
        for refused in ["https://localhost:1/cb", "http://example.com:1/cb", "myapp://cb", ""] {
            assert!(loopback_callback(refused).is_err(), "{refused}");
        }
    }

    #[test]
    fn the_cache_key_tells_apart_users_and_honours_token_ids() {
        let mut oauth2 = OAuth2::new(OAuth2Flow::ResourceOwnerPasswordCredentials);
        oauth2.access_token_url = "{{auth}}/token".into();
        oauth2.resource_owner =
            Some(crate::collection::OAuth2ResourceOwner { username: "{{user}}".into(), password: "p".into() });
        let vars = |user: &str| Env::from([("auth".into(), "https://id".into()), ("user".into(), user.into())]);
        let alice = Grant::resolve(&oauth2, &vars("alice")).key;
        assert_eq!(alice, "password\nhttps://id/token\n\n\nalice");
        assert_ne!(alice, Grant::resolve(&oauth2, &vars("bob")).key);

        oauth2.token_config = Some(crate::collection::OAuth2TokenConfig { id: "shared".into(), ..Default::default() });
        assert_eq!(Grant::resolve(&oauth2, &vars("bob")).key, "shared");
    }

    #[test]
    fn the_authorization_page_carries_pkce_and_extra_parameters() {
        let mut oauth2 = OAuth2::new(OAuth2Flow::AuthorizationCode);
        oauth2.authorization_url = "https://id/authorize?prompt=login".into();
        oauth2.callback_url = "http://localhost:1/cb".into();
        oauth2.scope = "openid profile".into();
        oauth2.pkce = Some(crate::collection::OAuth2Pkce::default());
        oauth2.additional_parameters = Some(crate::collection::OAuth2AdditionalParameters {
            authorization_request: vec![OAuth2Parameter {
                name: "audience".into(),
                value: "api".into(),
                placement: None,
            }],
            ..Default::default()
        });
        let page = Grant::resolve(&oauth2, &Env::new()).authorization_page("st", "verifier").unwrap();
        let query: BTreeMap<_, _> = page.query_pairs().into_owned().collect();
        assert_eq!(query["prompt"], "login");
        assert_eq!(query["response_type"], "code");
        assert_eq!(query["redirect_uri"], "http://localhost:1/cb");
        assert_eq!(query["scope"], "openid profile");
        assert_eq!(query["state"], "st");
        assert_eq!(query["code_challenge_method"], "S256");
        assert_eq!(query["code_challenge"], BASE64URL_NOPAD.encode(&Sha256::digest(b"verifier")));
        assert_eq!(query["audience"], "api");
    }
}
