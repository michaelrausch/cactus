//! Sending with digest auth, OAuth2 tokens and critters' cookie jars,
//! against a real local HTTP server.

use std::collections::BTreeMap;
use std::fs;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use digest_auth::{AuthContext, AuthorizationHeader, HttpMethod};

use super::test_server::{Reply, Request, serve_with_headers};
use super::*;
use crate::critter::Critter;
use crate::env::{Env, EnvFile};
use crate::http::build_client;
use crate::{cookies, layout, oauth2};

fn path(rel: &str) -> ProjectPath {
    ProjectPath::parse(rel).unwrap()
}

fn ok(body: &str) -> Reply {
    (200, vec![], body.into())
}

fn unauthorized() -> Reply {
    (401, vec![], "{}".into())
}

/// A project whose base env points at a server routing with `route`.
async fn project(route: impl Fn(&Request) -> Reply + Send + Sync + 'static) -> tempfile::TempDir {
    let base = serve_with_headers(route).await;
    let dir = tempfile::tempdir().unwrap();
    env::save(dir.path(), &EnvFile::Base, &Env::from([("base".into(), base)])).unwrap();
    dir
}

fn request(p: &Path, file: &str, yaml: &str) -> Scenario {
    fs::write(p.join(file), yaml).unwrap();
    scenario::load(p, &path(file)).unwrap()
}

async fn send(p: &Path, file: &str, scenario: &Scenario, critter: Option<&str>) -> Response {
    let scope = Scope { critter: critter.map(str::to_string), ..Scope::env(p, None) };
    send_and_record(&build_client(), p, &path(file), scenario, &scope, SendOptions::BACKGROUND).await.unwrap()
}

