//! MySQL and MariaDB, read from `information_schema`.
//!
//! Everything is scoped to `DATABASE()`. A connection string names exactly one
//! database, and `information_schema` spans the whole server — an unscoped query
//! would put another tenant's tables in the diagram.
//!
//! Every text column is read through `CAST(... AS CHAR)`. MySQL hands
//! `information_schema` strings back as `VARBINARY`, which a driver has no
//! reason to decode as text; the cast is what makes the column readable.

use craber_core::{
    Action, Column, Dialect, ForeignKey, Outcome, Schema, Table, TableIndex, TableRef,
};
use sqlx::mysql::{MySqlConnectOptions, MySqlRow};
use sqlx::{Connection, MySqlConnection, Row};

use crate::assemble::{self, Catalog};
use crate::{Error, connect};

/// The name `information_schema.STATISTICS` gives a table's primary key.
const PRIMARY: &str = "PRIMARY";

pub(crate) async fn introspect(dsn: &str) -> Result<Outcome<Schema>, Error> {
    let options = connect::options::<MySqlConnectOptions>(dsn, Dialect::MySql)?;
    let mut conn = connect::connect(&options).await?;

    let name: Option<String> = sqlx::query_scalar("SELECT CAST(DATABASE() AS CHAR)")
        .fetch_one(&mut conn)
        .await?;
    let name = name.ok_or(Error::NoDatabase)?;

    let mut catalog = Catalog::default();
    read_tables(&mut conn, &mut catalog).await?;
    read_columns(&mut conn, &mut catalog).await?;
    read_indexes(&mut conn, &mut catalog).await?;
    read_foreign_keys(&mut conn, &name, &mut catalog).await?;

    conn.close().await?;
    // No enums: MySQL's are a column type rather than a named one, and their
    // values ride along in `COLUMN_TYPE`. The parser reports none either.
    Ok(catalog.finish(name))
}

/// `Table::schema` stays `None` throughout this module.
///
/// MySQL's "schema" *is* the database, and `craber-parser` leaves it unset for a
/// `mysqldump` — the fixture's tables are a bare `users`, not `shop.users`.
/// Putting the database name in would make every `--include` glob and every
/// qualified name disagree between a file and a connection.
async fn read_tables(conn: &mut MySqlConnection, catalog: &mut Catalog) -> Result<(), Error> {
    let rows = sqlx::query(
        "SELECT CAST(TABLE_NAME    AS CHAR) AS name,
                CAST(TABLE_COMMENT AS CHAR) AS comment
           FROM information_schema.TABLES
          WHERE TABLE_SCHEMA = DATABASE() AND TABLE_TYPE = 'BASE TABLE'
          ORDER BY TABLE_NAME",
    )
    .fetch_all(&mut *conn)
    .await?;

    for row in rows {
        let mut table = Table::new(row.try_get::<String, _>("name")?);
        table.comment = row.try_get("comment")?;
        catalog.push(table);
    }
    Ok(())
}

/// `COLUMN_TYPE`, not `DATA_TYPE`.
///
/// `DATA_TYPE` flattens to `enum` and `bigint`; `COLUMN_TYPE` keeps
/// `enum('admin','member','guest')` and `bigint unsigned`, which is what the
/// DDL said and what the parser records.
async fn read_columns(conn: &mut MySqlConnection, catalog: &mut Catalog) -> Result<(), Error> {
    let rows = sqlx::query(
        "SELECT CAST(TABLE_NAME     AS CHAR) AS table_name,
                CAST(COLUMN_NAME    AS CHAR) AS name,
                CAST(COLUMN_TYPE    AS CHAR) AS data_type,
                CAST(IS_NULLABLE    AS CHAR) AS nullable,
                CAST(COLUMN_DEFAULT AS CHAR) AS default_expr,
                CAST(COLUMN_COMMENT AS CHAR) AS comment,
                CAST(EXTRA          AS CHAR) AS extra
           FROM information_schema.COLUMNS
          WHERE TABLE_SCHEMA = DATABASE()
          ORDER BY TABLE_NAME, ORDINAL_POSITION",
    )
    .fetch_all(&mut *conn)
    .await?;

    for row in rows {
        let mut column = Column::new(
            row.try_get::<String, _>("name")?,
            row.try_get::<String, _>("data_type")?,
        );
        column.nullable = row
            .try_get::<String, _>("nullable")?
            .eq_ignore_ascii_case("YES");
        column.default = default(
            row.try_get("default_expr")?,
            &column.data_type,
            &row.try_get::<String, _>("extra")?,
        );
        column.comment = row.try_get("comment")?;
        if let Some(table) = catalog.table_mut(&table_of(&row)?) {
            table.columns.push(column);
        }
    }
    Ok(())
}

