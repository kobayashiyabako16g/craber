//! `--watch`: re-read the schema when one of its files or a workflow file
//! changes, or a file is added to or taken out of a schema or workflow
//! directory, and swap what `serve` is handing out.
//!
//! Polling `mtime` rather than taking a filesystem-notification dependency: the
//! check is one `stat` a second per file, which is not worth a crate.
//!
//! This does not refresh the browser. Doing that needs the page to hold a
//! connection open — server-sent events or a websocket — which is a real
//! feature rather than a detail, and is not worth building before anyone has
//! asked for it. Reloading the tab shows the new diagram.

use std::path::{Path, PathBuf};

use anyhow::Context;

use crate::commands::report;
use crate::options::Input;
use crate::{pipeline, server, sql_files, workflows};

/// Spawn the poller. `path` is known to be a file or a directory:
/// [`crate::options`] refuses `--watch` on a live connection while the
/// invocation is being resolved, so there is no "nothing to watch" case left to
/// handle here.
pub(crate) fn reload_on_change(site: server::Site, path: PathBuf, input: Input) {
    // A schema or workflow directory is listed again on every tick, so a file
    // added to it, or taken out, is a change like an edit is. A directory that
    // cannot be listed is said once, not every second, and watching carries on
    // with the files that can be seen.
    if path.is_dir() {
        tracing::info!(path = %path.display(), "watching for schema files added or removed");
    }
    let mut unlisted: Option<String> = None;
    let mut watched = move |flows: &[PathBuf]| -> Vec<PathBuf> {
        let listed = schema_files(&path).and_then(|schema| {
            let flows = workflows::files(flows)?
                .value
                .into_iter()
                .map(|file| file.path);
            Ok(schema.into_iter().chain(flows).collect())
        });
        match listed {
            Ok(files) => {
                unlisted = None;
                files
            }
            Err(error) => {
                let error = format!("{error:#}");
                if unlisted.as_ref() != Some(&error) {
                    tracing::warn!(%error, "could not list the files to watch");
                    unlisted = Some(error);
                }
                vec![path.clone()]
            }
        }
    };
    let stamps = |paths: Vec<PathBuf>| -> Vec<_> {
        paths
            .into_iter()
            .map(|path| {
                let stamp = stamp(&path);
                (path, stamp)
            })
            .collect()
    };
    let mut last = stamps(watched(&input.flows));

    for (path, _) in &last {
        tracing::info!(path = %path.display(), "watching — reload the page after a change");
    }
    for dir in input.flows.iter().filter(|p| p.is_dir()) {
        tracing::info!(path = %dir.display(), "watching for workflow files added or removed");
    }

    std::thread::spawn(move || {
        loop {
            std::thread::sleep(std::time::Duration::from_secs(1));
            let current = stamps(watched(&input.flows));
            if current == last {
                continue;
            }
            last = current;
            match pipeline::load(&input) {
                Ok(loaded) => {
                    report(&loaded.diagnostics);
                    match site.swap(&loaded) {
                        Ok(()) => tracing::info!("reloaded"),
                        Err(error) => tracing::error!(%error, "could not swap the site"),
                    }
                }
                // A half-saved file is a normal thing to catch mid-write. Say
                // so and keep serving the last good diagram.
                Err(error) => tracing::warn!(%error, "kept the previous diagram"),
            }
        }
    });
}

/// The schema's own files: `path` itself, or the `.sql` files beneath it that
/// are read.
fn schema_files(path: &Path) -> anyhow::Result<Vec<PathBuf>> {
    if !path.is_dir() {
        return Ok(vec![path.to_path_buf()]);
    }
    sql_files::schema_under(path)
        .with_context(|| format!("reading schema directory {}", path.display()))
}

fn stamp(path: &Path) -> Option<std::time::SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}
