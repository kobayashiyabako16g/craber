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

use std::collections::HashSet;

use crate::diagnostic::Diagnostic;
use crate::index::SchemaIndex;
use crate::ir::{Cardinality, Schema};

/// Finish a parsed or introspected schema. The diagnostics name what had to be
/// dropped.
#[must_use = "the diagnostics name what was dropped"]
pub fn normalize(schema: &mut Schema) -> Vec<Diagnostic> {
    let mut diagnostics = drop_duplicate_tables(schema);
    diagnostics.extend(fill_referenced_keys(schema));
    drop_blank_comments(schema);
    annotate_relationships(schema);
    diagnostics
}

/// One table per name: the first definition is kept, as `CREATE TABLE IF NOT
/// EXISTS` would, and each later one is a warning.
///
/// Two tables with one qualified name are also one node id in the web app,
/// which keys its tables by that name and so would see only one of them —
/// while `SchemaIndex` would see two and call every reference to them
/// ambiguous. Dropping the duplicate here is what keeps the two readers of
/// `schema.json` resolving references the same way. An empty schema is read
/// as none on the way, as both of them already read it.
fn drop_duplicate_tables(schema: &mut Schema) -> Vec<Diagnostic> {
    let mut seen = HashSet::new();
    let mut diagnostics = Vec::new();
    schema.tables.retain_mut(|table| {
        table.schema = table.schema.take().filter(|name| !name.is_empty());
        if seen.insert(table.to_ref()) {
            return true;
        }
        diagnostics.push(
            Diagnostic::warning("table defined twice; the later definition is ignored")
                .with_context(table.qualified_name()),
        );
        false
    });
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
/// its target's primary key.
///
/// A key whose target is not in the schema, has no primary key, or has one of
/// a different width than the key's own columns is dropped with a warning:
/// there is no column list it could be given that pairs with its own, and
/// leaving it empty is what made the SQL download write `REFERENCES "t" ()`.
fn fill_referenced_keys(schema: &mut Schema) -> Vec<Diagnostic> {
    let index = SchemaIndex::build(schema);
    let mut diagnostics = Vec::new();
    // Per key: the target's primary key, or why there is none to use.
    let mut decided: Vec<(usize, usize, Result<Vec<String>, String>)> = Vec::new();
    for (t, table) in schema.tables.iter().enumerate() {
        for (f, fk) in table.fks.iter().enumerate() {
            if !fk.ref_columns.is_empty() {
                continue;
            }
            let pk = match index.resolve(&fk.ref_table) {
                None => Err("its target is not in the schema".to_owned()),
                Some(target) => {
                    let pk = &schema.tables[target.index()].pk;
                    if pk.is_empty() {
                        Err("its target has no primary key".to_owned())
                    } else if pk.len() == fk.columns.len() {
                        Ok(pk.clone())
                    } else {
                        Err(format!(
                            "its target's primary key has {} columns, not {}",
                            pk.len(),
                            fk.columns.len()
                        ))
                    }
                }
            };
            if let Err(reason) = &pk {
                diagnostics.push(
                    Diagnostic::warning(format!(
                        "foreign key names no columns and {reason}; skipped"
                    ))
                    .with_context(format!(
                        "{} -> {}",
                        table.qualified_name(),
                        fk.ref_table.qualified_name()
                    )),
                );
            }
            decided.push((t, f, pk));
        }
    }
    // Back to front, so removing one key does not shift the index of the next.
    for (t, f, pk) in decided.into_iter().rev() {
        let fks = &mut schema.tables[t].fks;
        match pk {
            Ok(pk) => fks[f].ref_columns = pk,
            Err(_) => {
                fks.remove(f);
            }
        }
    }
    diagnostics
}