#[tokio::test]
async fn each_critter_keeps_its_own_cookies_set_on_redirects_too() {
    let dir = project(|request| match request.path.as_str() {
        // The session cookie comes on the redirect, not the page it leads to.
        "/login" => {
            (302, vec![("location", "/home".into()), ("set-cookie", "sid=abc; Path=/; HttpOnly".into())], String::new())
        }
        "/home" => ok("{}"),
        "/me" if request.header("cookie") == Some("sid=abc") => ok(r#"{"me": true}"#),
        _ => unauthorized(),
    })
    .await;
    let p = dir.path();
    for id in ["bob", "alice"] {
        critter::create(p, id, &Critter::default()).unwrap();
    }
    let login = request(p, "login.yml", "info: {name: Login}\nhttp: {method: POST, url: '{{base}}/login'}\n");
    let me = request(p, "me.yml", "info: {name: Me}\nhttp: {method: GET, url: '{{base}}/me'}\n");

    assert_eq!(send(p, "login.yml", &login, Some("bob")).await.status, 200);
    assert_eq!(send(p, "me.yml", &me, Some("bob")).await.status, 200);
    assert_eq!(send(p, "me.yml", &me, Some("alice")).await.status, 401, "alice has her own jar");
    assert_eq!(send(p, "me.yml", &me, None).await.status, 401, "no critter, no jar");

    let jar = cookies::list(p, "bob").unwrap();
    assert_eq!(jar.iter().map(|c| (c.name.as_str(), c.value.as_str())).collect::<Vec<_>>(), [("sid", "abc")]);
    cookies::remove(p, "bob", None).unwrap();
    assert_eq!(send(p, "me.yml", &me, Some("bob")).await.status, 401);
}

#[tokio::test]
async fn an_explicit_cookie_header_wins_over_the_jar() {
    let dir = project(|request| match request.path.as_str() {
        "/login" => (200, vec![("set-cookie", "sid=abc".into())], "{}".into()),
        _ => ok(&format!(r#"{{"cookie": "{}"}}"#, request.header("cookie").unwrap_or(""))),
    })
    .await;
    let p = dir.path();
    critter::create(p, "bob", &Critter::default()).unwrap();
    let login = request(p, "login.yml", "info: {name: Login}\nhttp: {method: POST, url: '{{base}}/login'}\n");
    send(p, "login.yml", &login, Some("bob")).await;
    let echo = request(
        p,
        "echo.yml",
        "info: {name: Echo}\nhttp:\n  method: GET\n  url: '{{base}}/echo'\n  headers:\n    - {name: Cookie, value: own=1}\n",
    );
    assert_eq!(send(p, "echo.yml", &echo, Some("bob")).await.body, r#"{"cookie": "own=1"}"#);
}

#[tokio::test]
async fn a_probe_sends_the_jar_but_keeps_none_of_its_cookies() {
    let dir = project(|request| match request.path.as_str() {
        "/login" => (200, vec![("set-cookie", "sid=abc".into())], "{}".into()),
        "/logout" => (200, vec![("set-cookie", "sid=; Max-Age=0".into())], "{}".into()),
        "/me" if request.header("cookie") == Some("sid=abc") => ok("{}"),
        _ => unauthorized(),
    })
    .await;
    let p = dir.path();
    critter::create(p, "bob", &Critter::default()).unwrap();
    let login = request(p, "login.yml", "info: {name: Login}\nhttp: {method: POST, url: '{{base}}/login'}\n");
    let logout = request(p, "logout.yml", "info: {name: Logout}\nhttp: {method: POST, url: '{{base}}/logout'}\n");
    let me = request(p, "me.yml", "info: {name: Me}\nhttp: {method: GET, url: '{{base}}/me'}\n");
    send(p, "login.yml", &login, Some("bob")).await;

    let scope = Scope { critter: Some("bob".into()), ..Scope::env(p, None) };
    let client = build_client();
    assert_eq!(send_probe(&client, p, &path("me.yml"), &me, &scope).await.unwrap().status, 200, "the jar is sent");
    send_probe(&client, p, &path("logout.yml"), &logout, &scope).await.unwrap();
    assert_eq!(send(p, "me.yml", &me, Some("bob")).await.status, 200, "the logout's cookie wasn't kept");
}

const DIGEST_CHALLENGE: &str = r#"Digest realm="api", nonce="n0nce", qop="auth", algorithm=MD5"#;

/// Whether `authorization` answers [`DIGEST_CHALLENGE`] for `u`:`secret`.
fn digest_ok(request: &Request) -> bool {
    let Some(header) = request.header("authorization").and_then(|h| AuthorizationHeader::parse(h).ok()) else {
        return false;
    };
    let mut expected = header.clone();
    let context = AuthContext::new_with_method(
        "u",
        "secret",
        header.uri.clone(),
        None::<Vec<u8>>,
        HttpMethod::from(request.method.as_str()),
    );
    expected.digest(&context);
    header.username == "u"
        && header.nonce == "n0nce"
        && header.uri == request.path
        && expected.response == header.response
}

#[tokio::test]
async fn digest_auth_answers_the_challenge() {
    let calls = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&calls);
    let dir = project(move |request| {
        counter.fetch_add(1, Ordering::SeqCst);
        if digest_ok(request) {
            ok(r#"{"ok": true}"#)
        } else {
            (
                401,
                vec![
                    ("www-authenticate", r#"Basic realm="api""#.into()),
                    ("www-authenticate", DIGEST_CHALLENGE.into()),
                ],
                "{}".into(),
            )
        }
    })
    .await;
    let p = dir.path();
    let yaml = |password: &str| {
        format!(
            "info: {{name: Secret}}\nhttp:\n  method: GET\n  url: '{{{{base}}}}/secret?x=1'\n  auth: {{type: digest, username: u, password: {password}}}\n"
        )
    };
    let secret = request(p, "secret.yml", &yaml("secret"));
    let response = send(p, "secret.yml", &secret, None).await;
    assert_eq!((response.status, calls.load(Ordering::SeqCst)), (200, 2));

    let wrong = request(p, "secret.yml", &yaml("wrong"));
    assert_eq!(send(p, "secret.yml", &wrong, None).await.status, 401);
}

/// An identity server: `/token` hands out `t1`, `t2`, ... and `/api` takes
/// only the latest. Every token request's body is kept.
struct IdServer {
    issued: Arc<AtomicUsize>,
    bodies: Arc<Mutex<Vec<String>>>,
}

async fn id_server(refresh_token: Option<&'static str>, expires_in: u32) -> (tempfile::TempDir, IdServer) {
    let issued = Arc::new(AtomicUsize::new(0));
    let bodies = Arc::new(Mutex::new(Vec::new()));
    let (count, kept) = (Arc::clone(&issued), Arc::clone(&bodies));
    let dir = project(move |request| match request.path.as_str() {
        "/token" => {
            kept.lock().unwrap().push(format!("{} | {}", request.header("authorization").unwrap_or(""), request.body));
            let n = count.fetch_add(1, Ordering::SeqCst) + 1;
            let refresh = refresh_token.map_or_default(|r| format!(r#", "refresh_token": "{r}""#));
            ok(&format!(r#"{{"access_token": "t{n}", "token_type": "Bearer", "expires_in": {expires_in}{refresh}}}"#))
        }
        "/api" if request.header("authorization") == Some(&format!("Bearer t{}", count.load(Ordering::SeqCst))) => {
            ok("{}")
        }
        _ => unauthorized(),
    })
    .await;
    (dir, IdServer { issued, bodies })
}

const CLIENT_CREDENTIALS: &str = "info: {name: Api}\nhttp:\n  method: GET\n  url: '{{base}}/api'\n  auth:\n    type: oauth2\n    flow: client_credentials\n    accessTokenUrl: '{{base}}/token'\n    credentials: {clientId: cid, clientSecret: shh}\n    scope: read\n";

#[tokio::test]
async fn client_credentials_tokens_are_cached_and_replaced_when_refused() {
    let (dir, server) = id_server(None, 3600).await;
    let p = dir.path();
    let api = request(p, "api.yml", CLIENT_CREDENTIALS);

    assert_eq!(send(p, "api.yml", &api, None).await.status, 200);
    assert_eq!(send(p, "api.yml", &api, None).await.status, 200);
    assert_eq!(server.issued.load(Ordering::SeqCst), 1, "the second send uses the cached token");
    let first = server.bodies.lock().unwrap()[0].clone();
    // `cid:shh` in a Basic header, and the grant in the form body.
    assert_eq!(first, "Basic Y2lkOnNoaA== | grant_type=client_credentials&scope=read");

    // The server stops taking t1: the send gets a new token and goes again.
    server.issued.fetch_add(1, Ordering::SeqCst);
    let response = send(p, "api.yml", &api, None).await;
    assert_eq!(response.status, 200, "{:?}", response.warnings);
    assert!(response.warnings.iter().any(|w| w.contains("sent again with a new one")));
    assert_eq!(server.issued.load(Ordering::SeqCst), 3);
    assert!(layout::tokens_file(p).exists() && !layout::critter_tokens_file(p, "bob").exists());
}

#[tokio::test]
async fn a_probe_keeps_the_401_when_its_token_is_refused() {
    let (dir, server) = id_server(None, 3600).await;
    let p = dir.path();
    let api = request(p, "api.yml", CLIENT_CREDENTIALS);
    assert_eq!(send(p, "api.yml", &api, None).await.status, 200);

    server.issued.fetch_add(1, Ordering::SeqCst);
    let probe = send_probe(&build_client(), p, &path("api.yml"), &api, &Scope::env(p, None)).await.unwrap();
    assert_eq!((probe.status, server.issued.load(Ordering::SeqCst)), (401, 2), "no new token");
}

#[tokio::test]
async fn each_critter_gets_its_own_token_with_the_password_grant_and_refreshes_it() {
    let (dir, server) = id_server(Some("r1"), 0).await;
    let p = dir.path();
    for id in ["bob", "alice"] {
        critter::create(p, id, &Critter::default()).unwrap();
        env::save(p, &EnvFile::Critter(id.into()), &Env::from([("user".into(), id.into())])).unwrap();
    }
    let api = request(
        p,
        "api.yml",
        "info: {name: Api}\nhttp:\n  method: GET\n  url: '{{base}}/api'\n  auth:\n    type: oauth2\n    flow: resource_owner_password_credentials\n    accessTokenUrl: '{{base}}/token'\n    credentials: {clientId: app, placement: body}\n    resourceOwner: {username: '{{user}}', password: pw}\n",
    );
    assert_eq!(send(p, "api.yml", &api, Some("bob")).await.status, 200);
    // The token came with expires_in 0, so the next send refreshes it.
    assert_eq!(send(p, "api.yml", &api, Some("bob")).await.status, 200);
    assert_eq!(send(p, "api.yml", &api, Some("alice")).await.status, 200);
    let bodies = server.bodies.lock().unwrap().clone();
    assert_eq!(bodies[0], " | grant_type=password&username=bob&password=pw&client_id=app");
    assert_eq!(bodies[1], " | grant_type=refresh_token&refresh_token=r1&client_id=app");
    assert_eq!(bodies[2], " | grant_type=password&username=alice&password=pw&client_id=app");
    assert!(layout::critter_tokens_file(p, "bob").exists() && !layout::tokens_file(p).exists());
}

#[tokio::test]
async fn the_authorization_code_grant_needs_a_token_got_by_hand() {
    let (dir, server) = id_server(None, 3600).await;
    let p = dir.path();
    let port = std::net::TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();
    let api = request(
        p,
        "api.yml",
        &format!(
            "info: {{name: Api}}\nhttp:\n  method: GET\n  url: '{{{{base}}}}/api'\n  auth:\n    type: oauth2\n    flow: authorization_code\n    authorizationUrl: '{{{{base}}}}/authorize'\n    accessTokenUrl: '{{{{base}}}}/token'\n    callbackUrl: 'http://127.0.0.1:{port}/cb'\n    credentials: {{clientId: app}}\n    pkce: {{}}\n"
        ),
    );
    let scope = Scope::env(p, None);
    let client = build_client();
    let error = send_and_record(&client, p, &path("api.yml"), &api, &scope, SendOptions::BACKGROUND).await.unwrap_err();
    assert_eq!(error.to_string(), "OAuth2: no token yet; get one in the request's Auth tab");
    assert!(!oauth2::token_status(p, &scope, &path("api.yml"), &api).unwrap().has_token);

    // The "browser" follows the authorization page straight back to the callback.
    let browser = |page: &str| {
        let page = reqwest::Url::parse(page).unwrap();
        let query: BTreeMap<String, String> = page.query_pairs().into_owned().collect();
        assert_eq!(query["code_challenge_method"], "S256");
        let back = format!("{}?code=c0de&state={}", query["redirect_uri"], query["state"]);
        tokio::spawn(async move { reqwest::get(back).await.unwrap() });
        Ok(())
    };
    let status = oauth2::get_token(&client, p, &scope, &path("api.yml"), &api, browser).await.unwrap();
    assert!(status.has_token && status.expires_at.is_some());
    let body = server.bodies.lock().unwrap()[0].clone();
    assert!(body.starts_with(&format!(" | grant_type=authorization_code&code=c0de&redirect_uri=http%3A%2F%2F127.0.0.1%3A{port}%2Fcb&code_verifier=")), "{body}");
    assert_eq!(send(p, "api.yml", &api, None).await.status, 200);

    oauth2::clear_token(p, &scope, &path("api.yml"), &api).unwrap();
    assert!(!oauth2::token_status(p, &scope, &path("api.yml"), &api).unwrap().has_token);
}
