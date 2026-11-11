use craber_core::Dialect;
use craber_core::{Outcome, Severity, StepKind, Workflow};
use craber_parser::parse_flow;

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

/// `table.column kind text` per value, in the order the statement names them.
fn values(outcome: &Outcome<Workflow>, step: usize) -> Vec<String> {
    outcome.value.steps[step]
        .touches
        .iter()
        .flat_map(|t| {
            t.values
                .iter()
                .map(move |v| format!("{}.{} {:?} {}", t.table.name, v.column, v.kind, v.text))
        })
        .collect()
}

#[test]
fn insert_values_are_set_on_their_columns() {
    let outcome = postgres("INSERT INTO orders (id, status) VALUES (100, 'pending')");
    assert_eq!(
        values(&outcome, 0),
        ["orders.id Set 100", "orders.status Set 'pending'"]
    );
}

#[test]
fn a_multi_row_insert_lists_each_row_and_cuts_after_three() {
    let outcome = postgres("INSERT INTO t (id) VALUES (1), (2), (3), (4)");
    assert_eq!(values(&outcome, 0), ["t.id Set 1 | 2 | 3 | …"]);
}

#[test]
fn insert_select_pairs_columns_with_what_is_projected() {
    let outcome = postgres(
        "INSERT INTO receipts (id, order_id) SELECT 900, o.id FROM orders o WHERE o.id = 1",
    );
    assert_eq!(
        values(&outcome, 0),
        [
            "receipts.id Set 900",
            "receipts.order_id Set o.id",
            "orders.id Filter = 1"
        ]
    );
}

#[test]
fn update_sets_and_filters() {
    let outcome = postgres("UPDATE orders SET total = total + 1, status = 'paid' WHERE id = 100");
    assert_eq!(
        values(&outcome, 0),
        [
            "orders.total Set total + 1",
            "orders.status Set 'paid'",
            "orders.id Filter = 100"
        ]
    );
}

#[test]
fn conditions_read_column_first_whichever_way_they_were_written() {
    let outcome = postgres(
        "SELECT * FROM orders WHERE 10 < total AND status IN ('a', 'b') AND coupon_id IS NULL \
         AND id BETWEEN 1 AND 5 AND note NOT LIKE 'x%'",
    );
    assert_eq!(
        values(&outcome, 0),
        [
            "orders.total Filter > 10",
            "orders.status Filter IN ('a', 'b')",
            "orders.coupon_id Filter IS NULL",
            "orders.id Filter BETWEEN 1 AND 5",
            "orders.note Filter NOT LIKE 'x%'"
        ]
    );
}

#[test]
fn a_condition_inside_or_is_not_a_value() {
    let outcome = postgres("DELETE FROM orders WHERE id = 1 OR status = 'void'");
    assert_eq!(values(&outcome, 0), Vec::<String>::new());
}

#[test]
fn a_constant_in_an_on_clause_is_a_filter_and_a_column_pair_is_not() {
    let outcome = postgres(
        "SELECT * FROM users u JOIN coupons c ON c.code = 'WELCOME' JOIN orders o ON o.user_id = u.id",
    );
    assert_eq!(values(&outcome, 0), ["coupons.code Filter = 'WELCOME'"]);
}

#[test]
fn a_step_keeps_its_statement_pretty_printed() {
    let outcome =
        postgres("SELECT u.id FROM users u JOIN orders o ON o.user_id = u.id WHERE u.id = 1;");
    let sql = &outcome.value.steps[0].sql;
    assert!(sql.contains('\n'), "expected several lines, got {sql:?}");
    assert!(
        sql.starts_with("SELECT") && sql.ends_with("= 1"),
        "got {sql:?}"
    );
}

fn descriptions(outcome: &Outcome<Workflow>) -> Vec<Option<&str>> {
    outcome
        .value
        .steps
        .iter()
        .map(|s| s.description.as_deref())
        .collect()
}

#[test]
fn a_comment_directly_above_a_statement_describes_its_step() {
    let outcome = postgres(
        "-- Who is buying.
         SELECT * FROM users;

         -- Create the order,
         -- still pending.
         INSERT INTO orders (id) VALUES (1);

         UPDATE orders SET total = 1 WHERE id = 1;",
    );
    assert_eq!(
        descriptions(&outcome),
        [
            Some("Who is buying."),
            Some("Create the order,\nstill pending."),
            None
        ]
    );
    // Attached to the first statement, it is that step's and not the workflow's.
    assert_eq!(outcome.value.description, None);
}

#[test]
fn the_comment_at_the_top_set_apart_by_a_blank_line_describes_the_workflow() {
    let outcome = postgres(
        "-- A checkout:
         -- the order is created and paid.

         -- Who is buying.
         SELECT * FROM users;",
    );
    assert_eq!(
        outcome.value.description.as_deref(),
        Some("A checkout:\nthe order is created and paid.")
    );
    assert_eq!(descriptions(&outcome), [Some("Who is buying.")]);
}

#[test]
fn a_comment_a_blank_line_away_from_a_later_statement_describes_nothing() {
    let outcome = postgres(
        "SELECT * FROM users;

         -- Left here on its own.

         DELETE FROM orders;",
    );
    assert_eq!(descriptions(&outcome), [None, None]);
    assert_eq!(outcome.value.description, None);
}

#[test]
fn a_comment_after_a_semicolon_is_not_the_next_statements() {
    let outcome = postgres(
        "SELECT * FROM users; -- note on the select
         DELETE FROM orders;",
    );
    assert_eq!(descriptions(&outcome), [None, None]);
}

#[test]
fn comments_inside_a_statement_describe_nothing() {
    let outcome = postgres(
        "SELECT *
         -- not a description
         FROM users;
         DELETE FROM orders;",
    );
    assert_eq!(descriptions(&outcome), [None, None]);
}

#[test]
fn block_comments_describe_like_line_comments() {
    let outcome = postgres(
        "/*
          * Account closure.
          */

         /* Forget the sessions. */
         DELETE FROM sessions;",
    );
    assert_eq!(
        outcome.value.description.as_deref(),
        Some("Account closure.")
    );
    assert_eq!(descriptions(&outcome), [Some("Forget the sessions.")]);
}

#[test]
fn skipped_statements_keep_the_descriptions_in_line() {
    let outcome = postgres(
        "BEGIN;
         -- Not DML.
         CREATE TABLE t (id int);
         -- Forget the sessions.
         DELETE FROM sessions;
         COMMIT;",
    );
    assert_eq!(descriptions(&outcome), [Some("Forget the sessions.")]);
}

#[test]
fn the_checkout_fixture_is_described() {
    let sql = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/flows/checkout.sql"
    ))
    .unwrap();
    let outcome = postgres(&sql);
    assert!(
        outcome
            .value
            .description
            .as_deref()
            .is_some_and(|d| d.starts_with("A checkout against fixtures/tiny.sql"))
    );
    let described = descriptions(&outcome);
    assert_eq!(
        described.first(),
        Some(&Some("Who is buying, and whether their coupon is valid."))
    );
    assert_eq!(described.last(), Some(&Some("The coupon is single-use.")));
}
