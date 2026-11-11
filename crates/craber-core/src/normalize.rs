//! The pass that turns a parsed or introspected schema into a drawable one.
//!
//! Separate from [`ir`](crate::ir) because it is not part of the wire format:
//! `ir` defines what a schema *is*, and this fills in the parts of it that only
//! become knowable once a whole table has been read.
//!
//! Both producers end with [`normalize`] — `craber_parser::parse` and
//! `craber_introspect`'s `Catalog::finish` — which is what makes a file and a
//! live connection describe the same schema. Anything one producer would
//! otherwise tidy up on its own belongs here instead.

use crate::diagnostic::Diagnostic;
use crate::index::SchemaIndex;
use crate::ir::{Cardinality, Schema};

/// Finish a parsed or introspected schema. The diagnostics name what had to be
/// dropped.
#[must_use = "the diagnostics name foreign keys that were dropped"]
pub fn normalize(schema: &mut Schema) -> Vec<Diagnostic> {
    let diagnostics = fill_referenced_keys(schema);
    drop_blank_comments(schema);
    annotate_relationships(schema);
    diagnostics
}

/// A comment that is only whitespace is no comment.
///
/// MySQL stores `''` rather than NULL for a table with no comment, and `COMMENT
/// ON TABLE t IS ''` is how a file clears one; without this every such node
/// would carry an empty comment.
fn drop_blank_comments(schema: &mut Schema) {
    let blank = |comment: &mut Option<String>| {
        if comment
            .as_deref()
            .is_some_and(|text| text.trim().is_empty())
        {
            *comment = None;
        }
    };
    for table in &mut schema.tables {
        blank(&mut table.comment);
        for column in &mut table.columns {
            blank(&mut column.comment);
        }
    }
}

/// Fill in what the diagram's relationship ends are drawn from, none of which
/// a dialect states outright:
///
/// - a primary key column is `NOT NULL` whether or not the DDL repeated it,
///   which a table-level `PRIMARY KEY (a, b)` never does;
/// - [`ForeignKey::cardinality`](crate::ForeignKey::cardinality) from the uniqueness
///   of the FK's own columns —
///   a FK over a unique tuple is one-to-one, anything else many-to-one;
/// - [`ForeignKey::optional`](crate::ForeignKey::optional) from their nullability.
fn annotate_relationships(schema: &mut Schema) {
    for table in &mut schema.tables {
        for column in &mut table.columns {
            if table.pk.contains(&column.name) {
                column.nullable = false;
            }
        }

        let facts: Vec<(bool, bool)> = table
            .fks
            .iter()
            .map(|fk| {
                let optional = fk.columns.iter().any(|name| {
                    table
                        .columns
                        .iter()
                        .find(|column| &column.name == name)
                        .is_none_or(|column| column.nullable)
                });
                (table.is_unique_tuple(&fk.columns), optional)
            })
            .collect();

        for (fk, (is_unique, optional)) in table.fks.iter_mut().zip(facts) {
            fk.cardinality = if is_unique {
                Cardinality::OneToOne
            } else {
                Cardinality::ManyToOne
            };
            fk.optional = optional;
        }
    }
}

/// Point every foreign key with no referenced columns — `REFERENCES t` — at
/// its target's primary key. One whose target has no primary key is dropped
/// with a warning: it names no column for a line to attach to.
///
/// A target this schema does not have is left alone: there is no edge to draw
/// either way, and the details panel can still name it.
fn fill_referenced_keys(schema: &mut Schema) -> Vec<Diagnostic> {
    let index = SchemaIndex::build(schema);
    let mut diagnostics = Vec::new();
    let mut filled = Vec::new();
    for (t, table) in schema.tables.iter().enumerate() {
        for (f, fk) in table.fks.iter().enumerate() {
            if !fk.ref_columns.is_empty() {
                continue;
            }
            let Some(target) = index.resolve(&fk.ref_table) else {
                continue;
            };
            let pk = &schema.tables[target.index()].pk;
            if pk.is_empty() {
                diagnostics.push(
                    Diagnostic::warning(
                        "foreign key names no columns and its target has no primary key",
                    )
                    .with_context(format!(
                        "{} -> {}",
                        table.qualified_name(),
                        fk.ref_table.qualified_name()
                    )),
                );
            }
            filled.push((t, f, pk.clone()));
        }
    }
    // Back to front, so removing one key does not shift the index of the next.
    for (t, f, pk) in filled.into_iter().rev() {
        let fks = &mut schema.tables[t].fks;
        if pk.is_empty() {
            fks.remove(f);
        } else {
            fks[f].ref_columns = pk;
        }
    }
    diagnostics
}
