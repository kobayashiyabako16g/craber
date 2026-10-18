use craber_core::{Filter, ForeignKey, Schema, Table, TableRef};

/// users <- orders <- order_items, plus an unrelated `logs` island.
fn chain() -> Schema {
    let mut schema = Schema::new("chain");
    let mut orders = Table::new("orders");
    orders.fks = vec![ForeignKey::new("user_id", TableRef::new("users"), "id")];
    let mut items = Table::new("order_items");
    items.fks = vec![ForeignKey::new("order_id", TableRef::new("orders"), "id")];
    schema.tables = vec![Table::new("users"), orders, items, Table::new("logs")];
    schema
}

fn names(schema: &Schema) -> Vec<&str> {
    schema.tables.iter().map(|t| t.name.as_str()).collect()
}

#[test]
fn include_glob_selects_matching_tables() {
    let mut schema = chain();
    Filter::new(&["order*".into()], &[], None)
        .unwrap()
        .apply(&mut schema);
    assert_eq!(names(&schema), ["orders", "order_items"]);
}

#[test]
fn exclude_wins_over_include() {
    let mut schema = chain();
    Filter::new(&["order*".into()], &["order_items".into()], None)
        .unwrap()
        .apply(&mut schema);
    assert_eq!(names(&schema), ["orders"]);
}

#[test]
fn depth_walks_the_fk_graph_in_both_directions() {
    let mut schema = chain();
    Filter::new(&["orders".into()], &[], Some(1))
        .unwrap()
        .apply(&mut schema);
    assert_eq!(names(&schema), ["users", "orders", "order_items"]);
}

#[test]
fn depth_stops_at_the_requested_hop_count() {
    let mut schema = chain();
    Filter::new(&["users".into()], &[], Some(1))
        .unwrap()
        .apply(&mut schema);
    assert_eq!(names(&schema), ["users", "orders"]);
}

#[test]
fn excluded_tables_block_depth_expansion() {
    let mut schema = chain();
    Filter::new(&["users".into()], &["orders".into()], Some(5))
        .unwrap()
        .apply(&mut schema);
    assert_eq!(names(&schema), ["users"]);
}

#[test]
fn foreign_keys_to_filtered_out_tables_are_dropped() {
    let mut schema = chain();
    Filter::new(&["orders".into()], &[], None)
        .unwrap()
        .apply(&mut schema);
    let fks = &schema.table("orders").unwrap().fks;
    assert!(fks.is_empty(), "{fks:?}");
}

#[test]
fn empty_filter_changes_nothing() {
    let mut schema = chain();
    Filter::new(&[], &[], None).unwrap().apply(&mut schema);
    assert_eq!(names(&schema), ["users", "orders", "order_items", "logs"]);
}
