//! `--watch`: re-read the schema when its file changes, and swap what `serve`
//! is handing out.
//!
//! Polling `mtime` rather than taking a filesystem-notification dependency: the
//! check is one `stat` a second against one file, which is not worth a crate.
//!
//! This does not refresh the browser. Doing that needs the page to hold a
//! connection open — server-sent events or a websocket — which is a real
//! feature rather than a detail, and is not worth building before anyone has
//! asked for it. Reloading the tab shows the new diagram.

use std::path::{Path, PathBuf};

use crate::commands::report;
use crate::options::Input;
use crate::{pipeline, server};

/// Spawn the poller. `path` is known to be a file: [`crate::options`] refuses
/// `--watch` on a live connection while the invocation is being resolved, so
/// there is no "nothing to watch" case left to handle here.
pub fn reload_on_change(site: server::Site, path: PathBuf, input: Input) {
    let stamp = |path: &Path| std::fs::metadata(path).and_then(|m| m.modified()).ok();
    let mut last = stamp(&path);

    tracing::info!(path = %path.display(), "watching — reload the page after a change");

    std::thread::spawn(move || {
        loop {
            std::thread::sleep(std::time::Duration::from_secs(1));
            let current = stamp(&path);
            if current == last {
                continue;
            }
            last = current;
            match pipeline::load(&input) {
                Ok((schema, diagnostics)) => {
                    report(&diagnostics);
                    match server::swap(&site, &schema) {
                        Ok(()) => tracing::info!(path = %path.display(), "reloaded"),
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
