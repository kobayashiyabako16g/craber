//! The static site `craber build` writes.
//!
//! A published diagram is the web bundle plus `schema.json` and `flows.json`:
//! the bundle is the same for every schema and is embedded in the binary at
//! compile time (see `build.rs`), while the two JSON files are produced per run. Nothing here knows
//! about SQL, and nothing in the frontend knows about this crate — the contract
//! between them is the file layout below, `craber_core::SCHEMA_VERSION` and
//! `craber_core::FLOWS_VERSION`.
//!
//! ```text
//! dist/
//!   index.html
//!   assets/index-<hash>.js
//!   assets/index-<hash>.css
//!   schema.json              # IR + table coordinates
//!   flows.json               # workflows from --flow; empty when none were given
//!   serve.json               # cache headers for static hosts that read it
//! ```

mod dir;
mod html;

use std::borrow::Cow;
use std::path::PathBuf;

use craber_core::{FLOWS_FILE, Flows, SCHEMA_FILE, Schema};

pub use crate::dir::write;
use crate::html::index_html;

include!(concat!(env!("OUT_DIR"), "/bundle.rs"));

/// The page, which `craber serve` also answers `/` with.
pub const INDEX_FILE: &str = "index.html";

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(
        "this craber binary has no web bundle embedded: run `just web-build` to produce web/dist, \
         then rebuild craber (or set CRABER_WEB_DIST to an existing bundle)"
    )]
    MissingBundle,

    #[error("writing {path}")]
    Write {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("serializing the site's JSON")]
    Serialize(#[from] serde_json::Error),
}

/// Whether this binary can write a site at all.
///
/// False in a workspace where the frontend has never been built, which is the
/// normal state of a Rust-only checkout and of CI's Rust job.
pub fn is_embedded() -> bool {
    !FILES.is_empty()
}

/// Fails if this binary cannot write a site.
///
/// Callers use this to refuse before doing the work — parsing a large dump only
/// to discard it is a poor way to learn that the binary has no frontend in it.
pub fn ensure_embedded() -> Result<(), Error> {
    if is_embedded() {
        Ok(())
    } else {
        Err(Error::MissingBundle)
    }
}

/// One file of the site, and the bytes to serve or write for it.
///
/// `contents` is a [`Cow`] because most of a site is the embedded bundle, which
/// is already `&'static [u8]`. `craber serve` holds a whole site in memory and
/// answers requests from it — copying 400KB of JavaScript per request, or even
/// once per server, is pure waste. Only `index.html` (rewritten per schema) and
/// `schema.json` (produced per run) are owned.
#[derive(Debug, Clone)]
pub struct SiteFile {
    /// Path relative to the site root, and so also its URL: `/`-joined on
    /// every platform, which is why it is a string and not a `PathBuf`.
    pub path: String,
    pub contents: Cow<'static, [u8]>,
}

impl SiteFile {
    /// The `Content-Type` to serve this file with.
    ///
    /// A site is made of five kinds of file, and `craber build` emits nothing
    /// else, so anything unknown is a file the bundle gained without this list
    /// being updated — serving it as a download is the safe wrong answer rather
    /// than guessing.
    pub fn content_type(&self) -> &'static str {
        match self.path.rsplit('.').next() {
            Some("html") => "text/html; charset=utf-8",
            Some("js") => "text/javascript; charset=utf-8",
            Some("css") => "text/css; charset=utf-8",
            Some("svg") => "image/svg+xml",
            Some("json") => "application/json",
            _ => "application/octet-stream",
        }
    }
}

/// The servable site: the web bundle, with `index.html` adapted to this schema,
/// plus the `schema.json` it draws and the `flows.json` it can play back.
///
/// This is what a site *is*. Writing it to a directory adds things that are
/// facts about that directory rather than about the site — cache headers for a
/// static host, a record of what to clean up next time — so those live in
/// [`write()`] instead. `craber serve` wants exactly this list and none of that.
///
/// `base_path` is the URL prefix the site will be served under. It is optional
/// because the bundle references its assets relatively (`./assets/...`), which
/// already survives being dropped under any prefix — GitHub Pages' `/<repo>/`
/// included. Pass it only when an absolute prefix is actually wanted.
///
/// Returns the files in a stable order.
pub fn files(
    schema: &Schema,
    flows: &Flows,
    base_path: Option<&str>,
) -> Result<Vec<SiteFile>, Error> {
    if !is_embedded() {
        return Err(Error::MissingBundle);
    }

    let mut files = Vec::with_capacity(FILES.len() + 2);
    for (name, bytes) in FILES {
        let contents = match (*name, std::str::from_utf8(bytes)) {
            (INDEX_FILE, Ok(html)) => {
                Cow::Owned(index_html(html, &schema.name, base_path).into_bytes())
            }
            _ => Cow::Borrowed(*bytes),
        };
        files.push(SiteFile {
            path: (*name).to_owned(),
            contents,
        });
    }

    // Compact rather than pretty: a 200-table schema is megabytes of JSON that
    // every visitor downloads, and nobody reads this copy by hand — `craber
    // export --format json` is there for that.
    files.push(SiteFile {
        path: SCHEMA_FILE.to_owned(),
        contents: Cow::Owned(serde_json::to_vec(schema)?),
    });
    // Written even when empty, so the page can tell "no workflows" from a
    // site built before workflows existed by the version, not by a 404.
    files.push(SiteFile {
        path: FLOWS_FILE.to_owned(),
        contents: Cow::Owned(serde_json::to_vec(flows)?),
    });

    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `files()` is the site; the directory-only extras belong to `write()`.
    #[test]
    fn files_are_the_servable_site_and_nothing_about_a_directory() {
        if !is_embedded() {
            return;
        }
        let site = files(&Schema::new("shop"), &Flows::default(), None).expect("a site");
        let names: Vec<String> = site.iter().map(|f| f.path.clone()).collect();

        assert!(names.contains(&"index.html".to_owned()), "{names:?}");
        assert!(names.contains(&"schema.json".to_owned()), "{names:?}");
        assert!(names.contains(&"flows.json".to_owned()), "{names:?}");
        assert!(
            !names
                .iter()
                .any(|n| n == "serve.json" || n == ".craber-build.json"),
            "cache headers and the build manifest are facts about a directory, \
             not part of the site: {names:?}"
        );
    }

    /// The bundle must not be copied into every site: `serve` keeps one in
    /// memory and answers every request from it.
    #[test]
    fn bundle_bytes_are_borrowed_not_copied() {
        if !is_embedded() {
            return;
        }
        let site = files(&Schema::new("shop"), &Flows::default(), None).expect("a site");
        let js = site
            .iter()
            .find(|f| f.content_type().starts_with("text/javascript"))
            .expect("the bundle has a script");
        assert!(
            matches!(js.contents, Cow::Borrowed(_)),
            "the compiled-in script should be borrowed, not cloned"
        );
    }
}
