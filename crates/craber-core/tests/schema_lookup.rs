//! `Schema::table`, the lookup by the name a reader would type.

use craber_core::{Schema, Table};

fn schema(tables: &[(Option<&str>, &str)]) -> Schema {
    let mut schema = Schema::new("lookup");
    for (namespace, name) in tables {
        let mut table = Table::new(*name);
        table.schema = namespace.map(str::to_owned);
        schema.tables.push(table);
    }
    schema
}

#[test]
fn finds_by_qualified_name_then_by_unambiguous_bare_name() {
    let schema = schema(&[
        (Some("public"), "users"),
        (Some("audit"), "users"),
        (None, "orders"),
    ]);
    assert_eq!(
        schema.table("audit.users").unwrap().schema.as_deref(),
        Some("audit")
    );
    assert!(
        schema.table("users").is_none(),
        "two schemas share the bare name"
    );
    assert_eq!(schema.table("orders").unwrap().name, "orders");
}

/// A quoted identifier may contain a dot; comparing whole names finds it where
/// splitting on the dot would not.
#[test]
fn finds_a_name_that_contains_a_dot() {
    let schema = schema(&[(Some("public"), "a.b")]);
    assert_eq!(schema.table("public.a.b").unwrap().name, "a.b");
    assert_eq!(schema.table("a.b").unwrap().name, "a.b");
}
