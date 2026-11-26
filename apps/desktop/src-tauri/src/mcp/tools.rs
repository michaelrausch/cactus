//! The MCP tools: thin adapters over `cactus_core`, like `commands/*`.
//! Each takes its arguments as JSON and gives back JSON; core errors become
//! tool errors the agent can read. Arguments are owned because they are
//! deserialised per call.
#![allow(clippy::needless_pass_by_value)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use cactus_core::env::{self, Env, Scope, VarSource};
use cactus_core::http::{Response, scenario_passed};
use cactus_core::runner::{self, SendOptions};
use cactus_core::scenario::{self, Scenario};
use cactus_core::tools::{Answer, Ask, BoxFuture, CHOICE, ToolHost};
use cactus_core::{Error, ProjectPath, critter, history};
use rmcp::handler::server::common::schema_for_type;
use rmcp::model::{JsonObject, Tool, ToolAnnotations};
use schemars::JsonSchema;
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

/// Response bodies longer than this are cut short, so one big response
/// doesn't fill the agent's context.
const MAX_BODY_BYTES: usize = 64 * 1024;
const DEFAULT_HISTORY: usize = 5;

/// What the tools need from the app around them.
pub trait McpApp: Send + Sync + 'static {
    /// The project open in the app.
    fn project(&self) -> Option<PathBuf>;
    fn client(&self) -> cactus_core::reqwest::Client;
    /// Who a flow's tool steps ask: the person at the app.
    fn host(&self, critter: &str, flow: &str) -> Box<dyn ToolHost>;
    fn changed(&self, change: Change);
}

/// Something a tool did that the app should know about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Change {
    /// A critter's context may have changed.
    Context(String),
    /// A request's history has a new entry.
    History(ProjectPath),
    /// A flow file was written; its trigger may have changed.
    Flows,
}

/// A tool's result: JSON for the agent, or an error message.
pub type Outcome = Result<Value, String>;

struct Def {
    name: &'static str,
    description: &'static str,
    edits: bool,
    read_only: bool,
    schema: fn() -> Arc<JsonObject>,
}

const DEFS: &[Def] = &[
    Def {
        name: "list_requests",
        description: "List the project's requests in tree order: path, title, method and tags.",
        edits: false,
        read_only: true,
        schema: schema_for_type::<NoParams>,
    },
    Def {
        name: "get_request",
        description: "Read a request file as JSON (OpenCollection request).",
        edits: false,
        read_only: true,
        schema: schema_for_type::<PathParams>,
    },
    Def {
        name: "list_environments",
        description: "List the named environments and which one is active.",
        edits: false,
        read_only: true,
        schema: schema_for_type::<NoParams>,
    },
    Def {
        name: "list_critters",
        description: "List the critters (personas a request can be sent as), their flows, and which one is active.",
        edits: false,
        read_only: true,
        schema: schema_for_type::<NoParams>,
    },
    Def {
        name: "get_flow",
        description: "Read a critter's flow as YAML.",
        edits: false,
        read_only: true,
        schema: schema_for_type::<FlowParams>,
    },
    Def {
        name: "get_variables",
        description: "The variables a request sees and where each comes from. Secret values are hidden.",
        edits: false,
        read_only: true,
        schema: schema_for_type::<VariablesParams>,
    },
    Def {
        name: "get_history",
        description: "A request's last responses, newest first.",
        edits: false,
        read_only: true,
        schema: schema_for_type::<HistoryParams>,
    },
    Def {
        name: "send_request",
        description: "Send a saved request and return the response and its checks. Captured values are saved, as when sent from the app.",
        edits: false,
        read_only: false,
        schema: schema_for_type::<SendParams>,
    },
    Def {
        name: "run_folder",
        description: "Send every request in a folder (the whole project when empty) in order.",
        edits: false,
        read_only: false,
        schema: schema_for_type::<RunFolderParams>,
    },
    Def {
        name: "run_flow",
        description: "Run a critter's flow. Prompts in the flow are shown to the person at the app unless `vars` answers them.",
        edits: false,
        read_only: false,
        schema: schema_for_type::<RunFlowParams>,
    },
    Def {
        name: "create_request",
        description: "Create a request file. Returns its path.",
        edits: true,
        read_only: false,
        schema: schema_for_type::<CreateRequestParams>,
    },
    Def {
        name: "save_request",
        description: "Save a request, given as the JSON `get_request` returns. Only changed fields are rewritten.",
        edits: true,
        read_only: false,
        schema: schema_for_type::<SaveRequestParams>,
    },
    Def {
        name: "save_flow",
        description: "Write a critter's flow as YAML, creating it if needed. YAML that isn't a valid flow is refused.",
        edits: true,
        read_only: false,
        schema: schema_for_type::<SaveFlowParams>,
    },
];

