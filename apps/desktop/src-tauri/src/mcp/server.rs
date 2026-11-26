//! The MCP server: Streamable HTTP at `/mcp` on loopback, behind a bearer
//! token. Browsers are turned away (any `Origin` is refused) so a web page
//! can't reach it through DNS rebinding.

use std::sync::Arc;

use axum::extract::{Request, State};
use axum::http::{StatusCode, header};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use rmcp::model::{
    CallToolRequestParams, CallToolResponse, CallToolResult, ContentBlock, ErrorData, Implementation, ListToolsResult,
    PaginatedRequestParams, ServerCapabilities, ServerConfig,
};
use rmcp::service::{MaybeSendFuture, RequestContext};
use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use rmcp::transport::streamable_http_server::{StreamableHttpServerConfig, StreamableHttpService};
use rmcp::{RoleServer, ServerHandler};
use serde_json::Value;
use tokio_util::sync::CancellationToken;

use super::tools::{self, McpApp};

const INSTRUCTIONS: &str = "cactus is an HTTP client whose requests are YAML files in a project. \
These tools work on the project open in the cactus app: list and read requests, send them as a \
critter (a persona with its own variables and login) in an environment, and run critters' flows.";

#[derive(Clone)]
struct Handler {
    app: Arc<dyn McpApp>,
    allow_edits: bool,
}

impl ServerHandler for Handler {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("cactus", env!("CARGO_PKG_VERSION")))
            .with_instructions(INSTRUCTIONS)
    }

    fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<ListToolsResult, ErrorData>> + MaybeSendFuture + '_ {
        std::future::ready(Ok(ListToolsResult::with_all_items(tools::list(self.allow_edits))))
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, ErrorData> {
        if !tools::exists(&request.name, self.allow_edits) {
            return Err(ErrorData::invalid_params(format!("Unknown tool {}", request.name), None));
        }
        let args = Value::Object(request.arguments.unwrap_or_default());
        let outcome = tokio::select! {
            outcome = tools::call(&*self.app, &request.name, args) => outcome,
            () = context.ct.cancelled() => Err("Cancelled".into()),
        };
        Ok(match outcome {
            Ok(value) => CallToolResult::structured(value),
            Err(message) => CallToolResult::error(vec![ContentBlock::text(message)]),
        }
        .into())
    }
}

/// Serve MCP on `listener` until `stop` is cancelled.
pub async fn serve(
    listener: tokio::net::TcpListener,
    app: Arc<dyn McpApp>,
    allow_edits: bool,
    token: String,
    stop: CancellationToken,
) -> std::io::Result<()> {
    let mut config = StreamableHttpServerConfig::default().enforce_origin_validation();
    config.cancellation_token = stop.child_token();
    let service = StreamableHttpService::new(
        move || Ok(Handler { app: Arc::clone(&app), allow_edits }),
        Arc::new(LocalSessionManager::default()),
        config,
    );
    let router = axum::Router::new()
        .nest_service("/mcp", service)
        .layer(middleware::from_fn_with_state(Arc::new(token), require_token));
    axum::serve(listener, router).with_graceful_shutdown(stop.cancelled_owned()).await
}

async fn require_token(State(token): State<Arc<String>>, request: Request, next: Next) -> Response {
    let given = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "));
    if given.is_some_and(|given| same(given.as_bytes(), token.as_bytes())) {
        next.run(request).await
    } else {
        StatusCode::UNAUTHORIZED.into_response()
    }
}

