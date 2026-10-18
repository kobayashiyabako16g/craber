//! `craber build` — a static site written to a directory.

use anyhow::Result;

use crate::commands::report;
use crate::options::BuildOptions;
use crate::pipeline;

pub fn run(options: &BuildOptions) -> Result<()> {
    let (schema, diagnostics) = pipeline::load(&options.input)?;
    report(&diagnostics);

    let files = craber_site::write(&schema, &options.output, options.base_path.as_deref())?;
    tracing::info!(
        path = %options.output.display(),
        files = files.len(),
        "wrote static site"
    );
    Ok(())
}
