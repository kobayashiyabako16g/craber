//! Helpers shared by the database-backed introspection tests.
//!
//! Cargo builds each integration test as its own crate, so this module is
//! compiled once per file and every file uses only part of it — hence the
//! blanket `dead_code` allow, which is the conventional shape for a
//! `tests/common` module.
#![allow(dead_code)]

use std::path::PathBuf;

use craber_core::{Action, Cardinality, Schema};

/// Run a future to completion on a runtime of this test's own.
///
/// The crate under test is synchronous, so the tests have no runtime handed to
/// them and no reason to be `async` themselves.
pub fn block_on<F: std::future::Future>(future: F) -> F::Output {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("a runtime")
        .block_on(future)
}

/// Run `ddl` against a freshly started server.
///
/// The fixture is read from this repository at runtime, so it is not the
/// `&'static str` sqlx wants for a query it has not been told is safe.
/// Asserting it here is the whole of that promise, and it is written once: the
/// only callers are the suites below, with SQL out of `fixtures/`.
pub fn load_ddl<C>(dsn: &str, ddl: &str)
where
    C: sqlx::Connection,
    for<'c> &'c mut C: sqlx::Executor<'c, Database = C::Database>,
{
    block_on(async {
        let mut conn = C::connect(dsn).await.expect("connecting");
        sqlx::raw_sql(sqlx::AssertSqlSafe(ddl.to_owned()))
            .execute(&mut conn)
            .await
            .expect("loading the DDL");
        conn.close().await.expect("closing");
    });
}

/// A schema in `fixtures/`, by filename.
pub fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .join(name)
}

/// `fixtures/tiny.sql` with its trailing statement removed.
///
/// That statement is unapplied on purpose — it is what keeps the parser's
/// warn-and-continue path covered — and a real PostgreSQL rejects it, which
/// with an implicit transaction around the batch would roll the whole schema
/// back. The marker comment above it is load-bearing; `fixtures/README.md`
/// says so.
pub fn tiny_sql() -> String {
    const MARKER: &str = "-- Not applied:";
    let sql = std::fs::read_to_string(fixture("tiny.sql")).expect("reading tiny.sql");
    let at = sql
        .find(MARKER)
        .expect("tiny.sql should still carry its `-- Not applied:` marker");
    sql[..at].to_owned()
}

/// What the diagram is actually made of.
///
/// Deliberately not `data_type` and not index names: a live database
/// legitimately spells the first differently and knows the second, and the
/// crate documents both. Everything here is a fact the drawing depends on —
/// which is the claim worth asserting about two sources for the same schema.
///
/// **Tables are sorted by name, which is itself one of the divergences.** A
/// dump carries the order its DDL was written in and a catalog does not, so
/// the two sources hand `craber-layout` the same tables in a different order
/// and it stacks them differently within a layer. Sorting here is what lets
/// the rest of the comparison be about the schema rather than about that;
/// `the_known_divergences_are_these_and_only_these` asserts the ordering
/// difference outright so it stays visible.
pub type Shape = Vec<Table>;

#[derive(Debug, PartialEq, Eq)]
pub struct Table {
    pub name: String,
    pub schema: Option<String>,
    pub comment: Option<String>,
    pub columns: Vec<(String, bool, Option<String>)>,
    pub pk: Vec<String>,
    pub fks: Vec<Fk>,
    pub indexes: Vec<(Vec<String>, bool)>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Fk {
    pub columns: Vec<String>,
    pub ref_table: String,
    pub ref_columns: Vec<String>,
    pub on_delete: Action,
    pub on_update: Action,
    pub cardinality: Cardinality,
    pub optional: bool,
}

pub fn shape(schema: &Schema) -> Shape {
    let mut tables: Vec<Table> = schema
        .tables
        .iter()
        .map(|t| {
            let mut fks: Vec<Fk> = t
                .fks
                .iter()
                .map(|fk| Fk {
                    columns: fk.columns.clone(),
                    ref_table: fk.ref_table.name.clone(),
                    ref_columns: fk.ref_columns.clone(),
                    on_delete: fk.on_delete,
                    on_update: fk.on_update,
                    cardinality: fk.cardinality,
                    optional: fk.optional,
                })
                .collect();
            // Sorted by the names alone: `Action` and `Cardinality` are wire
            // format types and do not order, and the columns already identify
            // a key uniquely within its table.
            fks.sort_by(|a, b| {
                (&a.columns, &a.ref_table, &a.ref_columns).cmp(&(
                    &b.columns,
                    &b.ref_table,
                    &b.ref_columns,
                ))
            });

            let mut indexes: Vec<(Vec<String>, bool)> = t
                .indexes
                .iter()
                .map(|i| (i.columns.clone(), i.unique))
                .collect();
            indexes.sort();

            Table {
                name: t.name.clone(),
                schema: t.schema.clone(),
                comment: t.comment.clone(),
                columns: t
                    .columns
                    .iter()
                    .map(|c| (c.name.clone(), c.nullable, c.comment.clone()))
                    .collect(),
                pk: t.pk.clone(),
                fks,
                indexes,
            }
        })
        .collect();
    tables.sort_by(|a, b| a.name.cmp(&b.name));
    tables
}
