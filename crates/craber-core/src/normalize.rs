//! The pass that turns a parsed or introspected schema into a drawable one.
//!
//! Separate from [`ir`](crate::ir) because it is not part of the wire format:
//! `ir` defines what a schema *is*, and this fills in the parts of it that only
//! become knowable once a whole table has been read.
//!
//! Both producers end with this — `craber_parser::parse` and
//! `craber_introspect`'s `assemble::finish` — which is what makes a file and a
//! live connection describe the same relationships.

use crate::ir::{Cardinality, Schema};

/// Fill in what the diagram's relationship ends are drawn from, none of which
/// a dialect states outright:
///
/// - a primary key column is `NOT NULL` whether or not the DDL repeated it,
///   which a table-level `PRIMARY KEY (a, b)` never does;
/// - [`ForeignKey::cardinality`](crate::ForeignKey::cardinality) from the uniqueness
///   of the FK's own columns —
///   a FK over a unique tuple is one-to-one, anything else many-to-one;
/// - [`ForeignKey::optional`](crate::ForeignKey::optional) from their nullability.
pub fn annotate_relationships(schema: &mut Schema) {
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
