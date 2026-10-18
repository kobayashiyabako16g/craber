//! Pins the `schema.json` wire format. The static site, the single-file HTML
//! and the WASM demo all parse this shape, so a change here is a breaking
//! change and should have to be reviewed as one.

use craber_core::{
    Action, Column, Detail, ForeignKey, Position, Positions, Schema, Table, TableRef, normalize,
};

fn sample() -> Schema {
    let mut schema = Schema::new("shop");

    let mut users = Table::new("users");
    users.schema = Some("public".into());
    users.comment = Some("Registered accounts".into());
    users.pk = vec!["id".into()];
    users.columns = vec![
        Column {
            nullable: false,
            ..Column::new("id", "bigint")
        },
        Column::new("email", "text"),
    ];
    users.positions = Some(Positions::uniform(Position::new(0.0, 0.0)));

    let mut orders = Table::new("orders");
    orders.schema = Some("public".into());
    orders.pk = vec!["id".into()];
    orders.columns = vec![
        Column {
            nullable: false,
            ..Column::new("id", "bigint")
        },
        Column {
            nullable: false,
            ..Column::new("user_id", "bigint")
        },
    ];
    orders.fks = vec![ForeignKey {
        name: Some("orders_user_id_fkey".into()),
        on_delete: Action::Cascade,
        ..ForeignKey::new("user_id", TableRef::qualified("public", "users"), "id")
    }];
    // The detail levels share an x and differ in y, the way a real layout
    // emits them.
    let mut orders_positions = Positions::uniform(Position::new(320.0, 0.0));
    orders_positions.set(Detail::AllFields, Position::new(320.0, 180.0));
    orders.positions = Some(orders_positions);

    schema.tables = vec![users, orders];
    normalize::annotate_relationships(&mut schema);
    schema
}

#[test]
fn schema_json_shape_is_stable() {
    insta::assert_json_snapshot!(sample());
}

#[test]
fn round_trips_through_json() {
    let schema = sample();
    let json = serde_json::to_string(&schema).unwrap();
    assert_eq!(serde_json::from_str::<Schema>(&json).unwrap(), schema);
}
