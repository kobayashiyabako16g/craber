//! PostgreSQL introspection, against a real server.
//!
//! Every test here is `#[ignore]`d so that `cargo test --workspace` passes on a
//! machine with no Docker — and reports them as ignored rather than hiding
//! them. They still compile on every run, so clippy keeps linting them; a
//! `#[cfg(feature = ...)]` gate would take them out of `--all-targets` and let
//! them quietly stop building.
//!
//! `just test-db` runs them.

mod common;

use common::{Fk, load_ddl, shape, tiny_sql};
use craber_core::{Action, Cardinality, Dialect, Schema};
use sqlx::PgConnection;
use testcontainers::runners::SyncRunner;
use testcontainers::{Container, ImageExt};
use testcontainers_modules::postgres::Postgres;

/// Pinned rather than `latest`: `format_type`'s rendering and the catalog's
/// contents both move between major versions, and a floating tag would make
/// every assertion here a time bomb.
const IMAGE_TAG: &str = "17-alpine";

struct Database {
    // Held for its `Drop`, which stops the container.
    _container: Container<Postgres>,
    dsn: String,
}

fn start(ddl: &str) -> Database {
    let container = Postgres::default()
        .with_tag(IMAGE_TAG)
        .start()
        .expect("starting PostgreSQL — is the Docker daemon running?");
    let port = container.get_host_port_ipv4(5432).expect("the mapped port");
    let dsn = format!("postgres://postgres:postgres@127.0.0.1:{port}/postgres");

    load_ddl::<PgConnection>(&dsn, ddl);

    Database {
        _container: container,
        dsn,
    }
}

fn introspect(db: &Database) -> Schema {
    let outcome = craber_introspect::introspect(&db.dsn, Dialect::Postgres).expect("introspecting");
    assert!(
        outcome.diagnostics.is_empty(),
        "an ordinary schema should introspect without complaint: {:?}",
        outcome.diagnostics
    );
    outcome.value
}

/// The claim this whole crate exists to support.
///
/// `fixtures/tiny.sql` was built so that its four foreign keys are every
/// combination of "is the key over a unique tuple" and "are its columns
/// nullable" — which is everything a dialect says about a relationship. So if
/// the shape of a live read matches the shape of parsing the same file, every
/// crow's-foot end CrabER can draw has been shown identical from both sources.
#[test]
#[ignore = "needs a Docker daemon"]
fn introspecting_tiny_sql_agrees_with_parsing_it() {
    let db = start(&tiny_sql());
    let live = introspect(&db);
    let parsed = craber_parser::parse(&tiny_sql(), Dialect::Postgres, "tiny").value;

    assert_eq!(shape(&live), shape(&parsed));
}

/// Spelled out rather than left to the comparison above, because these are the
/// ends the notation actually draws and a regression in `annotate_relationships`
/// would move them without changing anything else.
#[test]
#[ignore = "needs a Docker daemon"]
fn every_relationship_end_survives_the_round_trip() {
    let db = start(&tiny_sql());
    let live = introspect(&db);

    let fk = |table: &str, column: &str| -> (Cardinality, bool) {
        let t = live.table(table).expect(table);
        let fk = t
            .fks
            .iter()
            .find(|fk| fk.columns == [column])
            .unwrap_or_else(|| panic!("no foreign key on {table}.{column}"));
        (fk.cardinality, fk.optional)
    };

    // Not unique, NOT NULL: one and only one user, zero or many orders.
    assert_eq!(fk("orders", "user_id"), (Cardinality::ManyToOne, false));
    // Not unique, nullable: zero or one coupon.
    assert_eq!(fk("orders", "coupon_id"), (Cardinality::ManyToOne, true));
    // Unique and NOT NULL: at most one profile, and it has a user.
    assert_eq!(
        fk("user_profiles", "user_id"),
        (Cardinality::OneToOne, false)
    );
    // Unique and nullable: at most one receipt, which may have no order yet.
    assert_eq!(
        fk("order_receipts", "order_id"),
        (Cardinality::OneToOne, true)
    );
}

#[test]
#[ignore = "needs a Docker daemon"]
fn comments_and_referential_actions_come_across() {
    let db = start(&tiny_sql());
    let live = introspect(&db);

    assert_eq!(
        live.table("users").expect("users").comment.as_deref(),
        Some("People who can sign in.")
    );
    assert_eq!(
        live.table("orders")
            .and_then(|t| t.column("total"))
            .expect("orders.total")
            .comment
            .as_deref(),
        Some("Gross total, in the order currency.")
    );

    let coupon = live
        .table("orders")
        .expect("orders")
        .fks
        .iter()
        .find(|fk| fk.columns == ["coupon_id"])
        .expect("the coupon key");
    assert_eq!(coupon.on_delete, Action::SetNull);
}