/// `COLUMN_DEFAULT` as an expression, which is what the IR holds.
///
/// MySQL stores a literal default stripped of its quotes — `member`, not
/// `'member'` — while the parser records the expression the DDL wrote. `EXTRA`
/// is what tells the two apart: an expression default is marked
/// `DEFAULT_GENERATED` there, so `CURRENT_TIMESTAMP` the function can be
/// distinguished from a `varchar` whose literal default happens to be the text
/// `CURRENT_TIMESTAMP`.
///
/// Numeric columns are left alone: their literal already reads as an
/// expression.
fn default(raw: Option<String>, data_type: &str, extra: &str) -> Option<String> {
    const NUMERIC: [&str; 12] = [
        "tinyint",
        "smallint",
        "mediumint",
        "int",
        "integer",
        "bigint",
        "decimal",
        "numeric",
        "float",
        "double",
        "real",
        "bit",
    ];

    let raw = raw?;
    if extra.to_ascii_uppercase().contains("DEFAULT_GENERATED") {
        return Some(raw);
    }
    let base = data_type
        .split_once(['(', ' '])
        .map_or(data_type, |(base, _)| base);
    if NUMERIC.contains(&base) {
        return Some(raw);
    }
    Some(format!("'{}'", raw.replace('\'', "''")))
}

/// The primary key and every other index, both out of `STATISTICS`.
///
/// Rows arrive one per column, so an index is finished when the name changes.
async fn read_indexes(conn: &mut MySqlConnection, catalog: &mut Catalog) -> Result<(), Error> {
    let rows = sqlx::query(
        "SELECT CAST(TABLE_NAME  AS CHAR) AS table_name,
                CAST(INDEX_NAME  AS CHAR) AS index_name,
                CAST(COLUMN_NAME AS CHAR) AS column_name,
                NON_UNIQUE                AS non_unique
           FROM information_schema.STATISTICS
          WHERE TABLE_SCHEMA = DATABASE()
            -- FULLTEXT and SPATIAL indexes are not the kind of key an ER
            -- diagram draws, and `craber-parser` skips their DDL too: a
            -- mysqldump's `FULLTEXT KEY` line has to produce the same nothing
            -- from either source.
            AND INDEX_TYPE NOT IN ('FULLTEXT', 'SPATIAL')
            -- A functional index keeps its expression in EXPRESSION and leaves
            -- this NULL. There is no column row for the edge to sit on.
            AND COLUMN_NAME IS NOT NULL
          ORDER BY TABLE_NAME, INDEX_NAME, SEQ_IN_INDEX",
    )
    .fetch_all(&mut *conn)
    .await?;

    // `ORDER BY TABLE_NAME, INDEX_NAME, SEQ_IN_INDEX` means one index's columns
    // arrive together, so an index is a consecutive run of rows — which
    // `chunk_by` gives without an accumulator and, more to the point, without a
    // trailing flush that drops the last index of the whole query if forgotten.
    let rows = keyed(rows, "index_name")?;
    for index in rows.chunk_by(|a, b| a.0 == b.0) {
        let ((table, name), first) = &index[0];
        let columns = index
            .iter()
            .map(|(_, row)| row.try_get("column_name"))
            .collect::<Result<Vec<String>, _>>()?;
        let unique = first.try_get::<i64, _>("non_unique")? == 0;
        let Some(table) = catalog.table_mut(table) else {
            continue;
        };
        if name == PRIMARY {
            table.pk = columns;
        } else {
            table.indexes.push(TableIndex {
                name: Some(name.clone()),
                columns,
                unique,
            });
        }
    }
    Ok(())
}

