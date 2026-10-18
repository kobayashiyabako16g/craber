//! Turning flat catalog rows into a [`Schema`].
//!
//! Every backend asks its server a handful of independent questions — tables,
//! then columns, then constraints, then indexes — and gets back rows that name
//! the table they belong to. This is the part that puts them together, so the
//! three backends cannot drift on how a table is looked up or on what has to
//! happen before a schema is finished.

use std::collections::HashMap;

use craber_core::{Action, Diagnostic, Outcome, Schema, Table};

/// Tables in the order the catalog listed them, with a lookup by name.
///
/// The order matters: it is what the diagram's table list and every snapshot
/// are in, so it has to come from the query's `ORDER BY` rather than from a
/// hash map's iteration.
#[derive(Debug, Default)]
pub(crate) struct Tables {
    tables: Vec<Table>,
    index: HashMap<(Option<String>, String), usize>,
}

impl Tables {
    pub(crate) fn push(&mut self, table: Table) {
        self.index.insert(
            (table.schema.clone(), table.name.clone()),
            self.tables.len(),
        );
        self.tables.push(table);
    }

    pub(crate) fn get_mut(&mut self, schema: Option<&str>, name: &str) -> Option<&mut Table> {
        let at = self.at(schema, name)?;
        self.tables.get_mut(at)
    }

    pub(crate) fn get(&self, schema: Option<&str>, name: &str) -> Option<&Table> {
        self.tables.get(self.at(schema, name)?)
    }

    /// The position of a table in `tables`.
    ///
    /// Builds the owned key the map is keyed by, which costs two allocations
    /// per lookup on a path that runs once per catalog row. Left that way on
    /// purpose: a borrowed lookup into a `HashMap<(Option<String>, String), _>`
    /// needs either a linear scan or a second representation of the key, and
    /// both are worse trades than two short allocations.
    fn at(&self, schema: Option<&str>, name: &str) -> Option<usize> {
        let key = (schema.map(ToOwned::to_owned), name.to_owned());
        self.index.get(&key).copied()
    }

    pub(crate) fn into_vec(self) -> Vec<Table> {
        self.tables
    }
}

/// Assemble the finished schema.
///
/// The [`annotate_relationships`](craber_core::normalize::annotate_relationships) call
/// is why this function exists rather than each backend building its own
/// `Outcome`. It is what fills in the crow's-foot ends and forces primary key
/// columns `NOT NULL`, `craber_parser::parse` ends with it, and a backend that
/// forgot it would produce a diagram whose every relationship reads
/// many-to-one — wrong, but not wrong enough to look broken.
pub(crate) fn finish(
    name: impl Into<String>,
    tables: Tables,
    enums: Vec<craber_core::EnumType>,
    diagnostics: Vec<Diagnostic>,
) -> Outcome<Schema> {
    let mut schema = Schema::new(name);
    schema.tables = tables.into_vec();
    schema.enums = enums;
    craber_core::normalize::annotate_relationships(&mut schema);
    Outcome {
        value: schema,
        diagnostics,
    }
}

/// A referential action, or a warning and `NO ACTION` when the catalog named
/// one this does not know.
///
/// `parse` is the only part that differs between backends — PostgreSQL reports
/// single characters where MySQL and SQLite report words — so the warning
/// itself is written once rather than three times.
///
/// A warning rather than a failure: an unrecognised action costs the diagram
/// nothing visible, and refusing to draw a schema over it would be a far worse
/// trade than reporting it.
pub(crate) fn action_or_warn(
    raw: &str,
    context: &str,
    diagnostics: &mut Vec<Diagnostic>,
    parse: impl FnOnce(&str) -> Option<Action>,
) -> Action {
    parse(raw).unwrap_or_else(|| {
        diagnostics.push(
            Diagnostic::warning(format!("unknown referential action `{raw}`"))
                .with_context(context.to_owned()),
        );
        Action::NoAction
    })
}

/// A referential action as MySQL and SQLite both spell it, or a warning.
///
/// [`action_or_warn`] paired with [`action_from_words`], which is what both of
/// those backends wanted and each wrote out for itself. PostgreSQL does not use
/// this: it reports single characters and maps them in its own module.
pub(crate) fn action_in_words(
    raw: &str,
    context: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> Action {
    action_or_warn(raw, context, diagnostics, action_from_words)
}

/// The spelling MySQL and SQLite both use for a referential action.
///
/// PostgreSQL reports a single character instead and maps it in its own module.
fn action_from_words(word: &str) -> Option<Action> {
    match word.to_ascii_uppercase().as_str() {
        "NO ACTION" | "" => Some(Action::NoAction),
        "RESTRICT" => Some(Action::Restrict),
        "CASCADE" => Some(Action::Cascade),
        "SET NULL" => Some(Action::SetNull),
        "SET DEFAULT" => Some(Action::SetDefault),
        _ => None,
    }
}

/// A comment that is only whitespace is no comment.
///
/// MySQL stores `''` rather than NULL for a table with no comment, and without
/// this every node would carry an empty one.
pub(crate) fn comment(raw: Option<String>) -> Option<String> {
    raw.filter(|text| !text.trim().is_empty())
}
