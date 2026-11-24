use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use super::test_server::serve;
use super::*;
use crate::collection::{LenientString, Variable};
use crate::critter::{Critter, Flow, FlowStep};
use crate::env::{Env, EnvFile};
use crate::http::build_client;
use crate::layout;

const LOGIN: &str = "info: {name: Login, type: http, tags: [auth]}\nhttp: {method: POST, url: '{{base}}/login'}\n";
const ME: &str = "info: {name: Me, type: http}\nhttp:\n  method: GET\n  url: '{{base}}/me'\n  headers:\n    - name: Authorization\n      value: 'Bearer {{token}}'\n";
const ADMIN: &str = "info: {name: Admin, type: http}\nhttp: {method: DELETE, url: '{{base}}/admin'}\n";
const BAD_LOGIN: &str = "info: {name: Bad, type: http}\nhttp: {method: POST, url: '{{base}}/bad-login'}\n";

/// A project talking to a fresh server, and how often `/me` was asked.
async fn project() -> (tempfile::TempDir, Arc<AtomicUsize>) {
    let me_calls = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&me_calls);
    let base = serve(move |request| match request.path.as_str() {
        "/login" => (200, r#"{"token": "fresh"}"#.into()),
        "/me" => {
            counter.fetch_add(1, Ordering::SeqCst);
            match request.header("authorization") {
                Some("Bearer fresh") => (200, r#"{"version": "1.4.2"}"#.into()),
                _ => (401, "{}".into()),
            }
        }
        "/admin" if request.method == "DELETE" => (200, "{}".into()),
        "/course/fresh" => (200, "{}".into()),
        _ => (401, "{}".into()),
    })
    .await;
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path();
    env::save(p, &EnvFile::Base, &Env::from([("base".into(), base)])).unwrap();
    fs::create_dir_all(p.join("auth")).unwrap();
    for (file, text) in [("auth/login.yml", LOGIN), ("me.yml", ME), ("admin.yml", ADMIN), ("bad-login.yml", BAD_LOGIN)]
    {
        fs::write(p.join(file), text).unwrap();
    }
    (dir, me_calls)
}

fn step(request: &str) -> FlowStep {
    FlowStep { request: request.into(), ..FlowStep::default() }
}

fn login_step() -> FlowStep {
    FlowStep {
        capture: BTreeMap::from([("token".into(), LenientString::from("res.body.token"))]),
        ..step("auth/login.yml")
    }
}

fn flow(steps: Vec<FlowStep>) -> Flow {
    Flow { name: "F".into(), steps, ..Flow::default() }
}

fn critter_with_login(p: &Path, id: &str, login: &str, login_steps: Vec<FlowStep>) {
    critter::create(p, id, &Critter { login: Some(login.into()), ..Critter::default() }).unwrap();
    critter::create_flow(p, id, login, &flow(login_steps)).unwrap();
}

fn path(rel: &str) -> ProjectPath {
    ProjectPath::parse(rel).unwrap()
}

#[tokio::test]
async fn flow_passes_captures_along_and_skips_after_a_failure() {
    let (dir, _) = project().await;
    let p = dir.path();
    critter::create(p, "bob", &Critter::default()).unwrap();
    let watch_version = FlowStep { watch: vec!["res.body.version".into()], ..step("me.yml") };
    let expect_403 = FlowStep { expect: Some(403), ..step("admin.yml") };
    critter::create_flow(p, "bob", "main", &flow(vec![login_step(), watch_version, expect_403, step("me.yml")]))
        .unwrap();

    let (mut started, mut finished) = (Vec::new(), Vec::new());
    let client = build_client();
    let run = run_flow(&client, p, "bob", "main", None, SendOptions::BACKGROUND, |event| match event {
        FlowEvent::Started(start) => started.push((start.index, start.method.clone(), start.url.clone())),
        FlowEvent::Finished(result) => finished.push(result.index),
    })
    .await
    .unwrap();
    assert_eq!(finished, [0, 1, 2, 3]);
    assert_eq!(started.iter().map(|(index, ..)| *index).collect::<Vec<_>>(), [0, 1, 2], "a skipped step never starts");
    let (_, method, url) = &started[0];
    assert!(method == "POST" && url.ends_with("/login") && !url.contains("{{"), "{method} {url}");
    assert_eq!(&run.steps[0].url, url);
    let states: Vec<_> = run.steps.iter().map(|s| (s.title.as_str(), s.ok, s.skipped)).collect();
    assert_eq!(states, [("Login", true, false), ("Me", true, false), ("Admin", false, false), ("me.yml", false, true)]);
    assert!(!run.ok);
    assert_eq!(run.steps[1].watched["res.body.version"].as_deref(), Some("1.4.2"));
    assert_eq!(run.failure().unwrap(), "step 3 (admin.yml): res.status: expected eq 403, got 200");
    assert_eq!(run.steps[1].response.as_ref().unwrap().critter.as_deref(), Some("bob"));
    assert_eq!(env::load_context(p, "bob").unwrap()["token"].from, "auth/login.yml");
    assert!(!p.join(".cactus/history").exists(), "background runs leave history alone");
    assert!(!layout::captured_file(p).exists(), "a critter's captures stay in its context");
}

#[tokio::test]
async fn with_values_take_captures_from_earlier_steps() {
    let (dir, _) = project().await;
    let p = dir.path();
    fs::write(p.join("course.yml"), "info: {name: Course}\nhttp: {method: GET, url: '{{base}}/course/{{COURSE}}'}\n")
        .unwrap();
    critter::create(p, "bob", &Critter::default()).unwrap();
    let course = FlowStep {
        with: BTreeMap::from([("COURSE".into(), LenientString::from("{{token}}"))]),
        expect: Some(200),
        ..step("course.yml")
    };
    critter::create_flow(p, "bob", "main", &flow(vec![login_step(), course])).unwrap();
    let run = run_flow(&build_client(), p, "bob", "main", None, SendOptions::BACKGROUND, |_| {}).await.unwrap();
    assert!(run.steps[1].url.ends_with("/course/fresh"), "{}", run.steps[1].url);
    assert!(run.ok, "{:?}", run.failure());
}

#[tokio::test]
async fn bad_request_paths_are_step_errors() {
    let (dir, _) = project().await;
    let p = dir.path();
    critter::create(p, "bob", &Critter::default()).unwrap();
    critter::create_flow(p, "bob", "broken", &flow(vec![step("nope.yml")])).unwrap();
    critter::create_flow(p, "bob", "escape", &flow(vec![step("../x.yml")])).unwrap();
    critter::create_flow(p, "bob", "not-a-request", &flow(vec![step("environments/dev.yml")])).unwrap();
    let client = build_client();
    for name in ["broken", "escape", "not-a-request"] {
        let run = run_flow(&client, p, "bob", name, None, SendOptions::BACKGROUND, |_| {}).await.unwrap();
        assert!(!run.ok && run.steps[0].error.is_some(), "{name}");
    }
    let missing = run_flow(&client, p, "bob", "gone", None, SendOptions::BACKGROUND, |_| {}).await;
    assert!(matches!(missing, Err(Error::FlowNotFound { .. })));
    let nobody = run_flow(&client, p, "nobody", "x", None, SendOptions::BACKGROUND, |_| {}).await;
    assert!(matches!(nobody, Err(Error::CritterNotFound(_))));
}

#[tokio::test]
async fn a_401_logs_in_again_and_resends_once() {
    let (dir, me_calls) = project().await;
    let p = dir.path();
    critter_with_login(p, "bob", "login", vec![login_step()]);
    env::merge_context(p, "bob", &Env::from([("token".into(), "stale".into())]), &path("auth/login.yml")).unwrap();
    let scenario = scenario::load(p, &path("me.yml")).unwrap();
    let scope = Scope::env(p, None).with_critter(Some("bob".into()));

    let client = build_client();
    let response = send_as(&client, p, &path("me.yml"), &scenario, &scope, SendOptions::INTERACTIVE).await.unwrap();
    assert_eq!((response.status, response.relogged), (200, true));
    assert!(response.warnings.iter().any(|w| w.contains("sent twice")), "{:?}", response.warnings);
    assert_eq!(me_calls.load(Ordering::SeqCst), 2);
    assert_eq!(env::load_context(p, "bob").unwrap()["token"].value, "fresh");
    assert_eq!(history::load(p, &path("me.yml")).len(), 2, "both sends are in history");
}

#[tokio::test]
async fn a_failed_login_returns_the_original_401() {
    let (dir, me_calls) = project().await;
    let p = dir.path();
    // The login flow itself gets a 401, which must not log in again.
    critter_with_login(p, "alice", "login", vec![step("bad-login.yml")]);
    let scenario = scenario::load(p, &path("me.yml")).unwrap();
    let scope = Scope::env(p, None).with_critter(Some("alice".into()));
    let client = build_client();
    let response = send_as(&client, p, &path("me.yml"), &scenario, &scope, SendOptions::BACKGROUND).await.unwrap();
    assert_eq!((response.status, response.relogged), (401, false));
    assert!(response.warnings.iter().any(|w| w.contains("failed: step 1 (bad-login.yml)")), "{:?}", response.warnings);
    assert_eq!(me_calls.load(Ordering::SeqCst), 1);

    // Running the login flow on its own terminates too.
    let run = run_flow(&client, p, "alice", "login", None, SendOptions::BACKGROUND, |_| {}).await.unwrap();
    assert!(!run.ok);
}

#[tokio::test]
async fn critters_keep_their_captures_apart() {
    let (dir, _) = project().await;
    let p = dir.path();
    critter::create(p, "alice", &Critter::default()).unwrap();
    critter::create(p, "bob", &Critter::default()).unwrap();
    let mut login = scenario::load(p, &path("auth/login.yml")).unwrap();
    login.runtime.actions.push(scenario::Action::SetVariable(scenario::SetVariable::new("token", "res.body.token")));
    let client = build_client();
    let as_alice = Scope::env(p, None).with_critter(Some("alice".into()));
    send_and_record(&client, p, &path("auth/login.yml"), &login, &as_alice, SendOptions::BACKGROUND).await.unwrap();

    let as_bob = Scope::env(p, None).with_critter(Some("bob".into()));
    let bob_vars = env::resolve_for_request(p, &as_bob, "", &[]).unwrap().vars;
    assert!(!bob_vars.contains_key("token"));
    assert!(!layout::captured_file(p).exists());

    send_and_record(&client, p, &path("auth/login.yml"), &login, &Scope::env(p, None), SendOptions::BACKGROUND)
        .await
        .unwrap();
    assert_eq!(env::load(p, &EnvFile::Captured).unwrap()["token"], "fresh");
}

#[tokio::test]
async fn matrix_sends_as_everyone_and_skips_missing_secrets() {
    let (dir, _) = project().await;
    let p = dir.path();
    critter_with_login(p, "bob", "login", vec![login_step()]);
    let secret = Variable { secret: true, ..Variable::new("API_KEY", "") };
    critter::create(p, "carol", &Critter { variables: vec![secret], ..Critter::default() }).unwrap();
    let scenario = scenario::load(p, &path("me.yml")).unwrap();

    let mut seen = Vec::new();
    let client = build_client();
    let cells = run_matrix(&client, p, &path("me.yml"), &scenario, |cell: &MatrixCell| seen.push(cell.critter.clone()))
        .await
        .unwrap();
    assert_eq!(seen, [None, Some("bob".into()), Some("carol".into())]);
    let statuses: Vec<_> = cells.iter().map(|c| c.response.as_ref().map(|r| r.status)).collect();
    assert_eq!(statuses, [Some(401), Some(200), None]);
    assert_eq!(cells[2].skipped.as_deref(), Some("missing secrets: API_KEY"));
    assert!(!p.join(".cactus/history").exists());
}

#[cfg(feature = "encryption")]
#[tokio::test]
async fn locked_secrets_stop_runs_before_anything_is_sent() {
    let (dir, me_calls) = project().await;
    let p = dir.path();
    critter::create(p, "bob", &Critter::default()).unwrap();
    critter::create_flow(p, "bob", "main", &flow(vec![step("me.yml")])).unwrap();
    env::save(p, &EnvFile::Secrets, &Env::from([("token".into(), "fresh".into())])).unwrap();
    crate::secrets::enable(p, &"password1".to_string().into(), None).unwrap();
    crate::secrets::lock(p);

    let client = build_client();
    let scenario = scenario::load(p, &path("me.yml")).unwrap();
    let locked = |e: Option<Error>| matches!(e, Some(Error::SecretsLocked));
    let sent = send_as(&client, p, &path("me.yml"), &scenario, &Scope::default(), SendOptions::INTERACTIVE).await;
    assert!(locked(sent.err()));
    assert!(locked(run(&client, p, "", &Scope::default(), |_| {}).await.err()));
    assert!(locked(run_flow(&client, p, "bob", "main", None, SendOptions::BACKGROUND, |_| {}).await.err()));
    assert!(locked(run_matrix(&client, p, &path("me.yml"), &scenario, |_| {}).await.err()));
    assert_eq!(me_calls.load(Ordering::SeqCst), 0);

    crate::secrets::unlock(p, &"password1".to_string().into()).unwrap();
    let response = send_as(&client, p, &path("me.yml"), &scenario, &Scope::default(), SendOptions::INTERACTIVE).await;
    assert_eq!(response.unwrap().status, 200);
}
