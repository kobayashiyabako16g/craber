//! DML → [`Workflow`]: what each statement reads, writes and deletes.
//!
//! Nothing is evaluated. A step records the tables and columns its statement
//! names, which is what the viewer needs to light the part of the diagram a
//! step is about. Column names are attributed to a table only when the SQL
//! says which one — through a qualifier or because one table is in scope — so
//! that an ambiguous `id` is dropped rather than drawn on the wrong node.
//!
//! Values are recorded the same way, as the SQL writes them: what `INSERT` and
//! `UPDATE` put in a column, and what `WHERE` and `ON` hold one to. Only the
//! conditions joined by `AND` are kept — one inside an `OR` is not something
//! the row is known to satisfy, so showing it beside the column would mislead.
//!
//! The same forgiving rule as DDL applies: a statement this cannot read is a
//! warning and is skipped, never the end of the workflow.
//!
//! - [`scope`]: which tables a column name can mean;
//! - [`builder`]: the step being recorded — touches, values, joins;
//! - [`condition`]: what `WHERE` and `ON` say about a column;
//! - [`statement`]: walking `SELECT`, `INSERT`, `UPDATE` and `DELETE`.

mod builder;
mod condition;
mod scope;
mod statement;

use craber_core::{Diagnostic, Outcome, Step, StepKind, Workflow};
use sqlparser::ast::Statement;

use crate::comments::Comments;

use self::builder::StepBuilder;
use self::scope::Scope;

/// Read each statement of `statements` as one step of a workflow called `name`,
/// described by the script's `comments`.
pub(crate) fn workflow(
    statements: &[Statement],
    name: String,
    comments: &Comments,
) -> Outcome<Workflow> {
    let mut outcome = Outcome::new(Workflow::new(name));
    outcome.value.description.clone_from(&comments.workflow);
    for (index, statement) in statements.iter().enumerate() {
        let mut step = StepBuilder::default();
        let kind = match statement {
            Statement::Query(query) => {
                step.query(query, &Scope::default());
                StepKind::Select
            }
            Statement::Insert(insert) => {
                step.insert(insert);
                StepKind::Insert
            }
            Statement::Update(update) => {
                step.update(update);
                StepKind::Update
            }
            Statement::Delete(delete) => {
                step.delete(delete);
                StepKind::Delete
            }
            // Transaction control frames the steps; it is not one of them.
            Statement::StartTransaction { .. }
            | Statement::Commit { .. }
            | Statement::Rollback { .. } => continue,
            other => {
                outcome.push(Diagnostic::warning(format!(
                    "skipped a statement that is not SELECT/INSERT/UPDATE/DELETE: {}",
                    first_words(&other.to_string())
                )));
                continue;
            }
        };
        let (touches, joins) = step.finish();
        outcome.value.steps.push(Step {
            kind,
            description: comments.step(index),
            // sqlparser's pretty form ends a statement with a space.
            sql: format!("{statement:#}").trim_end().to_owned(),
            touches,
            joins,
        });
    }
    outcome
}

/// Enough of a statement to recognise it in a warning.
fn first_words(sql: &str) -> String {
    let words: Vec<&str> = sql.split_whitespace().take(4).collect();
    let mut out = words.join(" ");
    if sql.split_whitespace().count() > 4 {
        out.push_str(" …");
    }
    out
}
