//! Pins the `flows.json` wire format, for the same reason `wire_format.rs`
//! pins `schema.json`: a site and the binary that wrote its data are deployed
//! independently, so a change here is a breaking change.

use craber_core::{Access, ColumnRef, Flows, Join, Step, StepKind, TableRef, Touch, Workflow};

fn sample() -> Flows {
    let mut workflow = Workflow::new("checkout");
    workflow.steps = vec![
        Step {
            kind: StepKind::Select,
            sql: "SELECT o.id FROM orders AS o JOIN users AS u ON o.user_id = u.id".into(),
            touches: vec![
                Touch {
                    table: TableRef::new("orders"),
                    access: Access::Read,
                    columns: vec!["id".into(), "user_id".into()],
                },
                Touch {
                    table: TableRef::new("users"),
                    access: Access::Read,
                    columns: vec!["id".into()],
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
            sql: "DELETE FROM orders".into(),
            touches: vec![Touch {
                table: TableRef::qualified("public", "orders"),
                access: Access::Delete,
                columns: Vec::new(),
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
