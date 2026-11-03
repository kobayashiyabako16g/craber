use craber_core::{Outcome, Severity, StepKind, Workflow};
use craber_parser::{Dialect, parse_flow};

fn postgres(sql: &str) -> Outcome<Workflow> {
    parse_flow(sql, Dialect::Postgres, "test")
}

/// `table: access [columns]` per touch, which is what a test wants to say.
fn touches(outcome: &Outcome<Workflow>, step: usize) -> Vec<String> {
    outcome.value.steps[step]
        .touches
        .iter()
        .map(|t| {
            format!(
                "{}: {:?} [{}]",
                t.table.qualified_name(),
                t.access,
                t.columns.join(", ")
            )
        })
        .collect()
}

#[test]
fn each_statement_is_a_step() {
    let outcome = postgres(
        "SELECT * FROM users;
         INSERT INTO orders (user_id, total) VALUES (1, 10);
         UPDATE orders SET total = 20 WHERE id = 1;
         DELETE FROM orders WHERE id = 1;",
    );
    let kinds: Vec<StepKind> = outcome.value.steps.iter().map(|s| s.kind).collect();
    assert_eq!(
        kinds,
        [
            StepKind::Select,
            StepKind::Insert,
            StepKind::Update,
            StepKind::Delete
        ]
    );
    assert_eq!(outcome.diagnostics, []);
}

#[test]
fn select_reads_its_tables_and_records_equality_joins() {
    let outcome = postgres(
        "SELECT o.id, u.email FROM orders o JOIN users AS u ON o.user_id = u.id WHERE u.active",
    );
    assert_eq!(
        touches(&outcome, 0),
        [
            "orders: Read [user_id, id]",
            "users: Read [id, email, active]"
        ]
    );
    let join = &outcome.value.steps[0].joins[0];
    assert_eq!(
        (join.left.table.name.as_str(), join.left.column.as_str()),
        ("orders", "user_id")
    );
    assert_eq!(
        (join.right.table.name.as_str(), join.right.column.as_str()),
        ("users", "id")
    );
}

#[test]
fn an_unqualified_column_is_dropped_when_two_tables_could_own_it() {
    let outcome = postgres("SELECT id FROM orders JOIN users ON orders.user_id = users.id");
    assert_eq!(
        touches(&outcome, 0),
        ["orders: Read [user_id]", "users: Read [id]"]
    );
}

#[test]
fn insert_select_writes_the_target_and_reads_the_source() {
    let outcome = postgres("INSERT INTO archive (id) SELECT id FROM orders WHERE total > 0");
    assert_eq!(
        touches(&outcome, 0),
        ["archive: Write [id]", "orders: Read [id, total]"]
    );
}

#[test]
fn update_writes_set_columns_and_keeps_the_stronger_access() {
    let outcome = postgres(
        "UPDATE stock s SET quantity = s.quantity - 1 FROM orders o WHERE o.product_id = s.product_id",
    );
    assert_eq!(
        touches(&outcome, 0),
        [
            "stock: Write [quantity, product_id]",
            "orders: Read [product_id]"
        ]
    );
}

#[test]
fn delete_follows_subqueries() {
    let outcome =
        postgres("DELETE FROM orders WHERE user_id IN (SELECT id FROM users WHERE banned)");
    assert_eq!(
        touches(&outcome, 0),
        ["orders: Delete [user_id]", "users: Read [id, banned]"]
    );
}

#[test]
fn a_cte_reads_its_tables_and_is_not_a_table_itself() {
    let outcome = postgres(
        "WITH recent AS (SELECT user_id FROM orders) SELECT u.id FROM users u JOIN recent r ON r.user_id = u.id",
    );
    assert_eq!(
        touches(&outcome, 0),
        ["orders: Read [user_id]", "users: Read [id]"]
    );
}

#[test]
fn schema_qualified_tables_keep_their_namespace() {
    let outcome = postgres("SELECT * FROM shop.orders");
    assert_eq!(touches(&outcome, 0), ["shop.orders: Read []"]);
}

#[test]
fn ddl_and_other_statements_are_warned_about_and_skipped() {
    let outcome = postgres("BEGIN; CREATE TABLE t (id int); SELECT * FROM t; COMMIT;");
    assert_eq!(outcome.value.steps.len(), 1);
    assert_eq!(outcome.diagnostics.len(), 1, "BEGIN/COMMIT are not news");
    assert_eq!(outcome.diagnostics[0].severity, Severity::Warning);
}

#[test]
fn unparseable_sql_is_an_error() {
    let outcome = postgres("SELEC nonsense");
    assert!(outcome.has_errors());
}
