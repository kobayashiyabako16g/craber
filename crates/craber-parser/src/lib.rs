//! DDL → [`Schema`], and DML → [`Workflow`].
//!
//! Parsing is deliberately forgiving. Real schema dumps are full of statements
//! CrabER has no opinion about (grants, functions, extensions) and of syntax
//! sqlparser-rs does not cover yet. A statement that does not parse is skipped
//! (see `statements`), and one that parses but cannot become part of a table is
//! reported as a [`craber_core::Diagnostic`], so a single unsupported statement
//! never costs the user the whole diagram.

mod comments;
mod dialect;
mod flow;
mod names;
mod statements;
mod translate;

use craber_core::{Diagnostic, Dialect, Outcome, Schema, SchemaIndex, Workflow};
use sqlparser::parser::Parser;

use crate::dialect::sqlparser_dialect;

/// Parse a DDL script into a schema, collecting warnings for what was skipped.
///
/// `name` becomes [`Schema::name`] — normally the source filename.
pub fn parse(sql: &str, dialect: Dialect, name: impl Into<String>) -> Outcome<Schema> {
    let mut outcome = Outcome::new(Schema::new(name));

    let statements = statements::read(sql, dialect, &mut outcome);
    // Nothing read at all is the one case a caller should treat as fatal.
    if outcome.has_errors() {
        return outcome;
    }

    // Two passes: define tables and types, then apply the statements that
    // amend them. See `translate` for why the order matters.
    for statement in &statements {
        translate::define(statement, dialect, &mut outcome);
    }
    let index = SchemaIndex::build(&outcome.value);
    for statement in &statements {
        translate::amend(statement, &index, &mut outcome);
    }

    let diagnostics = craber_core::normalize(&mut outcome.value);
    outcome.diagnostics.extend(diagnostics);
    outcome
}

/// Parse a script of DML into a workflow, one step per statement.
///
/// `name` becomes [`Workflow::name`] — normally the source filename. As with
/// [`parse`], only a script that cannot be parsed at all is an error; a
/// statement that is not DML is a warning and is skipped.
pub fn parse_flow(sql: &str, dialect: Dialect, name: impl Into<String>) -> Outcome<Workflow> {
    let parser_dialect = sqlparser_dialect(dialect);
    match Parser::parse_sql(parser_dialect.as_ref(), sql) {
        Ok(statements) => {
            let comments = comments::Comments::read(sql, parser_dialect.as_ref(), statements.len());
            flow::workflow(&statements, name.into(), &comments)
        }
        Err(error) => {
            let mut outcome = Outcome::new(Workflow::new(name));
            outcome.push(Diagnostic::error(format!(
                "could not parse SQL as {dialect}: {error}"
            )));
            outcome
        }
    }
}