/// The places a live database legitimately knows more than the DDL that built
/// it, or spells something differently. Asserted rather than waved at, so that
/// closing one of them is a deliberate change with a failing test behind it.
#[test]
#[ignore = "needs a Docker daemon"]
fn the_known_divergences_are_these_and_only_these() {
    let db = start(&tiny_sql());
    let live = introspect(&db);
    let parsed = craber_parser::parse(&tiny_sql(), Dialect::Postgres, "tiny").value;

    let data_type = |schema: &Schema, table: &str, column: &str| {
        schema
            .table(table)
            .and_then(|t| t.column(column))
            .unwrap_or_else(|| panic!("{table}.{column}"))
            .data_type
            .clone()
    };
    // The catalog's rendering, against sqlparser's. The IR's own documentation
    // is on the catalog's side of this; reconciling them means one canonical
    // spelling shared by both crates, and every snapshot in the repository
    // moving at once.
    assert_eq!(data_type(&live, "orders", "total"), "numeric(12,2)");
    assert_eq!(data_type(&parsed, "orders", "total"), "NUMERIC(12,2)");
    assert_eq!(
        data_type(&live, "users", "created_at"),
        "timestamp with time zone"
    );
    assert_eq!(data_type(&parsed, "users", "created_at"), "TIMESTAMPTZ");

    // A default keeps the cast the server records for it.
    let default = |schema: &Schema, table: &str, column: &str| {
        schema
            .table(table)
            .and_then(|t| t.column(column))
            .unwrap_or_else(|| panic!("{table}.{column}"))
            .default
            .clone()
    };
    assert_eq!(
        default(&live, "orders", "status").as_deref(),
        Some("'pending'::public.order_status")
    );
    assert_eq!(
        default(&parsed, "orders", "status").as_deref(),
        Some("'pending'")
    );

    // An inline UNIQUE gets a server-generated name the file never had.
    let index =
        |schema: &Schema, table: &str| schema.table(table).expect(table).indexes[0].name.clone();
    assert_eq!(index(&live, "users").as_deref(), Some("users_email_key"));
    assert_eq!(index(&parsed, "users"), None);

    // And the one that is not about a field at all: a dump carries the order
    // its DDL was written in and a catalog does not. `craber-layout` breaks
    // ties within a layer by the order it was handed the tables, so this is
    // what makes the two sources stack the same graph differently — identical
    // layers and edges, different rows inside a layer.
    assert_eq!(
        live.tables.iter().map(|t| &t.name).collect::<Vec<_>>(),
        [
            "coupons",
            "order_items",
            "order_receipts",
            "orders",
            "user_profiles",
            "users"
        ],
        "a catalog read is ordered by name"
    );
    assert_eq!(
        parsed.tables.iter().map(|t| &t.name).collect::<Vec<_>>(),
        [
            "users",
            "user_profiles",
            "coupons",
            "orders",
            "order_receipts",
            "order_items"
        ],
        "a parsed dump keeps the order its DDL declared"
    );
}

/// An empty `search_path` is what makes `format_type` qualify every type it
/// renders. Left at its default the same database describes itself differently
/// depending on the connecting role, which is the kind of failure that gets
/// blamed on the container.
#[test]
#[ignore = "needs a Docker daemon"]
fn a_type_is_schema_qualified_the_way_the_parser_writes_it() {
    let db = start(&tiny_sql());
    let live = introspect(&db);
    assert_eq!(
        live.table("orders")
            .and_then(|t| t.column("status"))
            .expect("orders.status")
            .data_type,
        "public.order_status"
    );
}

/// The trap that draws a wrong diagram rather than raising an error: a partial
/// unique index does not make the tuple unique, so a key over it is still
/// many-to-one. `tiny.sql` has no partial index, so this schema is its own.
#[test]
#[ignore = "needs a Docker daemon"]
fn a_partial_unique_index_does_not_make_a_key_one_to_one() {
    let db = start(
        "CREATE TABLE accounts (id bigint PRIMARY KEY);
         CREATE TABLE sessions (
             account_id bigint NOT NULL REFERENCES accounts (id),
             revoked_at timestamptz
         );
         CREATE UNIQUE INDEX sessions_live_idx
             ON sessions (account_id) WHERE revoked_at IS NULL;",
    );
    let live = introspect(&db);
    let sessions = live.table("sessions").expect("sessions");

    assert!(!sessions.indexes[0].unique, "{:?}", sessions.indexes);
    assert_eq!(sessions.fks[0].cardinality, Cardinality::ManyToOne);
}

/// Views have no primary key and no foreign keys, and `craber-parser` drops
/// `CREATE VIEW` outright — a connection that volunteered them would disagree
/// with every dump of the same database.
#[test]
#[ignore = "needs a Docker daemon"]
fn views_are_not_tables() {
    let db = start(
        "CREATE TABLE people (id bigint PRIMARY KEY, name text);
         CREATE VIEW named_people AS SELECT id, name FROM people WHERE name IS NOT NULL;",
    );
    let live = introspect(&db);
    let names: Vec<&str> = live.tables.iter().map(|t| t.name.as_str()).collect();
    assert_eq!(names, ["people"]);
}

/// A key the diagram needs but the DDL declared in pieces: the composite
/// foreign key has to arrive with its columns paired in the declared order, or
/// the edge lands on the wrong rows.
#[test]
#[ignore = "needs a Docker daemon"]
fn a_composite_foreign_key_keeps_its_pairing() {
    let db = start(
        "CREATE TABLE tags (kind text NOT NULL, slug text NOT NULL, PRIMARY KEY (kind, slug));
         CREATE TABLE taggings (
             id bigint PRIMARY KEY,
             kind text NOT NULL,
             slug text NOT NULL,
             FOREIGN KEY (kind, slug) REFERENCES tags (kind, slug) ON UPDATE CASCADE
         );",
    );
    let live = introspect(&db);
    let taggings = live.table("taggings").expect("taggings");
    assert_eq!(
        taggings.fks.first().map(|fk| Fk {
            columns: fk.columns.clone(),
            ref_table: fk.ref_table.name.clone(),
            ref_columns: fk.ref_columns.clone(),
            on_delete: fk.on_delete,
            on_update: fk.on_update,
            cardinality: fk.cardinality,
            optional: fk.optional,
        }),
        Some(Fk {
            columns: vec!["kind".into(), "slug".into()],
            ref_table: "tags".into(),
            ref_columns: vec!["kind".into(), "slug".into()],
            on_delete: Action::NoAction,
            on_update: Action::Cascade,
            cardinality: Cardinality::ManyToOne,
            optional: false,
        })
    );
}
