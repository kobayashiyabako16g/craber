//! `schema.md`: the site's schema as text, for a reader that is not a browser.
//!
//! Written for an AI agent handed the site's URL, which cannot read the canvas
//! and wants every fact a query needs — which column joins to which, what may be NULL — in as
//! few tokens as possible. `schema.json` has the facts but also every table's
//! coordinates, and Mermaid's `erDiagram` has no way to say nullability,
//! defaults, indexes, referential actions or enum values.
//!
//! Sections a table does not have are left out rather than written empty: a
//! heading over nothing is tokens that say nothing.

use std::fmt::Write as _;

use craber_core::{Action, Cardinality, Column, ForeignKey, Schema, Table, TableIndex};

pub(crate) fn encode(schema: &Schema) -> String {
    let mut out = format!("# {}\n", schema.name);
    for table in &schema.tables {
        write_table(&mut out, table);
    }
    if !schema.enums.is_empty() {
        out.push_str("\n## Enums\n\n");
        for enumeration in &schema.enums {
            let name = match enumeration.schema.as_deref() {
                Some(schema) if !schema.is_empty() => format!("{schema}.{}", enumeration.name),
                _ => enumeration.name.clone(),
            };
            let _ = writeln!(out, "- {name}: {}", enumeration.values.join(", "));
        }
    }
    out
}

fn write_table(out: &mut String, table: &Table) {
    let _ = writeln!(out, "\n## {}\n", table.qualified_name());
    if let Some(comment) = &table.comment {
        let _ = writeln!(out, "{}\n", comment.trim());
    }

    if !table.columns.is_empty() {
        out.push_str("| column | type | null | default | key | comment |\n");
        out.push_str("|---|---|---|---|---|---|\n");
        for column in &table.columns {
            write_column(out, table, column);
        }
    }

    if !table.fks.is_empty() {
        out.push_str("\nForeign keys:\n");
        for fk in &table.fks {
            write_fk(out, fk);
        }
    }

    if !table.indexes.is_empty() {
        out.push_str("\nIndexes:\n");
        for index in &table.indexes {
            write_index(out, index);
        }
    }
}

fn write_column(out: &mut String, table: &Table, column: &Column) {
    let mut keys = Vec::new();
    if table.pk.contains(&column.name) {
        keys.push("PK".to_owned());
    }
    for fk in &table.fks {
        // Composite keys pair their columns by position.
        for (from, to) in fk.columns.iter().zip(&fk.ref_columns) {
            if *from == column.name {
                keys.push(format!("FK → {}.{to}", fk.ref_table.qualified_name()));
            }
        }
    }
    let _ = writeln!(
        out,
        "| {} | {} | {} | {} | {} | {} |",
        cell(&column.name),
        cell(&column.data_type),
        if column.nullable { "YES" } else { "NO" },
        cell(column.default.as_deref().unwrap_or("")),
        cell(&keys.join(", ")),
        cell(column.comment.as_deref().unwrap_or("")),
    );
}

fn write_fk(out: &mut String, fk: &ForeignKey) {
    let _ = write!(
        out,
        "- ({}) → {}({})",
        fk.columns.join(", "),
        fk.ref_table.qualified_name(),
        fk.ref_columns.join(", "),
    );
    // NO ACTION is what a key that never said gets, so only a stated action is
    // worth the tokens.
    if fk.on_delete != Action::NoAction {
        let _ = write!(out, ", on delete {}", action(fk.on_delete));
    }
    if fk.on_update != Action::NoAction {
        let _ = write!(out, ", on update {}", action(fk.on_update));
    }
    out.push_str(match fk.cardinality {
        Cardinality::ManyToOne => ", many-to-one",
        Cardinality::OneToOne => ", one-to-one",
    });
    if fk.optional {
        out.push_str(", optional");
    }
    if fk.inferred {
        out.push_str(", inferred from naming");
    }
    if let Some(name) = &fk.name {
        let _ = write!(out, " `{name}`");
    }
    out.push('\n');
}

fn write_index(out: &mut String, index: &TableIndex) {
    out.push_str("- ");
    if index.unique {
        out.push_str("UNIQUE ");
    }
    let _ = write!(out, "({})", index.columns.join(", "));
    if let Some(name) = &index.name {
        let _ = write!(out, " `{name}`");
    }
    out.push('\n');
}

/// The action as SQL spells it, which is how the reader will write it back.
fn action(action: Action) -> &'static str {
    match action {
        Action::NoAction => "NO ACTION",
        Action::Restrict => "RESTRICT",
        Action::Cascade => "CASCADE",
        Action::SetNull => "SET NULL",
        Action::SetDefault => "SET DEFAULT",
    }
}

/// `text` made safe for one table cell: a `|` would end the cell and a newline
/// the row.
fn cell(text: &str) -> String {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .replace('|', "\\|")
}

#[cfg(test)]
mod tests {
    use super::encode;
    use craber_core::{Action, Column, EnumType, ForeignKey, Schema, Table, TableIndex, TableRef};

    fn shop() -> Schema {
        let mut users = Table::new("users");
        users.columns = vec![Column::new("id", "bigint"), Column::new("email", "text")];
        users.columns[0].nullable = false;
        users.pk = vec!["id".into()];
        users.indexes = vec![TableIndex {
            name: Some("users_email_key".into()),
            columns: vec!["email".into()],
            unique: true,
        }];

        let mut orders = Table::new("orders");
        orders.columns = vec![Column::new("user_id", "bigint")];
        orders.fks = vec![
            ForeignKey::new("user_id", TableRef::new("users"), "id")
                .with_actions(Action::Cascade, Action::NoAction),
        ];

        let mut schema = Schema::new("shop");
        schema.tables = vec![users, orders];
        schema
    }

    #[test]
    fn a_foreign_key_names_the_column_it_joins_to() {
        let encoded = encode(&shop());
        assert!(
            encoded.contains("| user_id | bigint | YES |  | FK → users.id |  |"),
            "{encoded}"
        );
        assert!(
            encoded.contains("- (user_id) → users(id), on delete CASCADE, many-to-one\n"),
            "{encoded}"
        );
    }

    #[test]
    fn indexes_are_listed_with_their_names() {
        let encoded = encode(&shop());
        assert!(
            encoded.contains("- UNIQUE (email) `users_email_key`"),
            "{encoded}"
        );
        assert!(
            encoded.contains("| id | bigint | NO |  | PK |  |"),
            "{encoded}"
        );
    }

    /// A section a table does not have costs tokens and says nothing.
    #[test]
    fn empty_sections_are_left_out() {
        let encoded = encode(&shop());
        let orders = &encoded[encoded.find("## orders").expect("orders")..];
        assert!(!orders.contains("Indexes:"), "{orders}");
        assert!(!encoded.contains("## Enums"), "{encoded}");
    }

    #[test]
    fn enums_list_their_values() {
        let mut schema = shop();
        schema.enums = vec![EnumType {
            schema: Some("public".into()),
            name: "status".into(),
            values: vec!["paid".into(), "shipped".into()],
        }];
        assert!(encode(&schema).contains("- public.status: paid, shipped\n"));
    }

    #[test]
    fn a_cell_cannot_break_the_table() {
        let mut schema = shop();
        schema.tables[0].columns[1].comment = Some("a | b\nc".into());
        assert!(encode(&schema).contains("| a \\| b c |"));
    }
}
