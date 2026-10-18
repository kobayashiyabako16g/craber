//! Content types for the files a CrabER site is made of.

/// The five types a CrabER site is made of.
///
/// `craber build` emits nothing else, so anything unknown is a file the bundle
/// gained without this list being updated — serving it as a download is the
/// safe wrong answer rather than guessing.
pub(super) fn content_type(path: &str) -> &'static str {
    match path.rsplit('.').next() {
        Some("html") => "text/html; charset=utf-8",
        Some("js") => "text/javascript; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("svg") => "image/svg+xml",
        Some("json") => "application/json",
        _ => "application/octet-stream",
    }
}