/// Compare in time that doesn't depend on where they differ.
fn same(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0, |diff, (x, y)| diff | (x ^ y)) == 0
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use cactus_core::reqwest::{self, StatusCode};
    use serde_json::json;

    use super::super::tools::tests::app;
    use super::*;

    const TOKEN: &str = "secret-token";

    async fn start(project: &Path, allow_edits: bool) -> (String, CancellationToken) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/mcp", listener.local_addr().unwrap());
        let stop = CancellationToken::new();
        let app: Arc<dyn McpApp> = Arc::new(app(project));
        tokio::spawn(serve(listener, app, allow_edits, TOKEN.into(), stop.clone()));
        (url, stop)
    }

    /// A JSON-RPC exchange: the HTTP status and, if any, the reply.
    struct Client {
        http: reqwest::Client,
        url: String,
        session: Option<String>,
    }

    impl Client {
        async fn post(&mut self, body: Value, token: &str, origin: Option<&str>) -> (StatusCode, Option<Value>) {
            let mut request = self
                .http
                .post(&self.url)
                .bearer_auth(token)
                .header("accept", "application/json, text/event-stream")
                .header("mcp-protocol-version", "2025-06-18")
                .header("content-type", "application/json")
                .body(body.to_string());
            if let Some(session) = &self.session {
                request = request.header("mcp-session-id", session);
            }
            if let Some(origin) = origin {
                request = request.header("origin", origin);
            }
            let response = request.send().await.unwrap();
            let status = response.status();
            if let Some(session) = response.headers().get("mcp-session-id") {
                self.session = Some(session.to_str().unwrap().to_string());
            }
            let text = response.text().await.unwrap();
            // A JSON body, or an SSE stream whose `data:` lines carry the reply.
            let reply = serde_json::from_str(&text).ok().or_else(|| {
                text.lines()
                    .filter_map(|line| line.strip_prefix("data:"))
                    .filter_map(|data| serde_json::from_str::<Value>(data.trim()).ok())
                    .find(|message| message.get("id").is_some())
            });
            (status, reply)
        }

        async fn connect(url: &str) -> Self {
            let mut client = Self { http: reqwest::Client::new(), url: url.into(), session: None };
            let init = json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {
                "protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": { "name": "test", "version": "1" }
            }});
            let (status, reply) = client.post(init, TOKEN, None).await;
            assert_eq!(status, StatusCode::OK);
            assert_eq!(reply.unwrap()["result"]["serverInfo"]["name"], "cactus");
            let initialized = json!({ "jsonrpc": "2.0", "method": "notifications/initialized" });
            assert!(client.post(initialized, TOKEN, None).await.0.is_success());
            client
        }

        async fn call(&mut self, id: u32, method: &str, params: Value) -> Value {
            let body = json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params });
            let (status, reply) = self.post(body, TOKEN, None).await;
            assert_eq!(status, StatusCode::OK);
            reply.unwrap()
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn serves_tools_to_clients_with_the_token_only() {
        let dir = tempfile::tempdir().unwrap();
        cactus_core::scenario::create_request(dir.path(), "", "Ping", "GET", "http://localhost/ping").unwrap();
        let (url, stop) = start(dir.path(), false).await;

        let mut client = Client::connect(&url).await;
        let tools = client.call(2, "tools/list", json!({})).await;
        let names: Vec<&str> =
            tools["result"]["tools"].as_array().unwrap().iter().map(|tool| tool["name"].as_str().unwrap()).collect();
        assert!(names.contains(&"send_request") && !names.contains(&"save_request"));

        let listed = client.call(3, "tools/call", json!({ "name": "list_requests", "arguments": {} })).await;
        assert_eq!(listed["result"]["isError"], false, "{listed}");
        assert_eq!(listed["result"]["structuredContent"]["requests"][0]["title"], "Ping", "{listed}");

        let failed =
            client.call(4, "tools/call", json!({ "name": "get_request", "arguments": { "path": "nope.yml" } })).await;
        assert_eq!(failed["result"]["isError"], true);

        let edit = json!({ "name": "save_flow", "arguments": { "critter": "a", "flow": "b", "yaml": "" } });
        assert!(client.call(5, "tools/call", edit).await.get("error").is_some());

        let ping = json!({ "jsonrpc": "2.0", "id": 6, "method": "ping" });
        assert_eq!(client.post(ping.clone(), "wrong", None).await.0, StatusCode::UNAUTHORIZED);
        assert_eq!(client.post(ping, TOKEN, Some("http://evil.example")).await.0, StatusCode::FORBIDDEN);
        stop.cancel();
    }

    #[test]
    fn tokens_compare_whole() {
        assert!(same(b"abc", b"abc"));
        assert!(!same(b"abc", b"abd") && !same(b"abc", b"ab") && !same(b"", b"a"));
    }
}