/// What makes two rows part of the same index or constraint: its table and its
/// name.
type Key = (TableRef, String);

/// Each row paired with its [`Key`], the name read from `name_column`. Decoded
/// up front, so grouping rows cannot fail halfway through.
fn keyed(rows: Vec<MySqlRow>, name_column: &str) -> Result<Vec<(Key, MySqlRow)>, sqlx::Error> {
    rows.into_iter()
        .map(|row| Ok(((table_of(&row)?, row.try_get(name_column)?), row)))
        .collect()
}

/// The table a row names in its `table_name` column.
///
/// Unqualified: see [`read_tables`].
fn table_of(row: &MySqlRow) -> Result<TableRef, sqlx::Error> {
    Ok(TableRef::new(row.try_get::<String, _>("table_name")?))
}

/// `database` is the one the connection is scoped to, which a key's
/// `REFERENCED_TABLE_SCHEMA` is compared against.
async fn read_foreign_keys(
    conn: &mut MySqlConnection,
    database: &str,
    catalog: &mut Catalog,
) -> Result<(), Error> {
    let rows = sqlx::query(
        "SELECT CAST(k.TABLE_NAME              AS CHAR) AS table_name,
                CAST(k.CONSTRAINT_NAME         AS CHAR) AS name,
                CAST(k.COLUMN_NAME             AS CHAR) AS column_name,
                CAST(k.REFERENCED_TABLE_NAME   AS CHAR) AS ref_table,
                CAST(k.REFERENCED_COLUMN_NAME  AS CHAR) AS ref_column,
                CAST(k.REFERENCED_TABLE_SCHEMA AS CHAR) AS ref_schema,
                CAST(r.DELETE_RULE             AS CHAR) AS on_delete,
                CAST(r.UPDATE_RULE             AS CHAR) AS on_update
           FROM information_schema.KEY_COLUMN_USAGE k
           JOIN information_schema.REFERENTIAL_CONSTRAINTS r
             ON r.CONSTRAINT_SCHEMA = k.CONSTRAINT_SCHEMA
            AND r.CONSTRAINT_NAME   = k.CONSTRAINT_NAME
            AND r.TABLE_NAME        = k.TABLE_NAME
          WHERE k.TABLE_SCHEMA = DATABASE()
            AND k.REFERENCED_TABLE_NAME IS NOT NULL
          ORDER BY k.TABLE_NAME, k.CONSTRAINT_NAME, k.ORDINAL_POSITION",
    )
    .fetch_all(&mut *conn)
    .await?;

    // As for the indexes: `ORDER BY k.TABLE_NAME, k.CONSTRAINT_NAME,
    // k.ORDINAL_POSITION` makes a constraint a consecutive run of rows.
    let rows = keyed(rows, "name")?;
    for key in rows.chunk_by(|a, b| a.0 == b.0) {
        let ((table, name), first) = &key[0];
        let context = assemble::context(table, name);
        let ref_schema: Option<String> = first.try_get("ref_schema")?;

        // A key into another database points at a table this diagram does not
        // contain, so the edge would dangle and `Filter` would drop it anyway.
        // `ref_schema` is a property of the constraint, so this is one warning
        // per key rather than one per column of it.
        if ref_schema.is_some_and(|schema| schema != database) {
            catalog.warn("foreign key into another database, skipped", context);
            continue;
        }

        let mut columns = Vec::with_capacity(key.len());
        let mut ref_columns = Vec::with_capacity(key.len());
        for (_, row) in key {
            columns.push(row.try_get("column_name")?);
            ref_columns.push(row.try_get("ref_column")?);
        }
        let on_delete = first.try_get::<String, _>("on_delete")?;
        let on_update = first.try_get::<String, _>("on_update")?;
        let fk = ForeignKey::declared(
            columns,
            TableRef::new(first.try_get::<String, _>("ref_table")?),
            ref_columns,
        )
        .named(name.clone())
        .with_actions(
            catalog.action(&on_delete, &context, Action::from_sql),
            catalog.action(&on_update, &context, Action::from_sql),
        );
        if let Some(table) = catalog.table_mut(table) {
            table.fks.push(fk);
        }
    }
    Ok(())
}
