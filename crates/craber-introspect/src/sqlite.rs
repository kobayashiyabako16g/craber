//! SQLite, read through the `pragma_*` table-valued functions.
//!
//! Not through `PRAGMA` statements: a `PRAGMA` cannot take a bind parameter, so
//! reading one table at a time would mean formatting its name into SQL. The
//! table-valued forms — `pragma_table_info(?)` and friends, available since
//! SQLite 3.16 — take the name as a parameter instead.

use craber_core::{
    Column, Diagnostic, Dialect, ForeignKey, Outcome, Schema, Table, TableIndex, TableRef,
};
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::{Connection, Row, SqliteConnection};

use crate::assemble::{self, Tables};
use crate::{Error, connect};

pub(crate) async fn introspect(dsn: &str) -> Result<Outcome<Schema>, Error> {
    let options = connect::options::<SqliteConnectOptions>(dsn, Dialect::Sqlite)?.read_only(true);

    // The file's own stem names the diagram, so `--dsn sqlite://shop.db` and
    // `--file shop.sql` title it the same way.
    let name = options
        .get_filename()
        .file_stem()
        .and_then(|stem| stem.to_str())
        .filter(|stem| !stem.is_empty())
        .unwrap_or("memory")
        .to_owned();

    let mut conn = connect::connect(&options).await?;

    let mut diagnostics = Vec::new();
    let names = read_table_names(&mut conn).await?;

    let mut tables = Tables::default();
    for table_name in &names {
        let mut table = Table::new(table_name.clone());
        read_columns(&mut conn, &mut table).await?;
        read_indexes(&mut conn, &mut table).await?;
        tables.push(table);
    }
    // Foreign keys come second, so a key that omits its referenced columns can
    // be resolved against the other table's primary key.
    read_foreign_keys(&mut conn, &names, &mut tables, &mut diagnostics).await?;

    conn.close().await?;
    // Neither comments nor named enum types exist in SQLite. That is the dialect,
    // not something that went wrong, so it produces no diagnostic.
    Ok(assemble::finish(name, tables, Vec::new(), diagnostics))
}

/// `sqlite_*` is the engine's own bookkeeping — `sqlite_sequence` most often —
/// and `craber-parser` skips it by the same rule. Left in, it lands a typeless
/// node in the middle of the diagram.
async fn read_table_names(conn: &mut SqliteConnection) -> Result<Vec<String>, Error> {
    Ok(sqlx::query_scalar(
        r"SELECT name FROM sqlite_master
           WHERE type = 'table' AND name NOT LIKE 'sqlite\_%' ESCAPE '\'
           ORDER BY name",
    )
    .fetch_all(&mut *conn)
    .await?)
}

async fn read_columns(conn: &mut SqliteConnection, table: &mut Table) -> Result<(), Error> {
    let rows =
        sqlx::query("SELECT name, type, \"notnull\", dflt_value, pk FROM pragma_table_info(?)")
            .bind(&table.name)
            .fetch_all(&mut *conn)
            .await?;

    // `pk` is the column's 1-based position within the primary key, 0 when it is
    // not part of one — so a composite key keeps its declared order.
    let mut key: Vec<(i64, String)> = Vec::new();
    for row in rows {
        let name: String = row.get("name");
        let mut column = Column::new(name.clone(), row.get::<String, _>("type"));
        column.nullable = row.get::<i64, _>("notnull") == 0;
        column.default = row.get("dflt_value");
        table.columns.push(column);

        let position: i64 = row.get("pk");
        if position > 0 {
            key.push((position, name));
        }
    }
    key.sort_unstable();
    table.pk = key.into_iter().map(|(_, name)| name).collect();
    Ok(())
}

async fn read_indexes(conn: &mut SqliteConnection, table: &mut Table) -> Result<(), Error> {
    let indexes = sqlx::query(
        // `origin = 'pk'` is the index behind the primary key, which is already
        // in `Table::pk` and which the parser never reports as an index.
        "SELECT name, \"unique\", partial FROM pragma_index_list(?)
          WHERE origin <> 'pk' ORDER BY name",
    )
    .bind(&table.name)
    .fetch_all(&mut *conn)
    .await?;

    for index in indexes {
        let name: String = index.get("name");
        let columns: Vec<Option<String>> =
            sqlx::query_scalar("SELECT name FROM pragma_index_info(?) ORDER BY seqno")
                .bind(&name)
                .fetch_all(&mut *conn)
                .await?;

        // A NULL column name is an indexed expression, with no column row for an
        // edge to attach to.
        if columns.iter().any(Option::is_none) {
            continue;
        }
        table.indexes.push(TableIndex {
            columns: columns.into_iter().flatten().collect(),
            // Same reasoning as PostgreSQL: a partial index constrains only the
            // rows its predicate matches, so the tuple is not unique and
            // `is_unique_tuple` must not say it is.
            unique: index.get::<i64, _>("unique") == 1 && index.get::<i64, _>("partial") == 0,
            name: Some(name),
        });
    }
    Ok(())
}

async fn read_foreign_keys(
    conn: &mut SqliteConnection,
    names: &[String],
    tables: &mut Tables,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<(), Error> {
    for table_name in names {
        let rows = sqlx::query(
            "SELECT id, \"table\", \"from\", \"to\", on_update, on_delete
               FROM pragma_foreign_key_list(?) ORDER BY id, seq",
        )
        .bind(table_name)
        .fetch_all(&mut *conn)
        .await?;

        // `ORDER BY id, seq` means one key's columns arrive together, so the
        // groups are consecutive runs of equal `id` — which `chunk_by` gives
        // without an accumulator to flush at the end. Forgetting that flush
        // dropped the last key of every table, and silently.
        let mut finished: Vec<ForeignKey> = Vec::new();
        for key in rows.chunk_by(|a, b| a.get::<i64, _>("id") == b.get::<i64, _>("id")) {
            let (first, rest) = key.split_first().expect("chunk_by yields no empty group");

            // `to` is NULL when the key omits the referenced column list, in
            // which case it means the other table's primary key.
            // The name stays `None`: PRAGMA does not report the constraint's
            // name, even when the DDL gave it one.
            let mut fk = ForeignKey::declared(
                vec![first.get("from")],
                TableRef::new(first.get::<String, _>("table")),
                first.get::<Option<String>, _>("to").into_iter().collect(),
            );
            for row in rest {
                fk.columns.push(row.get("from"));
                if let Some(to) = row.get::<Option<String>, _>("to") {
                    fk.ref_columns.push(to);
                }
            }
            fk.on_delete = assemble::action_in_words(
                &first.get::<String, _>("on_delete"),
                table_name,
                diagnostics,
            );
            fk.on_update = assemble::action_in_words(
                &first.get::<String, _>("on_update"),
                table_name,
                diagnostics,
            );
            finished.push(fk);
        }

        for mut fk in finished {
            if fk.ref_columns.is_empty() {
                match tables.get(None, &fk.ref_table.name) {
                    Some(referenced) if !referenced.pk.is_empty() => {
                        fk.ref_columns.clone_from(&referenced.pk);
                    }
                    _ => {
                        diagnostics.push(
                            Diagnostic::warning(
                                "foreign key names no columns and its target has no primary key",
                            )
                            .with_context(format!("{table_name} -> {}", fk.ref_table.name)),
                        );
                        continue;
                    }
                }
            }
            if let Some(table) = tables.get_mut(None, table_name) {
                table.fks.push(fk);
            }
        }
    }
    Ok(())
}
