//! Input → `Schema` → layout.
//!
//! Every command routes through here, which is what keeps `serve`, `build` and
//! `export` showing the same diagram: they differ only in how they package the
//! schema this module produces.
//!
//! What to read and how is decided before this is called, in
//! [`crate::options`]. By the time a command gets here the invocation has been
//! checked; what is left is the reading itself.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use craber_core::{Diagnostic, Filter, Flows, Outcome, Schema, SchemaIndex, Workflow};
use craber_layout::{Layout, SugiyamaLayout};

use crate::options::{Input, Source};

/// What a run draws: the laid-out schema, the workflows to play over it, and
/// what was skipped reading either.
pub struct Loaded {
    pub schema: Schema,
    pub flows: Flows,
    pub diagnostics: Vec<Diagnostic>,
}

/// Read the source, narrow it, lay it out, and read the workflows against it.
///
/// Diagnostics are returned rather than printed so that `serve` can eventually
/// surface them in the UI instead of only on the terminal.
pub fn load(input: &Input) -> Result<Loaded> {
    let (mut schema, mut diagnostics) = match &input.source {
        Source::File(path) => from_file(path, input)?.into_parts(),
        Source::Dsn(dsn) => from_dsn(dsn, input)?.into_parts(),
    };

    let filter = Filter::new(&input.include, &input.exclude, input.depth)?;
    filter.apply(&mut schema);

    SugiyamaLayout::default().apply(&mut schema);

    let (files, found) = workflow_files(&input.flows)?.into_parts();
    diagnostics.extend(found);
    let mut workflows: Vec<Workflow> = Vec::with_capacity(files.len());
    for (path, name) in &files {
        if workflows.iter().any(|w| &w.name == name) {
            diagnostics.push(Diagnostic::warning(format!(
                "two workflows are named {name}; the viewer opens only the first"
            )));
        }
        let (workflow, found) = workflow_from_file(path, name, input, &schema)?.into_parts();
        diagnostics.extend(found);
        workflows.push(workflow);
    }

    Ok(Loaded {
        schema,
        flows: Flows::new(workflows),
        diagnostics,
    })
}

/// The workflow files `--flow` names, each with the name it plays under.
///
/// A file is named by its stem, as it always was. A directory stands for every
/// `.sql` file beneath it, in order of their paths, each named by its path from
/// that directory without the extension — `billing/refund` — so that two
/// subdirectories can each have a `checkout.sql`. Hidden entries are passed
/// over, and so are symlinked directories, which could loop. Listing again on
/// every call is what lets `--watch` see a file that was added.
pub(crate) fn workflow_files(paths: &[PathBuf]) -> Result<Outcome<Vec<(PathBuf, String)>>> {
    let mut outcome = Outcome::new(Vec::new());
    for path in paths {
        if !path.is_dir() {
            let name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("workflow");
            outcome.value.push((path.clone(), name.to_owned()));
            continue;
        }
        let mut found = Vec::new();
        sql_files_under(path, &mut found)
            .with_context(|| format!("reading workflow directory {}", path.display()))?;
        if found.is_empty() {
            outcome.push(Diagnostic::warning(format!(
                "no .sql files under workflow directory {}",
                path.display()
            )));
        }
        found.sort();
        for file in found {
            let relative = file.strip_prefix(path).unwrap_or(&file).with_extension("");
            let name = relative
                .components()
                .map(|c| c.as_os_str().to_string_lossy())
                .collect::<Vec<_>>()
                .join("/");
            outcome.value.push((file, name));
        }
    }
    Ok(outcome)
}

fn sql_files_under(dir: &Path, found: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        if entry.file_name().to_string_lossy().starts_with('.') {
            continue;
        }
        let path = entry.path();
        if entry.file_type()?.is_dir() {
            sql_files_under(&path, found)?;
        } else if path.extension().is_some_and(|e| e == "sql") && path.is_file() {
            found.push(path);
        }
    }
    Ok(())
}

/// Read one workflow, and warn about each table it names that the diagram does
/// not have — a typo, or a table `--exclude` dropped. The step is kept either
/// way: the rest of what it touches is still worth showing.
fn workflow_from_file(
    path: &Path,
    name: &str,
    input: &Input,
    schema: &Schema,
) -> Result<Outcome<Workflow>> {
    let sql = std::fs::read_to_string(path)
        .with_context(|| format!("reading workflow file {}", path.display()))?;
    let mut outcome = craber_parser::parse_flow(&sql, input.dialect, name);
    refuse_on_errors(&outcome, &format!("could not read {}", path.display()))?;

    let index = SchemaIndex::build(schema);
    let mut missing: Vec<String> = Vec::new();
    for step in &outcome.value.steps {
        for touch in &step.touches {
            let table = touch.table.qualified_name();
            if index.resolve(&touch.table).is_none() && !missing.contains(&table) {
                missing.push(table);
            }
        }
    }
    for table in missing {
        outcome.push(Diagnostic::warning(format!(
            "workflow {name} touches `{table}`, which is not on the diagram"
        )));
    }
    Ok(outcome)
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
fn refuse_on_errors<T>(outcome: &Outcome<T>, what: &str) -> Result<()> {
    if outcome.has_errors() {
        bail!("{what}:\n  {}", outcome.errors().join("\n  "));
    }
    Ok(())
}
