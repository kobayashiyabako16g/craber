//! `craber serve` — the diagram on a local HTTP port, until interrupted.

use anyhow::Result;

use crate::commands::report;
use crate::options::ServeOptions;
use crate::{pipeline, server, watch};

pub(crate) fn run(options: &ServeOptions) -> Result<()> {
    let loaded = pipeline::load(&options.input)?;
    report(&loaded.diagnostics);

    let site = server::run(&loaded, options.port)?;
    if let Some(path) = &options.watch {
        watch::reload_on_change(site, path.clone(), options.input.clone());
    }

    // The listener runs on its own thread; park this one until the user stops
    // the process.
    loop {
        std::thread::park();
    }
}
