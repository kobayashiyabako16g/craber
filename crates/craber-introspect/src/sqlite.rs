//! SQLite, read through the `pragma_*` table-valued functions.
//!
//! Not through `PRAGMA` statements: a `PRAGMA` cannot take a bind parameter, so
//! reading one table at a time would mean formatting its name into SQL. The
//! table-valued forms — `pragma_table_info(?)` and friends, available since
//! SQLite 3.16 — take the name as a parameter instead.

use craber_core::{
    Action, Column, Dialect, ForeignKey, Outcome, Schema, Table, TableIndex, TableRef,
};
use sqlx::sqlite::{SqliteConnectOptions, SqliteRow};
use sqlx::{Connection, Row, SqliteConnection};

use crate::assemble::{self, Catalog};
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

    let mut catalog = Catalog::default();
    for table_name in read_table_names(&mut conn).await? {
        let mut table = Table::new(table_name);
        read_columns(&mut conn, &mut table).await?;
        read_indexes(&mut conn, &mut table).await?;
        read_foreign_keys(&mut conn, &mut table, &mut catalog).await?;
        catalog.push(table);
    }

    conn.close().await?;
    // Neither comments nor named enum types exist in SQLite. That is the dialect,
    // not something that went wrong, so it produces no diagnostic.
    Ok(catalog.finish(name))
}

/// The engine's own tables are left out by [`Dialect::is_engine_table`], the
/// rule `craber-parser` skips them by.
async fn read_table_names(conn: &mut SqliteConnection) -> Result<Vec<String>, Error> {
    let names: Vec<String> =
        sqlx::query_scalar("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name")
            .fetch_all(&mut *conn)
            .await?;
    Ok(names
        .into_iter()
        .filter(|name| !Dialect::Sqlite.is_engine_table(name))
        .collect())
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
        let name: String = row.try_get("name")?;
        let mut column = Column::new(name.clone(), row.try_get::<String, _>("type")?);
        column.nullable = row.try_get::<i64, _>("notnull")? == 0;
        column.default = row.try_get("dflt_value")?;
        table.columns.push(column);

        let position: i64 = row.try_get("pk")?;
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
        let name: String = index.try_get("name")?;
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
            unique: index.try_get::<i64, _>("unique")? == 1
                && index.try_get::<i64, _>("partial")? == 0,
            name: Some(name),
        });
    }
    Ok(())
}

/// The keys `table` declares, onto `table`.
///
/// The name stays `None`: PRAGMA does not report a constraint's name, even when
/// the DDL gave it one. A `to` of NULL means the key omits the referenced
/// column list; `normalize` fills in the primary key, for files
/// as well.
async fn read_foreign_keys(
    conn: &mut SqliteConnection,
    table: &mut Table,
    catalog: &mut Catalog,
) -> Result<(), Error> {
    let rows = sqlx::query(
        "SELECT id, \"table\", \"from\", \"to\", on_update, on_delete
           FROM pragma_foreign_key_list(?) ORDER BY id, seq",
    )
    .bind(&table.name)
    .fetch_all(&mut *conn)
    .await?;

    // `ORDER BY id, seq` means one key's columns arrive together, so the
    // groups are consecutive runs of equal `id` — which `chunk_by` gives
    // without an accumulator to flush at the end. Forgetting that flush
    // dropped the last key of every table, and silently.
    let rows = rows
        .into_iter()
        .map(|row| Ok((row.try_get::<i64, _>("id")?, row)))
        .collect::<Result<Vec<(i64, SqliteRow)>, sqlx::Error>>()?;
    for key in rows.chunk_by(|a, b| a.0 == b.0) {
        let (id, first) = &key[0];
        let ref_table: String = first.try_get("table")?;
        // No name to give it, so the key is told apart by its position.
        let context = assemble::context(&table.to_ref(), &format!("fk{id}"));

        let mut columns = Vec::with_capacity(key.len());
        let mut ref_columns = Vec::with_capacity(key.len());
        for (_, row) in key {
            columns.push(row.try_get("from")?);
            if let Some(to) = row.try_get::<Option<String>, _>("to")? {
                ref_columns.push(to);
            }
        }
        let on_delete = first.try_get::<String, _>("on_delete")?;
        let on_update = first.try_get::<String, _>("on_update")?;
        table.fks.push(
            ForeignKey::declared(columns, TableRef::new(ref_table), ref_columns).with_actions(
                catalog.action(&on_delete, &context, Action::from_sql),
                catalog.action(&on_update, &context, Action::from_sql),
            ),
        );
    }
    Ok(())
}
