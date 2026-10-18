//! The static site `craber build` writes.
//!
//! A published diagram is the web bundle plus one `schema.json`: the bundle is
//! the same for every schema and is embedded in the binary at compile time (see
//! `build.rs`), while `schema.json` is produced per run. Nothing here knows
//! about SQL, and nothing in the frontend knows about this crate — the contract
//! between them is the file layout below and `craber_core::SCHEMA_VERSION`.
//!
//! ```text
//! dist/
//!   index.html
//!   assets/index-<hash>.js
//!   assets/index-<hash>.css
//!   schema.json              # IR + table coordinates + edge routes
//!   serve.json               # cache headers for static hosts that read it
//! ```

use std::borrow::Cow;
use std::path::{Path, PathBuf};

use craber_core::Schema;

include!(concat!(env!("OUT_DIR"), "/bundle.rs"));

/// What the last build into this directory wrote.
///
/// Asset filenames carry a content hash, so a rebuilt bundle lands beside the
/// old one instead of replacing it. Without a record of what was written, a
/// directory built into repeatedly would accumulate every bundle it had ever
/// held — so the build leaves a list, and removes what it no longer writes.
/// Only files this list names are ever deleted: anything else in the directory
/// belongs to somebody else.
const MANIFEST: &str = ".craber-build.json";

/// Cache policy for the hosts that read this file (`serve`, Vercel and the
/// like). Asset filenames carry a content hash and never change meaning, so
/// they are immutable; `index.html` and `schema.json` keep their names across
/// deploys and must not be served from cache, or a redeployed diagram shows the
/// previous schema.
const SERVE_JSON: &str = r#"{
  "headers": [
    {
      "source": "**/*.@(html|json)",
      "headers": [{ "key": "Cache-Control", "value": "no-cache" }]
    },
    {
      "source": "assets/**",
      "headers": [{ "key": "Cache-Control", "value": "public, max-age=31536000, immutable" }]
    }
  ]
}
"#;

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

    #[error("serializing schema.json")]
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
    /// Path relative to the site root, and so also its URL. Always `/`-joined.
    pub path: PathBuf,
    pub contents: Cow<'static, [u8]>,
}

/// The servable site: the web bundle, with `index.html` adapted to this schema,
/// plus the `schema.json` it draws.
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
pub fn files(schema: &Schema, base_path: Option<&str>) -> Result<Vec<SiteFile>, Error> {
    if !is_embedded() {
        return Err(Error::MissingBundle);
    }

    let mut files = Vec::with_capacity(FILES.len() + 1);
    for (name, bytes) in FILES {
        let contents = match (*name, std::str::from_utf8(bytes)) {
            ("index.html", Ok(html)) => {
                Cow::Owned(index_html(html, &schema.name, base_path).into_bytes())
            }
            _ => Cow::Borrowed(*bytes),
        };
        files.push(SiteFile {
            path: PathBuf::from(name),
            contents,
        });
    }

    // Compact rather than pretty: a 200-table schema is megabytes of JSON that
    // every visitor downloads, and nobody reads this copy by hand — `craber
    // export --format json` is there for that.
    files.push(SiteFile {
        path: PathBuf::from("schema.json"),
        contents: Cow::Owned(serde_json::to_vec(schema)?),
    });

    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(files)
}

/// Writes the deployable site into `out_dir`, creating it if needed.
///
/// This is [`files`] persisted, plus the two things only a directory needs:
/// `serve.json` so a static host knows what to cache, and `.craber-build.json`
/// so the next build can clear this one's bundle and nothing else.
///
/// Returns the files written, relative to `out_dir`, in a stable order.
pub fn write(
    schema: &Schema,
    out_dir: &Path,
    base_path: Option<&str>,
) -> Result<Vec<PathBuf>, Error> {
    let mut written = Vec::with_capacity(FILES.len() + 2);
    for file in files(schema, base_path)? {
        written.push(write_file(out_dir, &file.path, &file.contents)?);
    }
    written.push(write_file(
        out_dir,
        Path::new("serve.json"),
        SERVE_JSON.as_bytes(),
    )?);

    written.sort();

    prune(out_dir, &written)?;
    let manifest: Vec<String> = written
        .iter()
        .map(|path| path.to_string_lossy().replace('\\', "/"))
        .collect();
    let manifest = serde_json::to_vec_pretty(&manifest)?;
    write_file(out_dir, Path::new(MANIFEST), &manifest)?;

    Ok(written)
}

/// Removes what the previous build into this directory wrote and this one did
/// not — the stale hashed assets of an older bundle.
fn prune(out_dir: &Path, written: &[PathBuf]) -> Result<(), Error> {
    let Ok(manifest) = std::fs::read(out_dir.join(MANIFEST)) else {
        return Ok(());
    };
    let Ok(previous) = serde_json::from_slice::<Vec<String>>(&manifest) else {
        // An unreadable manifest says nothing about what is safe to delete.
        return Ok(());
    };

    let kept: std::collections::BTreeSet<&Path> = written.iter().map(PathBuf::as_path).collect();
    for name in previous {
        let relative = Path::new(&name);
        if kept.contains(relative) || name.contains("..") || relative.is_absolute() {
            continue;
        }
        let path = out_dir.join(relative);
        if path.is_file() {
            std::fs::remove_file(&path).map_err(|source| Error::Write { path, source })?;
        }
    }
    Ok(())
}