/// The tools offered; edits only when allowed.
pub fn list(allow_edits: bool) -> Vec<Tool> {
    DEFS.iter()
        .filter(|def| allow_edits || !def.edits)
        .map(|def| {
            let mut annotations = ToolAnnotations::new();
            annotations.read_only_hint = Some(def.read_only);
            annotations.destructive_hint = Some(false);
            Tool::new(def.name, def.description, (def.schema)()).with_annotations(annotations)
        })
        .collect()
}

/// Whether `name` is a tool offered with `allow_edits`.
pub fn exists(name: &str, allow_edits: bool) -> bool {
    DEFS.iter().any(|def| def.name == name && (allow_edits || !def.edits))
}

/// Run tool `name` with `args`. The caller has checked it [`exists`].
pub async fn call(app: &dyn McpApp, name: &str, args: Value) -> Outcome {
    let project = app.project().ok_or("No project open in cactus")?;
    let project = project.as_path();
    match name {
        "list_requests" => list_requests(project),
        "get_request" => get_request(project, parse(args)?),
        "list_environments" => list_environments(project),
        "list_critters" => Ok(list_critters(project)),
        "get_flow" => get_flow(project, parse(args)?),
        "get_variables" => get_variables(project, parse(args)?),
        "get_history" => get_history(project, parse(args)?),
        "send_request" => send_request(app, project, parse(args)?).await,
        "run_folder" => run_folder(app, project, parse(args)?).await,
        "run_flow" => run_flow(app, project, parse(args)?).await,
        "create_request" => create_request(project, parse(args)?),
        "save_request" => save_request(project, parse(args)?),
        "save_flow" => save_flow(app, project, parse(args)?),
        _ => Err(format!("Unknown tool {name}")),
    }
}

fn parse<T: DeserializeOwned>(args: Value) -> Result<T, String> {
    serde_json::from_value(args).map_err(|e| format!("Invalid arguments: {e}"))
}

fn fail(error: Error) -> String {
    error.to_string()
}

fn to_json(value: impl serde::Serialize) -> Outcome {
    serde_json::to_value(value).map_err(|e| e.to_string())
}

#[derive(Deserialize, JsonSchema)]
struct NoParams {}

#[derive(Deserialize, JsonSchema)]
struct PathParams {
    /// Request file relative to the project, e.g. `auth/login.yml`.
    path: String,
}

#[derive(Deserialize, JsonSchema)]
struct FlowParams {
    /// The critter's id, e.g. `bob`.
    critter: String,
    /// The flow's id, e.g. `health` for `critters/bob/flows/health.yml`.
    flow: String,
}

