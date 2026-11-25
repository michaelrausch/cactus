//! Runner tests against a real local HTTP server (`test_server`), covering
//! captures, flows and logging in again on a 401.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use super::test_server::serve;
use super::*;
use crate::collection::{LenientString, Variable};
use crate::critter::{Critter, Flow, FlowStep, Step};
use crate::env::{Env, EnvFile};
use crate::http::build_client;
use crate::layout;
use crate::tools::{Answer, Ask, BoxFuture, Headless, MASK, ToolHost};

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
    tool_flow(steps.into_iter().map(Step::Request).collect())
}

fn tool_flow(steps: Vec<Step>) -> Flow {
    Flow { name: "F".into(), steps, ..Flow::default() }
}

/// A tool step from YAML, as it's written in a flow file.
fn tool(yaml: &str) -> Step {
    serde_yaml_ng::from_str(yaml).unwrap()
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
    let run = run_flow(&client, p, "bob", "main", None, SendOptions::BACKGROUND, &Headless, |event| match event {
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
    let run =
        run_flow(&build_client(), p, "bob", "main", None, SendOptions::BACKGROUND, &Headless, |_| {}).await.unwrap();
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
        let run = run_flow(&client, p, "bob", name, None, SendOptions::BACKGROUND, &Headless, |_| {}).await.unwrap();
        assert!(!run.ok && run.steps[0].error.is_some(), "{name}");
    }
    let missing = run_flow(&client, p, "bob", "gone", None, SendOptions::BACKGROUND, &Headless, |_| {}).await;
    assert!(matches!(missing, Err(Error::FlowNotFound { .. })));
    let nobody = run_flow(&client, p, "nobody", "x", None, SendOptions::BACKGROUND, &Headless, |_| {}).await;
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
    let run = run_flow(&client, p, "alice", "login", None, SendOptions::BACKGROUND, &Headless, |_| {}).await.unwrap();
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

/// A host that answers from a script and remembers what tools did.
#[derive(Default)]
struct Scripted {
    answers: std::sync::Mutex<Vec<Answer>>,
    asked: std::sync::Mutex<Vec<Ask>>,
    shown: std::sync::Mutex<Vec<(String, String)>>,
    notified: std::sync::Mutex<Vec<(String, String)>>,
    opened: std::sync::Mutex<Vec<String>>,
}

impl Scripted {
    fn answering(answers: Vec<Answer>) -> Self {
        Self { answers: std::sync::Mutex::new(answers), ..Self::default() }
    }
}

impl ToolHost for Scripted {
    fn ask(&self, ask: Ask) -> BoxFuture<'_, Answer> {
        self.asked.lock().unwrap().push(ask);
        let mut answers = self.answers.lock().unwrap();
        let answer = if answers.is_empty() { Answer::Unattended } else { answers.remove(0) };
        Box::pin(async move { answer })
    }

    fn show(&self, title: &str, text: &str) {
        self.shown.lock().unwrap().push((title.into(), text.into()));
    }

    fn notify(&self, title: &str, body: &str) {
        self.notified.lock().unwrap().push((title.into(), body.into()));
    }

    fn open_url(&self, url: &str) -> Result<()> {
        self.opened.lock().unwrap().push(url.into());
        Ok(())
    }
}

const COURSE: &str = "info: {name: Course}\nhttp: {method: GET, url: '{{base}}/course/{{COURSE}}'}\n";

#[tokio::test]
async fn a_prompt_gives_later_steps_its_answers_for_this_run_only() {
    let (dir, _) = project().await;
    let p = dir.path();
    fs::write(p.join("course.yml"), COURSE).unwrap();
    critter::create(p, "bob", &Critter::default()).unwrap();
    let prompt =
        tool("tool: prompt\ntitle: Course for {{base}}\nvars:\n  - {name: COURSE}\n  - {name: otp, secret: true}\n");
    let course = FlowStep { expect: Some(200), ..step("course.yml") };
    critter::create_flow(p, "bob", "main", &tool_flow(vec![prompt, Step::Request(course)])).unwrap();

    let answers = Env::from([("COURSE".into(), "fresh".into()), ("otp".into(), "123456".into())]);
    let host = Scripted::answering(vec![Answer::Given(answers)]);
    let mut started = Vec::new();
    let run = run_flow(&build_client(), p, "bob", "main", None, SendOptions::BACKGROUND, &host, |event| {
        if let FlowEvent::Started(start) = event {
            started.push((start.tool.clone(), start.title.clone()));
        }
    })
    .await
    .unwrap();
    assert!(run.ok, "{:?}", run.failure());
    assert!(run.steps[1].url.ends_with("/course/fresh"), "{}", run.steps[1].url);
    let Ask::Fields { title, fields } = &host.asked.lock().unwrap()[0] else { panic!("expected fields") };
    assert!(title.starts_with("Course for http://"), "templates are filled in: {title}");
    assert_eq!(fields.len(), 2);
    assert_eq!(started[0].0.as_deref(), Some("prompt"));
    let output = run.steps[0].output.as_ref().unwrap();
    assert_eq!((output.values["COURSE"].as_str(), output.values["otp"].as_str()), ("fresh", MASK));
    assert!(!env::load_context(p, "bob").unwrap().contains_key("otp"), "answers aren't saved");
}

#[tokio::test]
async fn an_unattended_prompt_uses_defaults_or_fails() {
    let (dir, _) = project().await;
    let p = dir.path();
    fs::write(p.join("course.yml"), COURSE).unwrap();
    critter::create(p, "bob", &Critter::default()).unwrap();
    let course = || Step::Request(FlowStep { expect: Some(200), ..step("course.yml") });
    let defaulted = tool("tool: prompt\nvars: [{name: COURSE, default: fresh}]\n");
    let required = tool("tool: prompt\nvars: [{name: COURSE}, {name: term}]\n");
    critter::create_flow(p, "bob", "defaulted", &tool_flow(vec![defaulted, course()])).unwrap();
    critter::create_flow(p, "bob", "required", &tool_flow(vec![required, course()])).unwrap();
    let client = build_client();

    let run = run_flow(&client, p, "bob", "defaulted", None, SendOptions::BACKGROUND, &Headless, |_| {}).await.unwrap();
    assert!(run.ok, "{:?}", run.failure());

    let run = run_flow(&client, p, "bob", "required", None, SendOptions::BACKGROUND, &Headless, |_| {}).await.unwrap();
    assert_eq!(run.failure().unwrap(), "step 1 (Prompt): Needs an answer for COURSE, term");
    assert!(run.steps[1].skipped);

    let stopped = Scripted::answering(vec![Answer::Stopped]);
    let run = run_flow(&client, p, "bob", "defaulted", None, SendOptions::BACKGROUND, &stopped, |_| {}).await.unwrap();
    assert_eq!((run.steps[0].reason().as_str(), run.steps[1].skipped), ("Stopped", true));
}

#[tokio::test]
async fn with_beats_what_tools_set() {
    let (dir, _) = project().await;
    let p = dir.path();
    fs::write(p.join("course.yml"), COURSE).unwrap();
    critter::create(p, "bob", &Critter::default()).unwrap();
    let set = tool("tool: set\nvalues: {COURSE: stale, picked: fresh}\n");
    let course = FlowStep {
        with: BTreeMap::from([("COURSE".into(), LenientString::from("{{picked}}"))]),
        expect: Some(200),
        ..step("course.yml")
    };
    critter::create_flow(p, "bob", "main", &tool_flow(vec![set, Step::Request(course)])).unwrap();
    let run =
        run_flow(&build_client(), p, "bob", "main", None, SendOptions::BACKGROUND, &Headless, |_| {}).await.unwrap();
    assert!(run.steps[1].url.ends_with("/course/fresh"), "{}", run.steps[1].url);
}

#[tokio::test]
async fn display_set_open_and_notify() {
    let (dir, _) = project().await;
    let p = dir.path();
    critter::create(p, "bob", &Critter::default()).unwrap();
    let steps = vec![
        tool("tool: set\nvalues: {id: $uuid, plain: 'at {{base}}'}\n"),
        tool("tool: display\ntext: 'id {{id}}'\npause: true\n"),
        tool("tool: open\nurl: '{{base}}/authorize'\n"),
        tool("tool: notify\ntitle: Done\nbody: '{{plain}}'\n"),
        tool("tool: display\ntitle: 'Order {{id}}'\ntext: placed\n"),
        tool("tool: open\nurl: 'file:///etc/passwd'\n"),
    ];
    critter::create_flow(p, "bob", "main", &tool_flow(steps)).unwrap();
    let host = Scripted::default();
    let run = run_flow(&build_client(), p, "bob", "main", None, SendOptions::BACKGROUND, &host, |_| {}).await.unwrap();

    let set = &run.steps[0].output.as_ref().unwrap().values;
    assert_eq!(set["id"].len(), 36);
    assert!(set["plain"].starts_with("at http://"));
    assert_eq!(run.steps[1].output.as_ref().unwrap().text.as_deref(), Some(format!("id {}", set["id"]).as_str()));
    let asked = host.asked.lock().unwrap();
    assert!(matches!(&asked[0], Ask::Continue { title, .. } if title == "Display"), "pause asks to continue");
    assert_eq!(asked.len(), 1, "without pause, display doesn't wait");
    assert_eq!(host.shown.lock().unwrap()[0], (format!("Order {}", set["id"]), "placed".to_string()));
    assert_eq!(run.steps[4].title, format!("Order {}", set["id"]));
    assert!(host.opened.lock().unwrap()[0].ends_with("/authorize"));
    assert_eq!(host.notified.lock().unwrap()[0], ("Done".to_string(), set["plain"].clone()));
    assert!(!run.steps[5].ok, "only web pages are opened");
    assert_eq!(host.opened.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn webhook_and_slack_post_and_fail_on_errors() {
    let received = Arc::new(std::sync::Mutex::new(Vec::new()));
    let log = Arc::clone(&received);
    let hooks = serve(move |request| {
        log.lock().unwrap().push((request.method.clone(), request.path.clone(), request.body.clone()));
        match request.path.as_str() {
            "/down" => (503, "busy".into()),
            _ => (200, "ok".into()),
        }
    })
    .await;
    let (dir, _) = project().await;
    let p = dir.path();
    env::save(p, &EnvFile::Base, &Env::from([("hooks".into(), hooks)])).unwrap();
    critter::create(p, "bob", &Critter::default()).unwrap();
    let steps = vec![
        tool("tool: webhook\nurl: '{{hooks}}/hook'\nmethod: put\nbody: '{\"ok\": true}'\n"),
        tool("tool: slack\nwebhook: '{{hooks}}/slack'\ntext: Passed\n"),
        tool("tool: webhook\nurl: '{{hooks}}/down'\n"),
    ];
    critter::create_flow(p, "bob", "main", &tool_flow(steps)).unwrap();
    let run =
        run_flow(&build_client(), p, "bob", "main", None, SendOptions::BACKGROUND, &Headless, |_| {}).await.unwrap();

    let output = run.steps[0].output.as_ref().unwrap();
    assert_eq!((output.status, output.text.as_deref()), (Some(200), Some("ok")));
    assert_eq!(run.steps[0].title, format!("PUT {}/hook", env::load(p, &EnvFile::Base).unwrap()["hooks"]));
    assert_eq!(
        run.failure().unwrap(),
        "step 3 (POST {}/down): Got 503: busy".replace("{}", &env::load(p, &EnvFile::Base).unwrap()["hooks"])
    );
    let received = received.lock().unwrap();
    assert_eq!(received[0], ("PUT".into(), "/hook".into(), r#"{"ok": true}"#.into()));
    assert_eq!(received[1], ("POST".into(), "/slack".into(), r#"{"text":"Passed"}"#.into()));
}

#[tokio::test(start_paused = true)]
async fn wait_sleeps_up_to_its_limit() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path();
    critter::create(p, "bob", &Critter::default()).unwrap();
    let steps = vec![tool("tool: wait\nseconds: 600\n"), tool("tool: wait\nseconds: 601\n")];
    critter::create_flow(p, "bob", "main", &tool_flow(steps)).unwrap();
    let started = tokio::time::Instant::now();
    let run =
        run_flow(&build_client(), p, "bob", "main", None, SendOptions::BACKGROUND, &Headless, |_| {}).await.unwrap();
    assert!(started.elapsed() >= std::time::Duration::from_secs(600));
    assert!(run.steps[0].ok && !run.steps[1].ok);
}

#[tokio::test]
async fn assert_checks_values() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path();
    critter::create(p, "bob", &Critter::default()).unwrap();
    let items = r#"'[{"id": "a", "tags": [1]}, {"id": "b", "tags": [1, 2]}, {"id": "", "tags": []}]'"#;
    let steps = vec![
        tool(&format!("tool: set\nvalues: {{count: '12', kind: x, items: {items}}}\n")),
        tool(
            "tool: assert\nchecks:\n  - {expression: '{{count}}', operator: gt, value: '10'}\n  - {expression: '{{kind}}', operator: in, value: 'x,y'}\n  - {expression: '{{items}}.length', operator: eq, value: '3'}\neach:\n  list: '{{items}}'\n  checks:\n    - {expression: tags, operator: isArray}\n",
        ),
        tool(
            "tool: assert\neach:\n  list: '{{items}}'\n  checks:\n    - {expression: id, operator: isNotEmpty}\n    - {expression: tags.length, operator: gt, value: '0'}\n",
        ),
        tool("tool: assert\nchecks:\n  - {expression: '{{count}}', operator: lt, value: '5'}\n"),
    ];
    critter::create_flow(p, "bob", "main", &tool_flow(steps)).unwrap();
    let run =
        run_flow(&build_client(), p, "bob", "main", None, SendOptions::BACKGROUND, &Headless, |_| {}).await.unwrap();
    assert!(run.steps[1].ok, "{:?}", run.steps[1].error);
    let passed = run.steps[1].output.as_ref().unwrap().text.clone().unwrap();
    assert!(passed.ends_with("{{items}}: all 3 items ✓"), "{passed}");
    assert_eq!(run.failure().unwrap(), "step 3 (Assert): {{items}} item 3: id: expected isNotEmpty, got \"\"");

    // Each step alone: the last one fails on the value, named by its expression.
    let only_last = tool_flow(vec![
        tool("tool: set\nvalues: {count: '12'}\n"),
        tool("tool: assert\nchecks:\n  - {expression: '{{count}}', operator: lt, value: '5'}\n"),
    ]);
    critter::create_flow(p, "bob", "last", &only_last).unwrap();
    let run =
        run_flow(&build_client(), p, "bob", "last", None, SendOptions::BACKGROUND, &Headless, |_| {}).await.unwrap();
    assert_eq!(run.failure().unwrap(), "step 2 (Assert): {{count}}: expected lt 5, got 12");
}

#[tokio::test]
async fn poll_resends_until_its_checks_pass_or_time_runs_out() {
    let calls = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&calls);
    let base = serve(move |_| {
        let call = counter.fetch_add(1, Ordering::SeqCst);
        let status = if call >= 2 { "ready" } else { "pending" };
        (200, format!(r#"{{"status": "{status}", "id": {call}}}"#))
    })
    .await;
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path();
    env::save(p, &EnvFile::Base, &Env::from([("base".into(), base)])).unwrap();
    fs::write(p.join("job.yml"), "info: {name: Job}\nhttp: {method: GET, url: '{{base}}/job'}\n").unwrap();
    critter::create(p, "bob", &Critter::default()).unwrap();
    let poll = "tool: poll\nrequest: job.yml\nevery: 1\nuntil: [{expression: res.body.status, operator: eq, value: ready}]\ncapture: {job_id: res.body.id}\n";
    critter::create_flow(p, "bob", "main", &tool_flow(vec![tool(poll)])).unwrap();
    critter::create_flow(p, "bob", "short", &tool_flow(vec![tool(&format!("{poll}timeout: 1\n"))])).unwrap();
    let client = build_client();

    let run = run_flow(&client, p, "bob", "main", None, SendOptions::BACKGROUND, &Headless, |_| {}).await.unwrap();
    assert!(run.ok, "{:?}", run.failure());
    assert_eq!(run.steps[0].output.as_ref().unwrap().text.as_deref(), Some("Done after 3 tries"));
    assert_eq!(env::load_context(p, "bob").unwrap()["job_id"].value, "2", "captured from the last response");

    calls.store(0, Ordering::SeqCst);
    let run = run_flow(&client, p, "bob", "short", None, SendOptions::BACKGROUND, &Headless, |_| {}).await.unwrap();
    let reason = run.steps[0].reason();
    assert!(reason.starts_with("Not done after 2 tries (1s): res.body.status: expected eq ready"), "{reason}");
}

#[tokio::test]
async fn run_runs_another_flow_as_its_critter_and_stops_loops() {
    let (dir, _) = project().await;
    let p = dir.path();
    critter::create(p, "bob", &Critter::default()).unwrap();
    critter::create(p, "alice", &Critter::default()).unwrap();
    critter::create_flow(p, "alice", "login", &flow(vec![login_step()])).unwrap();
    critter::create_flow(p, "bob", "main", &tool_flow(vec![tool("tool: run\nflow: login\ncritter: alice\n")])).unwrap();
    critter::create_flow(p, "bob", "loop", &tool_flow(vec![tool("tool: run\nflow: loop\n")])).unwrap();
    let client = build_client();

    let run = run_flow(&client, p, "bob", "main", None, SendOptions::BACKGROUND, &Headless, |_| {}).await.unwrap();
    assert!(run.ok, "{:?}", run.failure());
    assert_eq!(run.steps[0].title, "Run alice/login");
    assert_eq!(run.steps[0].output.as_ref().unwrap().text.as_deref(), Some("1/1 passed"));
    assert_eq!(env::load_context(p, "alice").unwrap()["token"].value, "fresh", "captured as alice");
    assert!(!env::load_context(p, "bob").unwrap().contains_key("token"));

    let run = run_flow(&client, p, "bob", "loop", None, SendOptions::BACKGROUND, &Headless, |_| {}).await.unwrap();
    assert!(run.failure().unwrap().contains("more than 8 deep"), "{:?}", run.failure());
}

#[tokio::test]
async fn pick_sets_the_chosen_item() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path();
    critter::create(p, "bob", &Critter::default()).unwrap();
    let items = r#"'[{"id": 7, "code": "A", "name": "First"}, {"id": 8, "code": "B", "name": "Second"}]'"#;
    let steps = vec![
        tool(&format!("tool: set\nvalues: {{items: {items}}}\n")),
        tool("tool: pick\ntitle: Pick one\nlist: '{{items}}'\nlabel: name\nset: {item_code: code, item_id: id}\n"),
    ];
    critter::create_flow(p, "bob", "main", &tool_flow(steps)).unwrap();
    let client = build_client();

    let host = Scripted::answering(vec![Answer::Given(Env::from([(crate::tools::CHOICE.into(), "1".into())]))]);
    let run = run_flow(&client, p, "bob", "main", None, SendOptions::BACKGROUND, &host, |_| {}).await.unwrap();
    let output = run.steps[1].output.as_ref().unwrap();
    assert_eq!((output.values["item_code"].as_str(), output.values["item_id"].as_str()), ("B", "8"));
    assert_eq!(output.text.as_deref(), Some("Second"), "the label of what was picked");
    {
        let asked = host.asked.lock().unwrap();
        let Ask::Choose { choices, .. } = &asked[0] else { panic!("expected a choice") };
        assert_eq!((choices[0].label.as_str(), choices[0].values["item_code"].as_str()), ("First", "A"));
    }

    let run = run_flow(&client, p, "bob", "main", None, SendOptions::BACKGROUND, &Headless, |_| {}).await.unwrap();
    assert_eq!(run.steps[1].reason(), "Needs a choice");
}
