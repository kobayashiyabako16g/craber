//! SQLite introspection, against a database this test builds.
//!
//! No `#[ignore]` on anything here: SQLite is a file, so these run in the
//! ordinary `cargo test --workspace` and are the fast feedback loop for the
//! translation layer that the PostgreSQL and MySQL suites can only get from
//! Docker.

mod common;

use std::path::PathBuf;

use craber_core::{Cardinality, Dialect, Schema};
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::{Connection, SqliteConnection};

const DDL: &str = r"
CREATE TABLE users (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    email      TEXT NOT NULL,
    deleted_at TEXT
);
CREATE UNIQUE INDEX users_email_idx ON users (email);
-- Partial: rows outside the predicate are unconstrained, so the tuple is not
-- unique and a key over it is still many-to-one.
CREATE UNIQUE INDEX users_email_live_idx ON users (email) WHERE deleted_at IS NULL;
CREATE INDEX users_lower_email_idx ON users (lower(email));

CREATE TABLE posts (
    id      INTEGER PRIMARY KEY,
    -- No column list: the reference is to whatever users' primary key is.
    user_id INTEGER NOT NULL REFERENCES users ON DELETE CASCADE,
    title   TEXT NOT NULL DEFAULT ''
);

CREATE TABLE tags (
    kind TEXT NOT NULL,
    slug TEXT NOT NULL,
    PRIMARY KEY (kind, slug)
);

CREATE TABLE post_tags (
    post_id  INTEGER NOT NULL REFERENCES posts (id) ON DELETE CASCADE,
    kind     TEXT NOT NULL,
    slug     TEXT NOT NULL,
    PRIMARY KEY (post_id, kind, slug),
    FOREIGN KEY (kind, slug) REFERENCES tags (kind, slug) ON UPDATE RESTRICT
);
";

fn database(name: &str) -> Schema {
    let path: PathBuf = std::env::temp_dir().join(format!(
        "craber-introspect-{name}-{}.db",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&path);

    common::block_on(async {
        let options = SqliteConnectOptions::new()
            .filename(&path)
            .create_if_missing(true);
        let mut conn = SqliteConnection::connect_with(&options)
            .await
            .expect("creating the database");
        sqlx::raw_sql(DDL)
            .execute(&mut conn)
            .await
            .expect("loading the DDL");
        conn.close().await.expect("closing");
    });

    let dsn = format!("sqlite://{}", path.display());
    let outcome = craber_introspect::introspect(&dsn, Dialect::Sqlite).expect("introspecting");
    assert!(
        outcome.diagnostics.is_empty(),
        "an ordinary schema should introspect without complaint: {:?}",
        outcome.diagnostics
    );
    let _ = std::fs::remove_file(&path);
    outcome.value
}

/// `sqlite_sequence` exists in this database because of `AUTOINCREMENT`, and it
/// is the engine's bookkeeping rather than anything a diagram should draw —
/// `craber-parser` skips it by the same rule.
#[test]
fn the_engines_own_tables_stay_out() {
    let schema = database("tables");
    let names: Vec<&str> = schema.tables.iter().map(|t| t.name.as_str()).collect();
    assert_eq!(names, ["post_tags", "posts", "tags", "users"]);
}

/// SQLite's namespace is the file, so there is nothing to qualify a table with —
/// the same `None` the parser records for a `sqlite3 .schema` dump.
#[test]
fn tables_are_not_namespaced() {
    let schema = database("namespace");
    assert!(schema.tables.iter().all(|t| t.schema.is_none()));
}

#[test]
fn columns_keep_their_type_nullability_and_default() {
    let schema = database("columns");
    let posts = schema.table("posts").expect("posts");

    let title = posts.column("title").expect("title");
    assert_eq!(title.data_type, "TEXT");
    assert!(!title.nullable);
    assert_eq!(title.default.as_deref(), Some("''"));

    let deleted_at = schema
        .table("users")
        .and_then(|t| t.column("deleted_at"))
        .expect("users.deleted_at");
    assert!(deleted_at.nullable);
    assert_eq!(deleted_at.default, None);
}

#[test]
fn a_composite_primary_key_keeps_its_order() {
    let schema = database("composite");
    assert_eq!(schema.table("tags").expect("tags").pk, ["kind", "slug"]);
    assert_eq!(
        schema.table("post_tags").expect("post_tags").pk,
        ["post_id", "kind", "slug"]
    );
}

/// `REFERENCES users` with no column list means users' primary key, and the
/// diagram needs the column name to anchor the edge to a row.
#[test]
fn a_key_without_a_column_list_resolves_to_the_primary_key() {
    let schema = database("implicit");
    let fk = &schema.table("posts").expect("posts").fks[0];
    assert_eq!(fk.columns, ["user_id"]);
    assert_eq!(fk.ref_table.name, "users");
    assert_eq!(fk.ref_columns, ["id"]);
    assert_eq!(fk.on_delete, craber_core::Action::Cascade);
}

#[test]
fn a_composite_key_arrives_whole() {
    let schema = database("composite-fk");
    let post_tags = schema.table("post_tags").expect("post_tags");
    let fk = post_tags
        .fks
        .iter()
        .find(|fk| fk.ref_table.name == "tags")
        .expect("the key into tags");
    assert_eq!(fk.columns, ["kind", "slug"]);
    assert_eq!(fk.ref_columns, ["kind", "slug"]);
    assert_eq!(fk.on_update, craber_core::Action::Restrict);
}

/// The index behind the primary key is not one the parser reports, so this must
/// not report it either — every table would otherwise gain a unique index it
/// does not have, and `is_unique_tuple` would start drawing one-to-one.
#[test]
fn the_primary_keys_own_index_is_not_listed() {
    let schema = database("pk-index");
    let tags = schema.table("tags").expect("tags");
    assert!(tags.indexes.is_empty(), "{:?}", tags.indexes);
}

/// An index over `lower(email)` has no column row for an edge to sit on, which
/// is why the parser drops one too.
#[test]
fn an_expression_index_is_dropped() {
    let schema = database("expression");
    let users = schema.table("users").expect("users");
    let names: Vec<&str> = users
        .indexes
        .iter()
        .filter_map(|i| i.name.as_deref())
        .collect();
    assert_eq!(names, ["users_email_idx", "users_email_live_idx"]);
}

/// The failure this guards against draws a wrong diagram rather than raising an
/// error: a partial unique index does not make the tuple unique, and recording
/// it as unique would put a one-to-one crow's foot on a key that is not.
#[test]
fn a_partial_unique_index_is_not_unique() {
    let schema = database("partial");
    let users = schema.table("users").expect("users");
    let partial = users
        .indexes
        .iter()
        .find(|i| i.name.as_deref() == Some("users_email_live_idx"))
        .expect("the partial index");
    assert!(!partial.unique);

    let full = users
        .indexes
        .iter()
        .find(|i| i.name.as_deref() == Some("users_email_idx"))
        .expect("the whole-table index");
    assert!(full.unique);
}

/// `annotate_relationships` is what fills these in, and a backend that forgot to
/// call it would leave every relationship reading many-to-one.
#[test]
fn relationships_are_annotated() {
    let schema = database("annotate");
    let posts = schema.table("posts").expect("posts");
    assert_eq!(posts.fks[0].cardinality, Cardinality::ManyToOne);
    assert!(!posts.fks[0].optional, "posts.user_id is NOT NULL");
    assert!(
        !schema
            .table("users")
            .and_then(|t| t.column("id"))
            .expect("users.id")
            .nullable,
        "a primary key column is NOT NULL whether or not the DDL said so"
    );
}
