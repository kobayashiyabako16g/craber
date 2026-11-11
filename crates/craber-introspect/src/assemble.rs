//! Turning flat catalog rows into a [`Schema`].
//!
//! Every backend asks its server a handful of independent questions — tables,
//! then columns, then constraints, then indexes — and gets back rows that name
//! the table they belong to. [`Catalog`] is the bookkeeping they share, so the
//! three backends cannot drift on how a table is looked up, how a warning is
//! worded, or what has to happen before a schema is finished. What is left in
//! each backend is its queries.

use std::collections::HashMap;

use craber_core::{Action, Diagnostic, EnumType, Outcome, Schema, Table, TableRef};

/// A schema being read: the tables so far, the enums, and what went wrong.
#[derive(Debug, Default)]
pub(crate) struct Catalog {
    /// In the order the catalog listed them. That is what the diagram's table
    /// list and every snapshot are in, so it has to come from the query's
    /// `ORDER BY` rather than from a hash map's iteration.
    tables: Vec<Table>,
    /// Exact names only. A catalog row names its table in full, so the lenient
    /// rule `SchemaIndex::resolve` applies to a reference has no place here.
    index: HashMap<TableRef, usize>,
    pub(crate) enums: Vec<EnumType>,
    diagnostics: Vec<Diagnostic>,
}

impl Catalog {
    pub(crate) fn push(&mut self, table: Table) {
        self.index.insert(table.to_ref(), self.tables.len());
        self.tables.push(table);
    }

    /// The table a catalog row belongs to.
    ///
    /// Every query spans the whole catalog, so a row naming a table that is
    /// not here is ordinary — a view, a partition child — rather than an error.
    pub(crate) fn table_mut(&mut self, name: &TableRef) -> Option<&mut Table> {
        let at = *self.index.get(name)?;
        self.tables.get_mut(at)
    }

    /// Record a warning about `context` — `table.constraint` for anything
    /// inside a table, so every backend's messages point the same way.
    pub(crate) fn warn(&mut self, message: impl Into<String>, context: impl Into<String>) {
        self.diagnostics
            .push(Diagnostic::warning(message).with_context(context));
    }

    /// A referential action, or a warning and `NO ACTION` when the catalog
    /// named one `parse` does not know.
    ///
    /// A warning rather than a failure: an unrecognised action costs the
    /// diagram nothing visible, and refusing to draw a schema over it would be
    /// a far worse trade than reporting it.
    pub(crate) fn action(
        &mut self,
        raw: &str,
        context: &str,
        parse: impl FnOnce(&str) -> Option<Action>,
    ) -> Action {
        parse(raw).unwrap_or_else(|| {
            self.warn(format!("unknown referential action `{raw}`"), context);
            Action::NoAction
        })
    }

    /// The finished schema.
    ///
    /// The [`normalize`](craber_core::normalize()) call is why this exists
    /// rather than each backend building its own `Outcome`. It is what fills in
    /// the crow's-foot ends and forces primary key columns `NOT NULL`;
    /// `craber_parser::parse` ends with it too, and a
    /// backend that forgot it would produce a diagram whose every relationship
    /// reads many-to-one — wrong, but not wrong enough to look broken.
    pub(crate) fn finish(mut self, name: impl Into<String>) -> Outcome<Schema> {
        let mut schema = Schema::new(name);
        schema.tables = self.tables;
        schema.enums = self.enums;
        self.diagnostics.extend(craber_core::normalize(&mut schema));
        Outcome {
            value: schema,
            diagnostics: self.diagnostics,
        }
    }
}

/// `table.constraint`, the context every backend gives a warning about a key.
pub(crate) fn context(table: &TableRef, constraint: &str) -> String {
    format!("{}.{constraint}", table.qualified_name())
}
