//! PostgreSQL, read from `pg_catalog`.
//!
//! `information_schema` is the portable choice and the wrong one here: it cannot
//! render a type the way the server writes it, it has no access to comments, and
//! it is slower. `format_type`, `pg_get_expr` and `obj_description` are what make
//! the result look like the DDL it came from.

use craber_core::{
    Action, Column, Diagnostic, Dialect, EnumType, ForeignKey, Outcome, Schema, Table, TableIndex,
    TableRef,
};
use sqlx::postgres::{PgConnectOptions, PgRow};
use sqlx::{Connection, PgConnection, Row};

use crate::assemble::{self, Tables};
use crate::{Error, connect};

/// The relations a diagram is drawn from, and the ones it never is.
///
/// Views and materialized views are left out on purpose. `craber-parser` only
/// translates `CREATE TABLE`, so a `.sql` file's views produce nothing; if a
/// connection produced them, the two sources would stop agreeing on the very
/// first real database. Partition children are left out because a partitioned
/// table with a year of daily partitions is 365 identical nodes.
///
/// A macro rather than a `const` so that every query below stays a string
/// literal: sqlx will not accept a query built at runtime without an explicit
/// assertion that it is safe, and `concat!` means there is nothing to assert.
/// The SQL in this module is fixed at compile time and every value is bound.
macro_rules! relations {
    () => {
        "
      c.relkind IN ('r', 'p', 'f')
  AND NOT c.relispartition
  AND n.nspname NOT IN ('pg_catalog', 'information_schema')
  AND n.nspname NOT LIKE 'pg\\_toast%'
  AND n.nspname NOT LIKE 'pg\\_temp%'
  AND NOT EXISTS (
        SELECT 1 FROM pg_catalog.pg_depend d
         WHERE d.classid = 'pg_class'::regclass
           AND d.objid = c.oid
           AND d.deptype = 'e')
"
    };
}

pub(crate) async fn introspect(dsn: &str) -> Result<Outcome<Schema>, Error> {
    let options = connect::options::<PgConnectOptions>(dsn, Dialect::Postgres)?
        // An empty search_path makes `format_type` schema-qualify every type it
        // renders. Left at its default it qualifies only what is off the path,
        // so `orders.status` comes back as `order_status` for one role and
        // `public.order_status` for another — the same database describing
        // itself differently depending on who asked.
        .options([("search_path", "")]);

    let mut conn = connect::connect(&options).await?;

    let name: String = sqlx::query_scalar("SELECT current_database()")
        .fetch_one(&mut conn)
        .await?;

    let mut diagnostics = Vec::new();
    let mut tables = read_tables(&mut conn).await?;
    read_columns(&mut conn, &mut tables).await?;
    read_constraints(&mut conn, &mut tables, &mut diagnostics).await?;
    read_indexes(&mut conn, &mut tables).await?;
    let enums = read_enums(&mut conn).await?;

    conn.close().await?;
    Ok(assemble::finish(name, tables, enums, diagnostics))
}

async fn read_tables(conn: &mut PgConnection) -> Result<Tables, Error> {
    let sql = concat!(
        "SELECT n.nspname AS schema,
                c.relname AS name,
                obj_description(c.oid, 'pg_class') AS comment
           FROM pg_catalog.pg_class c
           JOIN pg_catalog.pg_namespace n ON n.oid = c.relnamespace
          WHERE",
        relations!(),
        "          ORDER BY n.nspname, c.relname"
    );

    let mut tables = Tables::default();
    for row in sqlx::query(sql).fetch_all(&mut *conn).await? {
        let mut table = Table::new(row.get::<String, _>("name"));
        table.schema = Some(row.get::<String, _>("schema"));
        table.comment = assemble::comment(row.get("comment"));
        tables.push(table);
    }
    Ok(tables)
}

