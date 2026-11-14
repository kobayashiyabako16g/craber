//! The site written to a directory, for a static host to serve.

use std::path::{Component, Path, PathBuf};

use craber_core::{Flows, Schema};

use crate::{Error, FILES, files};

/// What the last build into this directory wrote.
///
/// Asset filenames carry a content hash, so a rebuilt bundle lands beside the
/// old one instead of replacing it. Without a record of what was written, a
/// directory built into repeatedly would accumulate every bundle it had ever
/// held — so the build leaves a list, and removes what it no longer writes.
/// Only files this list names are ever deleted: anything else in the directory
/// belongs to somebody else.
const MANIFEST: &str = ".craber-build.json";

/// Cache headers for a static host; only a written site has them.
const SERVE_FILE: &str = "serve.json";

/// Cache policy for the hosts that read this file (`serve`, Vercel and the
/// like). Asset filenames carry a content hash and never change meaning, so
/// they are immutable; `index.html`, `schema.json` and the Markdown beside it
/// keep their names across deploys and must not be served from cache, or a
/// redeployed diagram shows the previous schema.
const SERVE_JSON: &str = include_str!("serve.json");

/// Writes the deployable site into `out_dir`, creating it if needed.
///
/// This is [`files`] persisted, plus the two things only a directory needs:
/// `serve.json` so a static host knows what to cache, and `.craber-build.json`
/// so the next build can clear this one's bundle and nothing else.
///
/// Returns the files written, relative to `out_dir`, in a stable order.
pub fn write(
    schema: &Schema,
    flows: &Flows,
    out_dir: &Path,
    base_path: Option<&str>,
) -> Result<Vec<PathBuf>, Error> {
    let mut written = Vec::with_capacity(FILES.len() + 2);
    for file in files(schema, flows, base_path)? {
        written.push(write_file(out_dir, Path::new(&file.path), &file.contents)?);
    }
    written.push(write_file(
        out_dir,
        Path::new(SERVE_FILE),
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
        // Only plain names below `out_dir`: a manifest is a file anyone could
        // have edited, and `..` or a root would delete outside the site.
        let inside = relative
            .components()
            .all(|component| matches!(component, Component::Normal(_)));
        if kept.contains(relative) || !inside {
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A dotted filename is a filename; only a `..` *component* leaves the site.
    #[test]
    fn prune_removes_dotted_names_but_never_leaves_the_directory() {
        let root = std::env::temp_dir().join(format!("craber-prune-{}", std::process::id()));
        let out = root.join("site");
        std::fs::create_dir_all(&out).unwrap();
        std::fs::write(out.join("a..b.js"), b"").unwrap();
        std::fs::write(root.join("outside.txt"), b"").unwrap();
        std::fs::write(
            out.join(MANIFEST),
            serde_json::to_vec(&["a..b.js", "../outside.txt"]).unwrap(),
        )
        .unwrap();

        prune(&out, &[]).unwrap();
        assert!(!out.join("a..b.js").exists());
        assert!(root.join("outside.txt").exists());
        std::fs::remove_dir_all(&root).ok();
    }
}
