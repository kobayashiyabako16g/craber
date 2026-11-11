//! Input → `Schema` → layout.
//!
//! Every command routes through here, which is what keeps `serve`, `build` and
//! `export` showing the same diagram: they differ only in how they package the
//! schema this module produces.
//!
//! What to read and how is decided before this is called, in
//! [`crate::options`]. By the time a command gets here the invocation has been
//! checked; what is left is the reading itself.

use std::path::Path;

use anyhow::{Context, Result, bail};
use craber_core::{Diagnostic, Filter, Flows, Outcome, Schema, SchemaIndex, Workflow};
use craber_layout::{Layout, SugiyamaLayout};

use crate::options::{Input, Source};
use crate::workflows;

/// What a run draws: the laid-out schema, the workflows to play over it, and
/// what was skipped reading either.
pub(crate) struct Loaded {
    pub schema: Schema,
    pub flows: Flows,
    pub diagnostics: Vec<Diagnostic>,
}

/// Read the source, narrow it, lay it out, and read the workflows against it.
///
/// Diagnostics are returned rather than printed so that `serve` can eventually
/// surface them in the UI instead of only on the terminal.
pub(crate) fn load(input: &Input) -> Result<Loaded> {
    let (mut schema, mut diagnostics) = match &input.source {
        Source::File(path) => from_file(path, input)?.into_parts(),
        Source::Dsn(dsn) => from_dsn(dsn, input)?.into_parts(),
    };

    let filter = Filter::new(&input.include, &input.exclude, input.depth)?;
    filter.apply(&mut schema);

    SugiyamaLayout::default().apply(&mut schema);

    let (files, found) = workflows::files(&input.flows)?.into_parts();
    diagnostics.extend(found);
    let diagram = SchemaIndex::build(&schema);
    let mut read: Vec<Workflow> = Vec::with_capacity(files.len());
    for file in &files {
        if read.iter().any(|w| w.name == file.name) {
            diagnostics.push(Diagnostic::warning(format!(
                "two workflows are named {}; the viewer opens only the first",
                file.name
            )));
        }
        let (workflow, found) = workflows::read(file, input.dialect, &diagram)?.into_parts();
        diagnostics.extend(found);
        read.push(workflow);
    }

    Ok(Loaded {
        schema,
        flows: Flows::new(read),
        diagnostics,
    })
}

fn from_file(path: &Path, input: &Input) -> Result<Outcome<Schema>> {
    let sql = std::fs::read_to_string(path)
        .with_context(|| format!("reading schema file {}", path.display()))?;

    let name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("schema");
    let outcome = craber_parser::parse(&sql, input.dialect, name);

    refuse_on_errors(&outcome, &format!("could not read {}", path.display()))?;
    Ok(outcome)
}

/// Connect and read the schema.
///
/// Nothing here may put `dsn` in a message. A connection string carries a
/// password, and CrabER's errors go to stderr and from there into CI logs —
/// which is why this says "the postgres database" where [`from_file`] says
/// which file it could not read.
fn from_dsn(dsn: &str, input: &Input) -> Result<Outcome<Schema>> {
    let dialect = input.dialect;
    let outcome = craber_introspect::introspect(dsn, dialect)
        .with_context(|| format!("reading the {dialect} database"))?;

    refuse_on_errors(&outcome, &format!("could not read the {dialect} database"))?;
    Ok(outcome)
}

/// Give up when the source produced error-severity diagnostics, listing them
/// under `what`.
///
/// Warnings do not come through here. A schema CrabER could only partly read is
/// still a diagram worth drawing, and the warnings are reported alongside it —
/// an error is the case where there is nothing worth showing.
pub(crate) fn refuse_on_errors<T>(outcome: &Outcome<T>, what: &str) -> Result<()> {
    if outcome.has_errors() {
        bail!("{what}:\n  {}", outcome.errors().join("\n  "));
    }
    Ok(())
}
