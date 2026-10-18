//! MySQL and SQLite input, in the shape a real dump has.
//!
//! `examples/03-dialects` is hand-written DDL and stays readable on purpose, so
//! it never exercises the things a `mysqldump` or a `sqlite3 .schema` puts in
//! every file: version comments, DROP/LOCK scaffolding, a `KEY` line per index,
//! `sqlite_sequence`. The fixtures here do, and what they mostly assert is that
//! stderr stays empty — the failure this guards against was CrabER warning
//! about ordinary dump syntax, one line per index, which is how a real warning
//! gets lost.

mod common;

use common::{assert_success, column, craber, fixture, table, table_names};

fn export(name: &str) -> serde_json::Value {
    let output = craber(&["export", "--file", fixture(name).to_str().unwrap()]);
    assert_success(&output, &format!("exporting {name}"));
    assert_eq!(
        String::from_utf8_lossy(&output.stderr).trim(),
        "",
        "{name} is an ordinary dump and should parse cleanly"
    );
    serde_json::from_slice(&output.stdout).expect("valid JSON")
}

/// No `--dialect` anywhere here: the fixture names carry it, the same way the
/// commands in `examples/03-dialects/README.md` rely on.
#[test]
fn a_mysqldump_parses_without_complaint() {
    let schema = export("dump.mysql.sql");
    assert_eq!(table_names(&schema), ["users", "posts", "post_tags"]);
}

#[test]
fn mysql_indexes_and_comments_reach_the_diagram() {
    let schema = export("dump.mysql.sql");
    let users = table(&schema, "users");

    assert_eq!(
        users["comment"].as_str(),
        Some("One row per person who can sign in."),
        "the table comment rides in the trailing table options, not a COMMENT clause"
    );
    assert_eq!(
        column(users, "bio")["comment"].as_str(),
        Some("free-form profile text")
    );

    let indexes = users["indexes"].as_array().expect("indexes");
    let named = |name: &str| {
        indexes
            .iter()
            .find(|index| index["name"] == name)
            .unwrap_or_else(|| panic!("no index named {name}"))
    };
    assert_eq!(named("users_email_uniq")["unique"], true);
    assert_eq!(named("users_role_idx")["unique"], false);
    // FULLTEXT KEY is an access path with nothing to draw, so it is the one
    // key in the fixture that should not appear.
    assert_eq!(indexes.len(), 2, "{indexes:#?}");

    // An index added by a trailing ALTER TABLE lands like any other.
    let posts = table(&schema, "posts");
    assert!(
        posts["indexes"]
            .as_array()
            .expect("indexes")
            .iter()
            .any(|index| index["name"] == "posts_published_idx"),
        "{:#?}",
        posts["indexes"]
    );
}

#[test]
fn a_sqlite_schema_dump_parses_without_complaint() {
    let schema = export("dump.sqlite.sql");
    assert!(
        !table_names(&schema).contains(&"sqlite_sequence"),
        "SQLite's own bookkeeping table is not part of the model: {:?}",
        table_names(&schema)
    );
}

#[test]
fn sqlite_inline_references_become_edges() {
    let schema = export("dump.sqlite.sql");
    let books = table(&schema, "books");
    let fks = books["fks"].as_array().expect("fks");

    // A table-level CONSTRAINT ... FOREIGN KEY with DEFERRABLE after it, and a
    // column-level REFERENCES back into the same table.
    let named = fks
        .iter()
        .find(|fk| fk["name"] == "fk_rails_53d51ce16a")
        .expect("the named foreign key");
    assert_eq!(named["refTable"]["name"], "authors");

    let self_ref = fks
        .iter()
        .find(|fk| fk["columns"][0] == "sequel_to")
        .expect("the self-reference");
    assert_eq!(self_ref["refTable"]["name"], "books");
    assert_eq!(self_ref["optional"], true);
}