/// `ORDER BY a.attnum` is what puts the columns in the order the DDL declared
/// them, which is the order the node draws them in.
async fn read_columns(conn: &mut PgConnection, tables: &mut Tables) -> Result<(), Error> {
    let sql = concat!(
        "SELECT n.nspname AS schema,
                c.relname AS table_name,
                a.attname AS name,
                pg_catalog.format_type(a.atttypid, a.atttypmod) AS data_type,
                NOT a.attnotnull                           AS nullable,
                pg_catalog.pg_get_expr(d.adbin, d.adrelid) AS default_expr,
                col_description(c.oid, a.attnum)           AS comment
           FROM pg_catalog.pg_attribute a
           JOIN pg_catalog.pg_class     c ON c.oid = a.attrelid
           JOIN pg_catalog.pg_namespace n ON n.oid = c.relnamespace
           LEFT JOIN pg_catalog.pg_attrdef d
                  ON d.adrelid = c.oid AND d.adnum = a.attnum
          WHERE a.attnum > 0 AND NOT a.attisdropped AND",
        relations!(),
        "          ORDER BY n.nspname, c.relname, a.attnum"
    );

    for row in sqlx::query(sql).fetch_all(&mut *conn).await? {
        let Some(table) = table_of(tables, &row) else {
            continue;
        };
        let mut column = Column::new(
            row.get::<String, _>("name"),
            row.get::<String, _>("data_type"),
        );
        column.nullable = row.get("nullable");
        column.default = row.get("default_expr");
        column.comment = assemble::comment(row.get("comment"));
        table.columns.push(column);
    }
    Ok(())
}

/// Primary and foreign keys, both out of `pg_constraint`.
///
/// `conkey` and `confkey` are `smallint[]` of column numbers. They are expanded
/// in SQL with `unnest ... WITH ORDINALITY` rather than decoded here, which
/// keeps key order without this crate ever handling an array type.
async fn read_constraints(
    conn: &mut PgConnection,
    tables: &mut Tables,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<(), Error> {
    let sql = concat!(
        "SELECT n.nspname  AS schema,
                c.relname  AS table_name,
                con.conname AS name,
                con.contype::text     AS contype,
                con.confdeltype::text AS on_delete,
                con.confupdtype::text AS on_update,
                (SELECT array_agg(att.attname ORDER BY k.ord)
                   FROM unnest(con.conkey) WITH ORDINALITY AS k(attnum, ord)
                   JOIN pg_catalog.pg_attribute att
                     ON att.attrelid = con.conrelid AND att.attnum = k.attnum) AS columns,
                fn.nspname AS ref_schema,
                fc.relname AS ref_table,
                (SELECT array_agg(att.attname ORDER BY k.ord)
                   FROM unnest(con.confkey) WITH ORDINALITY AS k(attnum, ord)
                   JOIN pg_catalog.pg_attribute att
                     ON att.attrelid = con.confrelid AND att.attnum = k.attnum) AS ref_columns
           FROM pg_catalog.pg_constraint con
           JOIN pg_catalog.pg_class     c  ON c.oid = con.conrelid
           JOIN pg_catalog.pg_namespace n  ON n.oid = c.relnamespace
           LEFT JOIN pg_catalog.pg_class     fc ON fc.oid = con.confrelid
           LEFT JOIN pg_catalog.pg_namespace fn ON fn.oid = fc.relnamespace
          WHERE con.contype IN ('p', 'f') AND",
        relations!(),
        "          ORDER BY n.nspname, c.relname, con.contype, con.conname"
    );

    for row in sqlx::query(sql).fetch_all(&mut *conn).await? {
        let columns: Vec<String> = row.try_get("columns").unwrap_or_default();
        let name: String = row.get("name");
        let contype: String = row.get("contype");

        if columns.is_empty() {
            continue;
        }

        let foreign_key = if contype == "f" {
            let (Some(ref_table), Some(ref_columns)) = (
                row.get::<Option<String>, _>("ref_table"),
                row.try_get::<Option<Vec<String>>, _>("ref_columns")
                    .unwrap_or_default(),
            ) else {
                diagnostics.push(
                    Diagnostic::warning("foreign key without a referenced table")
                        .with_context(name.clone()),
                );
                continue;
            };
            Some((ref_table, ref_columns))
        } else {
            None
        };

        let Some(table) = table_of(tables, &row) else {
            continue;
        };

        match foreign_key {
            None => table.pk = columns,
            Some((ref_table, ref_columns)) => {
                let ref_schema: Option<String> = row.get("ref_schema");
                let mut fk = ForeignKey::declared(
                    columns,
                    TableRef {
                        schema: ref_schema,
                        name: ref_table,
                    },
                    ref_columns,
                );
                fk.name = Some(name.clone());
                fk.on_delete = assemble::action_or_warn(
                    &row.get::<String, _>("on_delete"),
                    &name,
                    diagnostics,
                    action_from_char,
                );
                fk.on_update = assemble::action_or_warn(
                    &row.get::<String, _>("on_update"),
                    &name,
                    diagnostics,
                    action_from_char,
                );
                table.fks.push(fk);
            }
        }
    }
    Ok(())
}

/// The table a catalog row belongs to.
///
/// Every query here spans the whole catalog and is filtered by `relations!()`,
/// so a row naming a table that is not in `tables` is ordinary — a view, a
/// partition child — rather than an error. Four read passes resolved this the
/// same three lines each.
fn table_of<'a>(tables: &'a mut Tables, row: &PgRow) -> Option<&'a mut craber_core::Table> {
    let schema: String = row.get("schema");
    let name: String = row.get("table_name");
    tables.get_mut(Some(&schema), &name)
}