/// The env and critter to use.
#[derive(Deserialize, JsonSchema, Default)]
struct ScopeParams {
    /// Named environment; the active one when omitted.
    #[serde(default)]
    env: Option<String>,
    /// Critter to send as; the active one when omitted, none when empty.
    #[serde(default)]
    critter: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
struct VariablesParams {
    /// Request file relative to the project; the project root when omitted.
    #[serde(default)]
    path: Option<String>,
    #[serde(flatten)]
    scope: ScopeParams,
}

#[derive(Deserialize, JsonSchema)]
struct HistoryParams {
    /// Request file relative to the project.
    path: String,
    /// How many responses; 5 when omitted.
    #[serde(default)]
    limit: Option<usize>,
}

#[derive(Deserialize, JsonSchema)]
struct SendParams {
    /// Request file relative to the project, e.g. `auth/login.yml`.
    path: String,
    #[serde(flatten)]
    scope: ScopeParams,
}

#[derive(Deserialize, JsonSchema)]
struct RunFolderParams {
    /// Folder relative to the project; the whole project when omitted or empty.
    #[serde(default)]
    folder: String,
    #[serde(flatten)]
    scope: ScopeParams,
}

#[derive(Deserialize, JsonSchema)]
struct RunFlowParams {
    /// The critter's id.
    critter: String,
    /// The flow's id.
    flow: String,
    /// Named environment; the active one when omitted.
    #[serde(default)]
    env: Option<String>,
    /// Answers for the flow's prompts, by variable name.
    #[serde(default)]
    vars: BTreeMap<String, String>,
}

#[derive(Deserialize, JsonSchema)]
struct CreateRequestParams {
    /// Folder relative to the project; the root when omitted.
    #[serde(default)]
    folder: String,
    title: String,
    /// e.g. `GET`.
    method: String,
    url: String,
}

#[derive(Deserialize, JsonSchema)]
struct SaveRequestParams {
    /// Request file relative to the project.
    path: String,
    /// The request, as `get_request` returns it.
    request: Value,
}

#[derive(Deserialize, JsonSchema)]
struct SaveFlowParams {
    critter: String,
    flow: String,
    /// The whole flow file.
    yaml: String,
}

fn request_path(path: &str) -> Result<ProjectPath, String> {
    ProjectPath::parse(path).map_err(fail)
}

/// `env` and `critter` as given, else the active ones. Names that don't
/// exist are refused rather than sending with missing variables.
fn scope(project: &Path, params: ScopeParams) -> Result<Scope, String> {
    let env = match params.env {
        Some(name) => {
            if !env::list_named(project).map_err(fail)?.contains(&name) {
                return Err(format!("No environment {name}"));
            }
            Some(name)
        }
        None => env::active(project),
    };
    let critter = match params.critter {
        Some(id) if id.is_empty() => None,
        Some(id) => {
            critter::require(project, &id).map_err(fail)?;
            Some(id)
        }
        None => critter::active(project),
    };
    Ok(Scope { env, critter, overrides: Env::new() })
}

fn list_requests(project: &Path) -> Outcome {
    Ok(json!({ "requests": to_json(scenario::list(project).map_err(fail)?)? }))
}

fn get_request(project: &Path, params: PathParams) -> Outcome {
    to_json(scenario::load(project, &request_path(&params.path)?).map_err(fail)?)
}

fn list_environments(project: &Path) -> Outcome {
    let names = env::list_named(project).map_err(fail)?;
    Ok(json!({ "active": env::active(project), "environments": names }))
}

fn list_critters(project: &Path) -> Value {
    json!({ "active": critter::active(project), "critters": critter::list(project) })
}

fn get_flow(project: &Path, params: FlowParams) -> Outcome {
    let yaml = critter::load_flow_text(project, &params.critter, &params.flow).map_err(fail)?;
    Ok(json!({ "yaml": yaml }))
}

fn get_variables(project: &Path, params: VariablesParams) -> Outcome {
    let scope = scope(project, params.scope)?;
    let (dir, request_vars) = match &params.path {
        Some(path) => {
            let path = request_path(path)?;
            let scenario = scenario::load(project, &path).map_err(fail)?;
            (path.dir().to_string(), scenario.runtime.variables)
        }
        None => (String::new(), Vec::new()),
    };
    let values = env::resolve_for_request(project, &scope, &dir, &request_vars).map_err(fail)?.vars;
    let sources = env::sources_for_request(project, &scope, &dir, &request_vars).map_err(fail)?;
    let variables: Vec<Value> = sources
        .into_iter()
        .map(|(name, source)| {
            let secret = matches!(source, VarSource::Secret | VarSource::CritterSecret);
            let value = if secret { None } else { values.get(&name).cloned() };
            json!({ "name": name, "source": source, "value": value, "secret": secret })
        })
        .collect();
    Ok(json!({ "env": scope.env, "critter": scope.critter, "variables": variables }))
}

fn get_history(project: &Path, params: HistoryParams) -> Outcome {
    let path = request_path(&params.path)?;
    let mut entries = history::load(project, &path);
    entries.truncate(params.limit.unwrap_or(DEFAULT_HISTORY));
    for entry in &mut entries {
        trim(&mut entry.response);
    }
    Ok(json!({ "history": to_json(entries)? }))
}

async fn send_request(app: &dyn McpApp, project: &Path, params: SendParams) -> Outcome {
    let path = request_path(&params.path)?;
    let scope = scope(project, params.scope)?;
    let scenario = scenario::load(project, &path).map_err(fail)?;
    let sent = runner::send_as(&app.client(), project, &path, &scenario, &scope, SendOptions::INTERACTIVE).await;
    sent_as(app, &scope);
    let mut response = sent.map_err(fail)?;
    app.changed(Change::History(path));
    let passed = scenario_passed(&scenario, &response);
    trim(&mut response);
    let mut result = to_json(response)?;
    result["passed"] = passed.into();
    Ok(result)
}

async fn run_folder(app: &dyn McpApp, project: &Path, params: RunFolderParams) -> Outcome {
    let scope = scope(project, params.scope)?;
    let ran = runner::run(&app.client(), project, params.folder.trim_matches('/'), &scope, |_| {}).await;
    sent_as(app, &scope);
    let mut results = ran.map_err(fail)?;
    for result in &mut results {
        app.changed(Change::History(result.rel_path.clone()));
        if let Some(response) = &mut result.response {
            trim(response);
        }
    }
    let passed = results.iter().filter(|result| result.ok).count();
    Ok(json!({ "passed": passed, "failed": results.len() - passed, "results": to_json(results)? }))
}

async fn run_flow(app: &dyn McpApp, project: &Path, params: RunFlowParams) -> Outcome {
    if let Some(env) = &params.env
        && !env::list_named(project).map_err(fail)?.contains(env)
    {
        return Err(format!("No environment {env}"));
    }
    let host = Answering { vars: params.vars.into_iter().collect(), host: app.host(&params.critter, &params.flow) };
    let client = app.client();
    let ran = runner::run_flow(
        &client,
        project,
        &params.critter,
        &params.flow,
        params.env,
        SendOptions::INTERACTIVE,
        &host,
        |_| {},
    )
    .await;
    app.changed(Change::Context(params.critter.clone()));
    let mut run = ran.map_err(fail)?;
    for step in &mut run.steps {
        if let Some(response) = &mut step.response {
            trim(response);
        }
    }
    to_json(run)
}

fn create_request(project: &Path, params: CreateRequestParams) -> Outcome {
    let dir = params.folder.trim_matches('/');
    let path = scenario::create_request(project, dir, &params.title, &params.method, &params.url).map_err(fail)?;
    Ok(json!({ "path": path }))
}

fn save_request(project: &Path, params: SaveRequestParams) -> Outcome {
    let path = request_path(&params.path)?;
    let request: Scenario = serde_json::from_value(params.request).map_err(|e| format!("Invalid request: {e}"))?;
    scenario::save(project, &path, &request).map_err(fail)?;
    Ok(json!({ "saved": path }))
}

fn save_flow(app: &dyn McpApp, project: &Path, params: SaveFlowParams) -> Outcome {
    critter::save_flow_text(project, &params.critter, &params.flow, &params.yaml).map_err(fail)?;
    app.changed(Change::Flows);
    Ok(json!({ "saved": format!("critters/{}/flows/{}.yml", params.critter, params.flow) }))
}

/// Sending as a critter may capture into its context, or log it in again.
fn sent_as(app: &dyn McpApp, scope: &Scope) {
    if let Some(critter) = &scope.critter {
        app.changed(Change::Context(critter.clone()));
    }
}

/// Cut a long body short at a character boundary.
fn trim(response: &mut Response) {
    if response.body.len() <= MAX_BODY_BYTES {
        return;
    }
    let mut end = MAX_BODY_BYTES;
    while !response.body.is_char_boundary(end) {
        end -= 1;
    }
    response.body.truncate(end);
    response.truncated = true;
}

/// Answers a flow's prompts from `vars` when they cover every field, and
/// its picks when `vars` names a choice's values; anything else goes to
/// the person at the app.
struct Answering {
    vars: Env,
    host: Box<dyn ToolHost>,
}

impl Answering {
    fn answer(&self, ask: &Ask) -> Option<Env> {
        match ask {
            Ask::Fields { fields, .. } if !fields.is_empty() => fields
                .iter()
                .map(|field| {
                    let name = field.name.trim();
                    Some((name.to_string(), self.vars.get(name)?.clone()))
                })
                .collect(),
            Ask::Choose { choices, .. } => {
                let index = choices.iter().position(|choice| {
                    !choice.values.is_empty()
                        && choice.values.iter().all(|(name, value)| self.vars.get(name) == Some(value))
                })?;
                Some(Env::from([(CHOICE.to_string(), index.to_string())]))
            }
            _ => None,
        }
    }
}

impl ToolHost for Answering {
    fn ask(&self, ask: Ask) -> BoxFuture<'_, Answer> {
        match self.answer(&ask) {
            Some(values) => Box::pin(async move { Answer::Given(values) }),
            None => self.host.ask(ask),
        }
    }

