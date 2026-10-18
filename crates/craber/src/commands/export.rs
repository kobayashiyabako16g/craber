//! `craber export` — the diagram as a file, or on stdout.
//!
//! Which formats exist and how each one is produced is `craber-export`'s
//! business. What is left here is where the bytes go, which is the only part of
//! exporting that is a property of being a command-line tool: a path, or stdout
//! when there is no path.

use std::io::Write;
use std::path::Path;

use anyhow::{Context, Result};

use crate::commands::report;
use crate::options::ExportOptions;
use crate::pipeline;

pub fn run(options: &ExportOptions) -> Result<()> {
    let (schema, diagnostics) = pipeline::load(&options.input)?;
    report(&diagnostics);
    let encoded = craber_export::encode(&schema, options.format)?;
    write_out(options.output.as_deref(), &encoded)
}

fn write_out(output: Option<&Path>, contents: &str) -> Result<()> {
    let Some(path) = output else {
        let mut stdout = std::io::stdout().lock();
        stdout.write_all(contents.as_bytes())?;
        stdout.write_all(b"\n")?;
        return Ok(());
    };

    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    std::fs::write(path, contents).with_context(|| format!("writing {}", path.display()))?;
    tracing::info!(path = %path.display(), "wrote output");
    Ok(())
}