/// `confdeltype` / `confupdtype` are single characters, not words — the one
/// thing about referential actions PostgreSQL does differently.
fn action_from_char(raw: &str) -> Option<Action> {
    match raw {
        "a" => Some(Action::NoAction),
        "r" => Some(Action::Restrict),
        "c" => Some(Action::Cascade),
        "n" => Some(Action::SetNull),
        "d" => Some(Action::SetDefault),
        _ => None,
    }
}

async fn read_indexes(conn: &mut PgConnection, tables: &mut Tables) -> Result<(), Error> {
    let sql = concat!(
        "SELECT n.nspname  AS schema,
                c.relname  AS table_name,
                ic.relname AS name,
                i.indisunique AS is_unique,
                (i.indpred IS NOT NULL) AS partial,
                (SELECT array_agg(a.attname ORDER BY k.ord)
                   FROM unnest(i.indkey::int2[]) WITH ORDINALITY AS k(attnum, ord)
                   JOIN pg_catalog.pg_attribute a
                     ON a.attrelid = c.oid AND a.attnum = k.attnum
                  WHERE k.attnum <> 0) AS columns
           FROM pg_catalog.pg_index i
           JOIN pg_catalog.pg_class     ic ON ic.oid = i.indexrelid
           JOIN pg_catalog.pg_class     c  ON c.oid  = i.indrelid
           JOIN pg_catalog.pg_namespace n  ON n.oid  = c.relnamespace
          WHERE NOT i.indisprimary AND i.indisvalid AND",
        relations!(),
        "          ORDER BY n.nspname, c.relname, ic.relname"
    );

    for row in sqlx::query(sql).fetch_all(&mut *conn).await? {
        let columns: Vec<String> = row.try_get("columns").unwrap_or_default();

        // An index over an expression alone has no column row to attach to, the
        // same reason `craber_parser` drops one.
        if columns.is_empty() {
            continue;
        }
        let Some(table) = table_of(tables, &row) else {
            continue;
        };
        table.indexes.push(TableIndex {
            name: Some(row.get::<String, _>("name")),
            columns,
            // A partial unique index does not make the tuple unique — rows
            // outside its predicate are unconstrained. `Table::is_unique_tuple`
            // reads this field to decide a foreign key's cardinality, so
            // recording one as unique draws the wrong crow's foot. The index is
            // still worth listing; its uniqueness is not.
            unique: row.get::<bool, _>("is_unique") && !row.get::<bool, _>("partial"),
        });
    }
    Ok(())
}

async fn read_enums(conn: &mut PgConnection) -> Result<Vec<EnumType>, Error> {
    let sql = "SELECT n.nspname AS schema,
                      t.typname AS name,
                      array_agg(e.enumlabel ORDER BY e.enumsortorder) AS values
                 FROM pg_catalog.pg_type t
                 JOIN pg_catalog.pg_namespace n ON n.oid = t.typnamespace
                 JOIN pg_catalog.pg_enum e ON e.enumtypid = t.oid
                WHERE n.nspname NOT IN ('pg_catalog', 'information_schema')
                GROUP BY 1, 2
                ORDER BY 1, 2";

    Ok(sqlx::query(sql)
        .fetch_all(&mut *conn)
        .await?
        .into_iter()
        .map(|row| EnumType {
            schema: Some(row.get("schema")),
            name: row.get("name"),
            values: row.get("values"),
        })
        .collect())
}