    fn show(&self, title: &str, text: &str) {
        self.host.show(title, text);
    }

    fn notify(&self, title: &str, body: &str) {
        self.host.notify(title, body);
    }

    fn open_url(&self, url: &str) -> cactus_core::Result<()> {
        self.host.open_url(url)
    }
}

#[cfg(test)]
pub(super) mod tests {
    use std::sync::Mutex;

    use cactus_core::tools::Headless;

    use super::*;

    /// A project in a temp dir, with what the tools reported.
    pub struct TestApp {
        pub project: Option<PathBuf>,
        pub changes: Mutex<Vec<Change>>,
    }

    impl McpApp for TestApp {
        fn project(&self) -> Option<PathBuf> {
            self.project.clone()
        }

        fn client(&self) -> cactus_core::reqwest::Client {
            cactus_core::reqwest::Client::new()
        }

        fn host(&self, _critter: &str, _flow: &str) -> Box<dyn ToolHost> {
            Box::new(Headless)
        }

        fn changed(&self, change: Change) {
            self.changes.lock().unwrap().push(change);
        }
    }

    pub fn app(project: &Path) -> TestApp {
        TestApp { project: Some(project.to_path_buf()), changes: Mutex::default() }
    }

    /// Answer every request on localhost with 200 and `body`; the base URL.
    pub async fn serve(body: String) -> String {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        tokio::spawn(async move {
            while let Ok((mut socket, _)) = listener.accept().await {
                let mut head = [0u8; 4096];
                let _ = socket.read(&mut head).await;
                let reply =
                    format!("HTTP/1.1 200 OK\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}", body.len());
                let _ = socket.write_all(reply.as_bytes()).await;
            }
        });
        base
    }