fn write_file(out_dir: &Path, name: &Path, contents: &[u8]) -> Result<PathBuf, Error> {
    let path = out_dir.join(name);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|source| Error::Write {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    std::fs::write(&path, contents).map_err(|source| Error::Write {
        path: path.clone(),
        source,
    })?;
    Ok(name.to_path_buf())
}

/// Adapts the bundle's `index.html` to the schema being published.
///
/// The browser tab and any bookmark of it are all a reader has to tell two
/// published diagrams apart, so the schema name goes in the title. The bundle
/// itself cannot do this: it is built once and serves every schema.
fn index_html(html: &str, schema_name: &str, base_path: Option<&str>) -> String {
    let titled = retitle(html, schema_name);
    match base_path {
        Some(base) => with_base_path(&titled, base),
        None => titled,
    }
}

/// Replaces whatever is between `<title>` and `</title>`.
///
/// This used to match the literal `<title>CrabER</title>`, which meant editing
/// `web/index.html` silently turned the feature off — no error, just every
/// published diagram sharing one tab name. Scanning for the tags survives any
/// title the frontend cares to set.
fn retitle(html: &str, schema_name: &str) -> String {
    let Some(open) = html.find("<title>") else {
        return html.to_owned();
    };
    let start = open + "<title>".len();
    let Some(length) = html[start..].find("</title>") else {
        return html.to_owned();
    };
    let mut out = String::with_capacity(html.len() + schema_name.len());
    out.push_str(&html[..start]);
    out.push_str(schema_name);
    out.push_str(" — CrabER");
    out.push_str(&html[start + length..]);
    out
}

/// Points the bundle's relative asset references at an absolute prefix.
///
/// Only asset URLs move. `schema.json` is fetched relative to the document, so
/// it follows `index.html` wherever it is served from without being rewritten.
fn with_base_path(html: &str, base_path: &str) -> String {
    let trimmed = base_path.trim().trim_matches('/');
    let base = if trimmed.is_empty() {
        "/".to_owned()
    } else {
        format!("/{trimmed}/")
    };
    // Every root-relative reference the bundle emits, not just the hashed
    // assets: the favicon is copied to the site root as-is.
    html.replace("./assets/", &format!("{base}assets/"))
        .replace("./favicon.svg", &format!("{base}favicon.svg"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const INDEX: &str = r#"<title>CrabER</title>
<link rel="icon" href="./favicon.svg" />
<script type="module" crossorigin src="./assets/index-abc.js"></script>
<link rel="stylesheet" crossorigin href="./assets/index-abc.css">"#;

    #[test]
    fn base_path_is_normalized_to_one_leading_and_trailing_slash() {
        for base in ["/myrepo/", "myrepo", "/myrepo", "  myrepo/  "] {
            let html = with_base_path(INDEX, base);
            assert!(
                html.contains(r#"src="/myrepo/assets/index-abc.js""#),
                "base {base:?} produced {html}"
            );
            assert!(html.contains(r#"href="/myrepo/assets/index-abc.css""#));
        }
    }

    #[test]
    fn the_favicon_follows_the_base_path() {
        assert!(with_base_path(INDEX, "myrepo").contains(r#"href="/myrepo/favicon.svg""#));
    }

    #[test]
    fn the_title_names_the_schema_so_two_published_diagrams_differ() {
        let html = index_html(INDEX, "billing", None);
        assert!(html.contains("<title>billing — CrabER</title>"), "{html}");
        assert!(
            html.contains(r#"src="./assets/index-abc.js""#),
            "assets stay relative"
        );
    }

    /// The title rewrite must not depend on `web/index.html` saying exactly
    /// "CrabER" — that coupling used to break the feature silently.
    #[test]
    fn the_title_is_rewritten_whatever_it_said_before() {
        for before in [
            "<title>CrabER</title>",
            "<title>Something Else</title>",
            "<title></title>",
            "<title>CrabER — diagrams</title>",
        ] {
            let html = index_html(before, "billing", None);
            assert!(
                html.contains("<title>billing — CrabER</title>"),
                "{before} produced {html}"
            );
        }
    }

    /// An `index.html` with no title at all is passed through rather than
    /// mangled: better a missing tab name than a corrupted document.
    #[test]
    fn html_without_a_title_is_left_alone() {
        let html = index_html("<p>no title here</p>", "billing", None);
        assert_eq!(html, "<p>no title here</p>");
    }

    /// `files()` is the site; the directory-only extras belong to `write()`.
    #[test]
    fn files_are_the_servable_site_and_nothing_about_a_directory() {
        if !is_embedded() {
            return;
        }
        let site = files(&Schema::new("shop"), None).expect("a site");
        let names: Vec<String> = site
            .iter()
            .map(|f| f.path.to_string_lossy().into_owned())
            .collect();

        assert!(names.contains(&"index.html".to_owned()), "{names:?}");
        assert!(names.contains(&"schema.json".to_owned()), "{names:?}");
        assert!(
            !names.iter().any(|n| n == "serve.json" || n == MANIFEST),
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
        let site = files(&Schema::new("shop"), None).expect("a site");
        let js = site
            .iter()
            .find(|f| f.path.extension().is_some_and(|e| e == "js"))
            .expect("the bundle has a script");
        assert!(
            matches!(js.contents, Cow::Borrowed(_)),
            "the compiled-in script should be borrowed, not cloned"
        );
    }

    #[test]
    fn an_empty_base_path_serves_from_the_root() {
        assert!(with_base_path(INDEX, "/").contains(r#"src="/assets/index-abc.js""#));
    }
}
