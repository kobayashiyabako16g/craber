//! One HTTP/1.1 exchange, on a socket.
//!
//! Nothing here knows what CrabER is. It reads a request line, asks [`Routes`]
//! for a path, and writes a response — which is the part of `serve` that is
//! about HTTP rather than about diagrams, and the part most likely to be read
//! by someone checking a header rather than a feature.
//!
//! Keep-alive is not implemented, so every response says `Connection: close`
//! and the socket is dropped afterwards. That is a complete and correct
//! HTTP/1.1 exchange — the header is exactly how a server declines to persist
//! the connection.

use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::time::Duration;

use crate::server::mime::content_type;
use crate::server::routes::Routes;

/// How long to wait for the client's own bytes while draining.
const DRAIN_TIMEOUT: Duration = Duration::from_millis(50);

/// Most of a request CrabER will ever see. A browser asking for five files does
/// not send more than this, and the cap is what stops a drain from reading
/// forever.
const DRAIN_LIMIT: u64 = 64 * 1024;

/// Answer one request, then close.
pub(super) fn handle(stream: &TcpStream, routes: &Routes) -> std::io::Result<()> {
    let Some(request) = read_request(stream)? else {
        return Ok(());
    };

    let mut out = std::io::BufWriter::new(stream);
    let head_only = request.method == "HEAD";
    match request.method.as_str() {
        // The content type comes from the file that was found, not from the
        // URL: `/` has no extension but is `index.html`, and serving that as a
        // download gives the browser a blank page.
        "GET" | "HEAD" => match routes.get(&request.path) {
            Some(file) => respond(
                &mut out,
                "200 OK",
                content_type(&file.path.to_string_lossy()),
                &file.contents,
                head_only,
            ),
            None => respond(
                &mut out,
                "404 Not Found",
                "text/plain; charset=utf-8",
                b"not found\n",
                head_only,
            ),
        },
        _ => respond(
            &mut out,
            "405 Method Not Allowed",
            "text/plain; charset=utf-8",
            b"only GET and HEAD are served\n",
            false,
        ),
    }
}

/// What a request line amounts to here: a method and a file to look up.
struct Request {
    method: String,
    path: String,
}

/// Read the request line and discard the headers.
///
/// `Ok(None)` is a connection that closed without sending anything, which
/// browsers do routinely.
fn read_request(stream: &TcpStream) -> std::io::Result<Option<Request>> {
    let mut reader = BufReader::new(stream);
    let mut request_line = String::new();
    if reader.read_line(&mut request_line)? == 0 {
        return Ok(None);
    }

    // Read past the headers so the client is not writing into a closed socket
    // while we reply. The body is never read: only GET and HEAD are answered,
    // and neither carries one.
    let mut header = String::new();
    while reader.read_line(&mut header)? > 0 {
        if header == "\r\n" || header == "\n" {
            break;
        }
        header.clear();
    }

    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or_default().to_owned();
    let target = parts.next().unwrap_or("/");

    // `?table=...&at=...` is the frontend's own view state; it never reaches a
    // file name. Fragments never arrive at a server at all, but trim defensively.
    let path = target
        .split(['?', '#'])
        .next()
        .filter(|path| !path.is_empty())
        .unwrap_or("/")
        .to_owned();

    Ok(Some(Request { method, path }))
}

fn respond(
    out: &mut impl Write,
    status: &str,
    content_type: &str,
    body: &[u8],
    headers_only: bool,
) -> std::io::Result<()> {
    write!(
        out,
        "HTTP/1.1 {status}\r\n\
         Content-Type: {content_type}\r\n\
         Content-Length: {}\r\n\
         Cache-Control: no-store\r\n\
         Connection: close\r\n\r\n",
        body.len()
    )?;
    if !headers_only {
        out.write_all(body)?;
    }
    out.flush()
}

/// Shut down writes, then briefly drain whatever the client still had in
/// flight.
///
/// Closing on unread bytes can send an RST that truncates the response being
/// read at the other end. The short timeout is the point: a client is not
/// obliged to close its own write half, so draining to EOF would hold this
/// thread for the full read timeout on every single request. A few
/// milliseconds is enough to clear a request that is already on the wire,
/// which is all this is for.
pub(super) fn close(stream: &TcpStream) {
    let _ = stream.shutdown(std::net::Shutdown::Write);
    let _ = stream.set_read_timeout(Some(DRAIN_TIMEOUT));
    let mut drain = std::io::Read::take(stream, DRAIN_LIMIT);
    let _ = std::io::copy(&mut drain, &mut std::io::sink());
}