    #[test]
    fn edits_are_listed_only_when_allowed() {
        let names = |allow| list(allow).into_iter().map(|tool| tool.name.to_string()).collect::<Vec<_>>();
        assert!(!names(false).contains(&"save_request".to_string()));
        assert!(names(true).contains(&"save_request".to_string()));
        assert_eq!(names(true).len(), DEFS.len());
        assert!(!exists("save_flow", false) && exists("save_flow", true) && !exists("nope", true));
        for tool in list(true) {
            assert_eq!(tool.input_schema.get("type"), Some(&json!("object")), "{}", tool.name);
        }
    }

    #[tokio::test]
    async fn without_a_project_every_tool_says_so() {
        let app = TestApp { project: None, changes: Mutex::default() };
        assert_eq!(call(&app, "list_requests", json!({})).await.unwrap_err(), "No project open in cactus");
    }

    #[tokio::test]
    async fn sends_record_history_and_hide_secrets() {
        let server = serve("x".repeat(MAX_BODY_BYTES + 10)).await;
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path();
        let app = app(project);
        let created = call(
            &app,
            "create_request",
            json!({ "folder": "api", "title": "Ping", "method": "GET", "url": "{{base}}/ping" }),
        )
        .await
        .unwrap();
        let path = created["path"].as_str().unwrap().to_string();
        let one = |name: &str, value: &str| Env::from([(name.to_string(), value.to_string())]);
        env::save(project, &env::EnvFile::Named("dev".into()), &one("base", &server)).unwrap();
        env::save(project, &env::EnvFile::Secrets, &one("key", "hunter2")).unwrap();

        let sent = call(&app, "send_request", json!({ "path": path, "env": "dev" })).await.unwrap();
        assert_eq!(sent["status"], 200);
        assert_eq!(sent["truncated"], true);
        assert_eq!(sent["body"].as_str().unwrap().len(), MAX_BODY_BYTES);
        assert_eq!(app.changes.lock().unwrap().as_slice(), [Change::History(ProjectPath::parse(&path).unwrap())]);
        let history = call(&app, "get_history", json!({ "path": path })).await.unwrap();
        assert_eq!(history["history"].as_array().unwrap().len(), 1);

        let vars = call(&app, "get_variables", json!({ "path": path, "env": "dev" })).await.unwrap();
        let vars = vars["variables"].as_array().unwrap();
        let find = |name: &str| vars.iter().find(|var| var["name"] == name).unwrap().clone();
        assert_eq!(find("base")["value"], server.as_str());
        assert_eq!((find("key")["value"].clone(), find("key")["secret"].clone()), (Value::Null, json!(true)));
        assert!(!serde_json::to_string(vars).unwrap().contains("hunter2"));

        let missing = call(&app, "send_request", json!({ "path": path, "env": "prod" })).await.unwrap_err();
        assert_eq!(missing, "No environment prod");
        assert!(call(&app, "get_request", json!({ "path": "../outside.yml" })).await.is_err());
    }

    #[test]
    fn prompts_are_answered_from_vars_only_when_covered() {
        let field = |name: &str| serde_json::from_value(json!({ "name": name })).unwrap();
        let host = Answering { vars: Env::from([("otp".into(), "123".into())]), host: Box::new(Headless) };
        let ask = |fields| Ask::Fields { title: String::new(), fields };
        assert_eq!(host.answer(&ask(vec![field("otp")])), Some(Env::from([("otp".into(), "123".into())])));
        assert_eq!(host.answer(&ask(vec![field("otp"), field("user")])), None);
        let choices = vec![
            serde_json::from_value(json!({ "label": "a", "values": { "otp": "1" } })).unwrap(),
            serde_json::from_value(json!({ "label": "b", "values": { "otp": "123" } })).unwrap(),
        ];
        let picked = host.answer(&Ask::Choose { title: String::new(), choices });
        assert_eq!(picked, Some(Env::from([(CHOICE.into(), "1".into())])));
    }
}
