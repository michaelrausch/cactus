//! One request over the wire: build it, send it, read the whole body, then
//! evaluate actions and assertions. No side effects; those live in `runner`.

use std::path::Path;
use std::time::Instant;

use digest_auth::{AuthContext, HttpMethod};
use reqwest::header::{AUTHORIZATION, HeaderMap, HeaderValue, WWW_AUTHENTICATE};
use reqwest::{StatusCode, Url};

use super::assertions::evaluate;
use super::request::{BearerToken, DigestCredentials, build_request};
use super::response::{ReceivedResponse, Response, SentRequest};
use crate::Result;
use crate::collection::RequestDefaults;
use crate::env::Env;
use crate::scenario::Scenario;

/// Send `scenario` with what it inherits and `env` applied, then run its
/// actions and assertions. `token` is what OAuth2 auth sends. Nothing is
/// persisted; see [`crate::runner::send_and_record`].
///
/// With digest auth the request goes out without credentials first; a 401
/// with a `Digest` challenge is answered by sending it once more.
pub async fn send(
    client: &reqwest::Client,
    project: &Path,
    scenario: &Scenario,
    inherited: &[RequestDefaults],
    env: &Env,
    token: Option<BearerToken<'_>>,
) -> Result<Response> {
    let prepared = build_request(client, project, scenario, inherited, env, token)?;
    let mut sent = SentRequest {
        method: prepared.request.method().to_string(),
        url: prepared.request.url().to_string(),
        unresolved: prepared.unresolved,
        warnings: prepared.warnings,
    };
    let started = Instant::now();
    let mut reply = client.execute(prepared.request).await?;
    if let Some(credentials) = prepared.digest
        && reply.status() == StatusCode::UNAUTHORIZED
        // After a redirect the challenge is another server's: don't answer it here.
        && reply.url().as_str() == sent.url
    {
        match digest_challenge(reply.headers()) {
            Some(challenge) => {
                // A multipart body can't be cloned, so the request is built again.
                let mut again = build_request(client, project, scenario, inherited, env, token)?.request;
                match digest_authorization(&challenge, &credentials, &again) {
                    Ok(answer) => {
                        again.headers_mut().insert(AUTHORIZATION, answer);
                        reply = client.execute(again).await?;
                    }
                    Err(reason) => sent.warnings.push(format!("couldn't answer the digest challenge: {reason}")),
                }
            }
            None => sent.warnings.push("got 401 without a Digest challenge".into()),
        }
    }
    let status = reply.status();
    let headers = reply.headers().clone();
    let body = reply.bytes().await?;
    let received = ReceivedResponse { status, headers: &headers, body: &body, elapsed: started.elapsed() };

    let mut response = Response::new(sent, &received);
    evaluate(scenario, &mut response, &body);
    Ok(response)
}

/// The first `WWW-Authenticate` challenge of the Digest scheme.
fn digest_challenge(headers: &HeaderMap) -> Option<String> {
    headers
        .get_all(WWW_AUTHENTICATE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .map(str::trim)
        .find(|value| value.get(..7).is_some_and(|scheme| scheme.eq_ignore_ascii_case("digest ")))
        .map(str::to_string)
}

fn digest_authorization(
    challenge: &str,
    credentials: &DigestCredentials,
    request: &reqwest::Request,
) -> std::result::Result<HeaderValue, String> {
    let mut prompt = digest_auth::parse(challenge).map_err(|e| e.to_string())?;
    let body = request.body().and_then(reqwest::Body::as_bytes);
    let context = AuthContext::new_with_method(
        credentials.username.as_str(),
        credentials.password.as_str(),
        request_uri(request.url()),
        body,
        HttpMethod::from(request.method().as_str()),
    );
    let answer = prompt.respond(&context).map_err(|e| e.to_string())?;
    HeaderValue::from_str(&answer.to_header_string()).map_err(|e| e.to_string())
}

/// The digest `uri`: the path and query, as in the request line.
fn request_uri(url: &Url) -> String {
    match url.query() {
        Some(query) => format!("{}?{query}", url.path()),
        None => url.path().to_string(),
    }
}
