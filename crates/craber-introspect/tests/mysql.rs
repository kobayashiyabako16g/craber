//! MySQL introspection, against a real server.
//!
//! `#[ignore]`d for the same reason as the PostgreSQL suite: `cargo test
//! --workspace` has to pass without a Docker daemon, while these still compile
//! on every run so clippy keeps them honest. `just test-db` runs them.

mod common;

use common::{fixture, load_ddl, shape};
use craber_core::{Action, Cardinality, Dialect, Schema};
use sqlx::MySqlConnection;
use testcontainers::runners::SyncRunner;
use testcontainers::{Container, ImageExt};
use testcontainers_modules::mysql::Mysql;

/// Pinned: `information_schema`'s contents and MySQL's default quoting both
/// move between versions, and a floating tag would make these assertions a
/// time bomb.
const IMAGE_TAG: &str = "8.4";

struct Database {
    _container: Container<Mysql>,
    dsn: String,
}

fn start(ddl: &str) -> Database {
    let container = Mysql::default()
        .with_tag(IMAGE_TAG)
        .start()
        .expect("starting MySQL — is the Docker daemon running?");
    let port = container.get_host_port_ipv4(3306).expect("the mapped port");
    let dsn = format!("mysql://root@127.0.0.1:{port}/test");

    load_ddl::<MySqlConnection>(&dsn, ddl);

    Database {
        _container: container,
        dsn,
    }
}

fn dump() -> String {
    std::fs::read_to_string(fixture("dump.mysql.sql")).expect("reading dump.mysql.sql")
}

fn introspect(db: &Database) -> Schema {
    let outcome = craber_introspect::introspect(&db.dsn, Dialect::MySql).expect("introspecting");
    assert!(
        outcome.diagnostics.is_empty(),
        "an ordinary dump should introspect without complaint: {:?}",
        outcome.diagnostics
    );
    outcome.value
}

/// The same claim the PostgreSQL suite makes, against the dump shape a real
/// `mysqldump` produces.
#[test]
#[ignore = "needs a Docker daemon"]
fn introspecting_the_dump_agrees_with_parsing_it() {
    let db = start(&dump());
    let live = introspect(&db);
    let parsed = craber_parser::parse(&dump(), Dialect::MySql, "dump").value;

    assert_eq!(shape(&live), shape(&parsed));
}

/// MySQL's namespace is the database, and the parser leaves a dump's tables
/// unqualified. Putting the database name in `Table::schema` would make every
/// `--include` glob disagree between a file and a connection.
#[test]
#[ignore = "needs a Docker daemon"]
fn tables_are_not_qualified_with_the_database() {
    let db = start(&dump());
    let live = introspect(&db);
    assert_eq!(live.name, "test");
    assert!(live.tables.iter().all(|t| t.schema.is_none()));
}

/// `COLUMN_TYPE`, not `DATA_TYPE`: the latter flattens these to `enum` and
/// `bigint` and throws away what the details panel is for.
#[test]
#[ignore = "needs a Docker daemon"]
fn a_column_keeps_the_full_type_the_dump_declared() {
    let db = start(&dump());
    let live = introspect(&db);
    let users = live.table("users").expect("users");

    assert_eq!(
        users.column("role").expect("role").data_type,
        "enum('admin','member','guest')"
    );
    assert_eq!(users.column("id").expect("id").data_type, "bigint unsigned");
}

/// The IR holds a default as an expression, which is what the parser records.
/// MySQL stores a literal with its quotes stripped, and `EXTRA` is what tells
/// a literal apart from an expression.
#[test]
#[ignore = "needs a Docker daemon"]
fn a_literal_default_is_quoted_back_into_an_expression() {
    let db = start(&dump());
    let live = introspect(&db);
    let parsed = craber_parser::parse(&dump(), Dialect::MySql, "dump").value;

    let default = |schema: &Schema, column: &str| {
        schema
            .table("users")
            .and_then(|t| t.column(column))
            .unwrap_or_else(|| panic!("users.{column}"))
            .default
            .clone()
    };

    assert_eq!(default(&live, "role").as_deref(), Some("'member'"));
    assert_eq!(default(&parsed, "role").as_deref(), Some("'member'"));

    // An expression default is left alone — `CURRENT_TIMESTAMP` is a function
    // call, not a string that wants quoting.
    assert_eq!(
        default(&live, "created_at").as_deref(),
        Some("CURRENT_TIMESTAMP")
    );
}

/// A `FULLTEXT KEY` is not a key an ER diagram draws, and the parser reports
/// none for the same line — `crates/craber/tests/dialects.rs` pins that to
/// exactly two indexes on this table.
#[test]
#[ignore = "needs a Docker daemon"]
fn fulltext_indexes_stay_out() {
    let db = start(&dump());
    let live = introspect(&db);
    let users = live.table("users").expect("users");

    let names: Vec<&str> = users
        .indexes
        .iter()
        .filter_map(|i| i.name.as_deref())
        .collect();
    assert_eq!(names, ["users_email_uniq", "users_role_idx"]);
    assert_eq!(users.pk, ["id"]);
}

#[test]
#[ignore = "needs a Docker daemon"]
fn comments_and_referential_actions_come_across() {
    let db = start(&dump());
    let live = introspect(&db);

    assert_eq!(
        live.table("users").expect("users").comment.as_deref(),
        Some("One row per person who can sign in.")
    );
    assert_eq!(
        live.table("posts")
            .and_then(|t| t.column("user_id"))
            .expect("posts.user_id")
            .comment
            .as_deref(),
        Some("author")
    );

    let fk = &live.table("posts").expect("posts").fks[0];
    assert_eq!(fk.columns, ["user_id"]);
    assert_eq!(fk.ref_table.name, "users");
    assert_eq!(fk.ref_columns, ["id"]);
    assert_eq!(fk.on_delete, Action::Cascade);
    assert_eq!(fk.cardinality, Cardinality::ManyToOne);
    assert!(!fk.optional, "posts.user_id is NOT NULL");
}

#[test]
#[ignore = "needs a Docker daemon"]
fn a_composite_key_keeps_its_pairing() {
    let db = start(
        "CREATE TABLE tags (
             kind varchar(32) NOT NULL,
             slug varchar(64) NOT NULL,
             PRIMARY KEY (kind, slug)
         ) ENGINE=InnoDB;
         CREATE TABLE taggings (
             id bigint NOT NULL PRIMARY KEY,
             kind varchar(32) NOT NULL,
             slug varchar(64) NOT NULL,
             CONSTRAINT taggings_tag_fk FOREIGN KEY (kind, slug)
                 REFERENCES tags (kind, slug) ON UPDATE CASCADE
         ) ENGINE=InnoDB;",
    );
    let live = introspect(&db);
    let fk = &live.table("taggings").expect("taggings").fks[0];

    assert_eq!(fk.columns, ["kind", "slug"]);
    assert_eq!(fk.ref_columns, ["kind", "slug"]);
    assert_eq!(fk.on_update, Action::Cascade);
}

/// Views are dropped for the same reason as in PostgreSQL: `craber-parser`
/// never produces one, so a connection that did would disagree with every dump.
#[test]
#[ignore = "needs a Docker daemon"]
fn views_are_not_tables() {
    let db = start(
        "CREATE TABLE people (id bigint NOT NULL PRIMARY KEY, name varchar(64)) ENGINE=InnoDB;
         CREATE VIEW named_people AS SELECT id, name FROM people WHERE name IS NOT NULL;",
    );
    let live = introspect(&db);
    let names: Vec<&str> = live.tables.iter().map(|t| t.name.as_str()).collect();
    assert_eq!(names, ["people"]);
}
