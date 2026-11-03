//! DDL → [`Schema`], and DML → [`Workflow`].
//!
//! Parsing is deliberately forgiving. Real schema dumps are full of statements
//! CrabER has no opinion about (grants, functions, extensions) and of syntax
//! sqlparser-rs does not cover yet. Anything we cannot turn into a table is
//! reported as a [`craber_core::Diagnostic`] and skipped, so a
//! single unsupported statement never costs the user the whole diagram.

mod dialect;
mod flow;
mod translate;

use craber_core::{Diagnostic, Outcome, Schema, Workflow};
use sqlparser::parser::Parser;

use crate::dialect::sqlparser_dialect;

pub use dialect::{Dialect, UnknownDialect};

/// Parse a DDL script into a schema, collecting warnings for what was skipped.
///
/// `name` becomes [`Schema::name`] — normally the source filename.
pub fn parse(sql: &str, dialect: Dialect, name: impl Into<String>) -> Outcome<Schema> {
    let mut outcome = Outcome::new(Schema::new(name));

    let statements = match Parser::parse_sql(sqlparser_dialect(dialect).as_ref(), sql) {
        Ok(statements) => statements,
        Err(error) => {
            // A whole-script parse failure means we produced nothing, which is
            // the one case a caller should treat as fatal.
            outcome.push(Diagnostic::error(format!(
                "could not parse SQL as {dialect}: {error}"
            )));
            return outcome;
        }
    };

    // Two passes: define tables and types, then apply the statements that
    // amend them. See `translate` for why the order matters.
    for statement in &statements {
        translate::define(statement, dialect, &mut outcome);
    }
    for statement in &statements {
        translate::amend(statement, &mut outcome);
    }

    craber_core::normalize::annotate_relationships(&mut outcome.value);
    outcome
}

/// Parse a script of DML into a workflow, one step per statement.
///
/// `name` becomes [`Workflow::name`] — normally the source filename. As with
/// [`parse`], only a script that cannot be parsed at all is an error; a
/// statement that is not DML is a warning and is skipped.
pub fn parse_flow(sql: &str, dialect: Dialect, name: impl Into<String>) -> Outcome<Workflow> {
    match Parser::parse_sql(sqlparser_dialect(dialect).as_ref(), sql) {
        Ok(statements) => flow::workflow(&statements, name.into()),
        Err(error) => {
            let mut outcome = Outcome::new(Workflow::new(name));
            outcome.push(Diagnostic::error(format!(
                "could not parse SQL as {dialect}: {error}"
            )));
            outcome
        }
    }
}
