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
use craber_core::{Diagnostic, Filter, Outcome, Schema};
use craber_layout::{Layout, SugiyamaLayout};

use crate::options::{Input, Source};

/// Read the source, narrow it, and lay it out.
///
/// Diagnostics are returned rather than printed so that `serve` can eventually
/// surface them in the UI instead of only on the terminal.
pub fn load(input: &Input) -> Result<(Schema, Vec<Diagnostic>)> {
    let (mut schema, diagnostics) = match &input.source {
        Source::File(path) => from_file(path, input)?.into_parts(),
        Source::Dsn(dsn) => from_dsn(dsn, input)?.into_parts(),
    };

    let filter = Filter::new(&input.include, &input.exclude, input.depth)?;
    filter.apply(&mut schema);

    SugiyamaLayout::default().apply(&mut schema);

    Ok((schema, diagnostics))
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
fn refuse_on_errors(outcome: &Outcome<Schema>, what: &str) -> Result<()> {
    if outcome.has_errors() {
        bail!("{what}:\n  {}", outcome.errors().join("\n  "));
    }
    Ok(())
}
