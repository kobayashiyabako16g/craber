//! Pins the `flows.json` wire format, for the same reason `wire_format.rs`
//! pins `schema.json`: a site and the binary that wrote its data are deployed
//! independently, so a change here is a breaking change.

use craber_core::{
    Access, ColumnRef, ColumnValue, Flows, Join, Step, StepKind, TableRef, Touch, ValueKind,
    Workflow,
};

fn sample() -> Flows {
    let mut workflow = Workflow::new("checkout");
    workflow.description = Some("A checkout: the order is created and paid.".into());
    workflow.steps = vec![
        Step {
            kind: StepKind::Select,
            description: Some("Who is buying.".into()),
            sql: "SELECT o.id FROM orders AS o JOIN users AS u ON o.user_id = u.id".into(),
            touches: vec![
                Touch {
                    table: TableRef::new("orders"),
                    access: Access::Read,
                    columns: vec!["id".into(), "user_id".into()],
                    values: Vec::new(),
                },
                Touch {
                    table: TableRef::new("users"),
                    access: Access::Read,
                    columns: vec!["id".into()],
                    values: Vec::new(),
                },
            ],
            joins: vec![Join {
                left: ColumnRef {
                    table: TableRef::new("orders"),
                    column: "user_id".into(),
                },
                right: ColumnRef {
                    table: TableRef::new("users"),
                    column: "id".into(),
                },
            }],
        },
        Step {
            kind: StepKind::Delete,
            description: None,
            sql: "DELETE FROM orders".into(),
            touches: vec![Touch {
                table: TableRef::qualified("public", "orders"),
                access: Access::Delete,
                columns: Vec::new(),
                values: Vec::new(),
            }],
            joins: Vec::new(),
        },
        Step {
            kind: StepKind::Update,
            description: None,
            sql: "UPDATE orders SET total = 59.80 WHERE id = 100".into(),
            touches: vec![Touch {
                table: TableRef::new("orders"),
                access: Access::Write,
                columns: vec!["total".into(), "id".into()],
                values: vec![
                    ColumnValue {
                        column: "total".into(),
                        kind: ValueKind::Set,
                        text: "59.80".into(),
                    },
                    ColumnValue {
                        column: "id".into(),
                        kind: ValueKind::Filter,
                        text: "= 100".into(),
                    },
                ],
            }],
            joins: Vec::new(),
        },
    ];
    Flows::new(vec![workflow])
}

#[test]
fn flows_json_shape_is_stable() {
    insta::assert_json_snapshot!(sample());
}

#[test]
fn round_trips_through_json() {
    let flows = sample();
    let json = serde_json::to_string(&flows).unwrap();
    assert_eq!(serde_json::from_str::<Flows>(&json).unwrap(), flows);
}
