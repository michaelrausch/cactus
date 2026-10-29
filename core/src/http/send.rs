use std::path::Path;
use std::time::Instant;

use super::assertions::evaluate;
use super::request::build_request;
use super::response::{ReceivedResponse, Response, SentRequest};
use crate::Result;
use crate::env::Env;
use crate::scenario::Scenario;

/// Send `scenario` with `env` applied, then run its captures and checks.
/// Nothing is persisted; see [`crate::runner::send_and_record`].
pub async fn send(client: &reqwest::Client, project: &Path, scenario: &Scenario, env: &Env) -> Result<Response> {
    let prepared = build_request(client, project, scenario, env)?;
    let sent = SentRequest {
        method: prepared.request.method().to_string(),
        url: prepared.request.url().to_string(),
        unresolved: prepared.unresolved,
    };
    let started = Instant::now();
    let reply = client.execute(prepared.request).await?;
    let status = reply.status();
    let headers = reply.headers().clone();
    let body = reply.bytes().await?;
    let received = ReceivedResponse { status, headers: &headers, body: &body, elapsed: started.elapsed() };

    let mut response = Response::new(sent, &received);
    evaluate(scenario, &mut response, &body);
    Ok(response)
}
