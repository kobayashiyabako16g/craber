//! The workflow files `--flow` names, and reading one against the diagram.
//!
//! Separate from [`crate::pipeline`] because `--watch` needs the list of files
//! on its own, every second, without loading anything.

use std::path::PathBuf;

use anyhow::{Context, Result};
use craber_core::{Diagnostic, Dialect, Outcome, SchemaIndex, Workflow};

use crate::pipeline::refuse_on_errors;
use crate::sql_files;

/// One workflow file, and the name it plays under.
#[derive(Debug, Clone)]
pub(crate) struct WorkflowFile {
    pub path: PathBuf,
    pub name: String,
}

/// The workflow files `paths` stand for.
///
/// A file is named by its stem, as it always was. A directory stands for every
/// `.sql` file beneath it, in order of their paths, each named by its path from
/// that directory without the extension — `billing/refund` — so that two
/// subdirectories can each have a `checkout.sql`. Hidden entries are passed
/// over, and so are symlinked directories, which could loop. Listing again on
/// every call is what lets `--watch` see a file that was added.
pub(crate) fn files(paths: &[PathBuf]) -> Result<Outcome<Vec<WorkflowFile>>> {
    let mut outcome = Outcome::new(Vec::new());
    for path in paths {
        if !path.is_dir() {
            let name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("workflow");
            outcome.value.push(WorkflowFile {
                path: path.clone(),
                name: name.to_owned(),
            });
            continue;
        }
        let found = sql_files::sorted_under(path)
            .with_context(|| format!("reading workflow directory {}", path.display()))?;
        if found.is_empty() {
            outcome.push(Diagnostic::warning(format!(
                "no .sql files under workflow directory {}",
                path.display()
            )));
        }
        for file in found {
            let name = sql_files::name_within(path, &file);
            outcome.value.push(WorkflowFile { path: file, name });
        }
    }
    Ok(outcome)
}

/// Read one workflow, and warn about each table it names that the diagram does
/// not have — a typo, or a table `--exclude` dropped. The step is kept either
/// way: the rest of what it touches is still worth showing.
pub(crate) fn read(
    file: &WorkflowFile,
    dialect: Dialect,
    diagram: &SchemaIndex,
) -> Result<Outcome<Workflow>> {
    let WorkflowFile { path, name } = file;
    let sql = std::fs::read_to_string(path)
        .with_context(|| format!("reading workflow file {}", path.display()))?;
    let mut outcome = craber_parser::parse_flow(&sql, dialect, name);
    refuse_on_errors(&outcome, &format!("could not read {}", path.display()))?;

    let mut missing: Vec<String> = Vec::new();
    for step in &outcome.value.steps {
        for touch in &step.touches {
            let table = touch.table.qualified_name();
            if diagram.resolve(&touch.table).is_none() && !missing.contains(&table) {
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
