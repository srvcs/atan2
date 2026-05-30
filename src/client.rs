//! A minimal localhost HTTP/1.1 client for calling dependency services.
//!
//! srvcs composed services call other primitives over HTTP. Rather than take on
//! a full client stack for one concern, this is hand-rolled: open a connection,
//! write a `Connection: close` request, read the response to EOF, parse the
//! body. It only ever speaks to other srvcs services on localhost.

use serde_json::Value;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

/// The outcome of failing to reach a dependency.
pub enum DepError {
    Unreachable,
}

async fn request(method: &str, url: &str, body: Option<&str>) -> std::io::Result<(u16, String)> {
    let rest = url.strip_prefix("http://").unwrap_or(url);
    let (authority, path) = match rest.find('/') {
        Some(i) => (&rest[..i], &rest[i..]),
        None => (rest, "/"),
    };

    let mut stream = TcpStream::connect(authority).await?;
    let body = body.unwrap_or("");
    let req = format!(
        "{method} {path} HTTP/1.1\r\n\
         Host: {authority}\r\n\
         Content-Type: application/json\r\n\
         Content-Length: {len}\r\n\
         Connection: close\r\n\
         \r\n\
         {body}",
        len = body.len(),
    );
    stream.write_all(req.as_bytes()).await?;
    stream.flush().await?;

    let mut raw = Vec::new();
    stream.read_to_end(&mut raw).await?;
    let text = String::from_utf8_lossy(&raw).into_owned();

    let (head, body) = text.split_once("\r\n\r\n").unwrap_or((text.as_str(), ""));
    let status = head
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|code| code.parse().ok())
        .unwrap_or(0);

    Ok((status, body.to_string()))
}

/// POST an arbitrary JSON `body` to a dependency service's `/` endpoint and
/// return `(status, parsed body)`. Returns `DepError::Unreachable` if the
/// dependency cannot be reached, which the caller surfaces as a degraded `503`.
pub async fn call(base_url: &str, body: &Value) -> Result<(u16, Value), DepError> {
    match request("POST", base_url, Some(&body.to_string())).await {
        Ok((status, raw)) => Ok((status, serde_json::from_str(&raw).unwrap_or(Value::Null))),
        Err(_) => Err(DepError::Unreachable),
    }
}
