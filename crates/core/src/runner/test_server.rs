//! A minimal HTTP/1.1 server on localhost for runner tests: one response
//! per connection, chosen by a routing function.

use std::sync::Arc;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

/// What the routing function sees of a request.
pub struct Request {
    pub method: String,
    pub path: String,
    pub body: String,
    head: String,
}

impl Request {
    /// The first header with this name, case-insensitively.
    pub fn header(&self, name: &str) -> Option<&str> {
        self.head.lines().skip(1).find_map(|line| {
            let (key, value) = line.split_once(':')?;
            key.trim().eq_ignore_ascii_case(name).then(|| value.trim())
        })
    }
}

/// A reply: status, extra headers and a JSON body.
pub type Reply = (u16, Vec<(&'static str, String)>, String);

type Route = dyn Fn(&Request) -> Reply + Send + Sync;

/// Serve `route` until the test's runtime shuts down; returns the base URL.
pub async fn serve(route: impl Fn(&Request) -> (u16, String) + Send + Sync + 'static) -> String {
    serve_with_headers(move |request| {
        let (status, body) = route(request);
        (status, Vec::new(), body)
    })
    .await
}

/// [`serve`], with headers on the replies.
pub async fn serve_with_headers(route: impl Fn(&Request) -> Reply + Send + Sync + 'static) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let route: Arc<Route> = Arc::new(route);
    tokio::spawn(async move {
        while let Ok((mut socket, _)) = listener.accept().await {
            let route = Arc::clone(&route);
            tokio::spawn(async move {
                let Some(request) = read_request(&mut socket).await else { return };
                let (status, headers, body) = route(&request);
                let headers: String = headers.iter().flat_map(|(name, value)| [name, ": ", value, "\r\n"]).collect();
                let reply = format!(
                    "HTTP/1.1 {status} X\r\ncontent-type: application/json\r\n{headers}content-length: {}\r\nconnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = socket.write_all(reply.as_bytes()).await;
                let _ = socket.shutdown().await;
            });
        }
    });
    base
}

/// The request: its head, then its body.
async fn read_request(socket: &mut tokio::net::TcpStream) -> Option<Request> {
    let mut buffer = Vec::new();
    let mut chunk = [0; 4096];
    let head_end = loop {
        let read = socket.read(&mut chunk).await.ok()?;
        if read == 0 {
            return None;
        }
        buffer.extend_from_slice(&chunk[..read]);
        if let Some(end) = buffer.windows(4).position(|w| w == b"\r\n\r\n") {
            break end + 4;
        }
    };
    let head = String::from_utf8_lossy(&buffer[..head_end]).into_owned();
    let mut request_line = head.lines().next()?.split(' ');
    let mut request = Request {
        method: request_line.next()?.to_string(),
        path: request_line.next()?.to_string(),
        body: String::new(),
        head: head.clone(),
    };
    let length: usize = request.header("content-length").and_then(|n| n.parse().ok()).unwrap_or(0);
    while buffer.len() - head_end < length {
        let read = socket.read(&mut chunk).await.ok()?;
        if read == 0 {
            break;
        }
        buffer.extend_from_slice(&chunk[..read]);
    }
    request.body = String::from_utf8_lossy(&buffer[head_end..]).into_owned();
    Some(request)
}
