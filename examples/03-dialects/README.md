# 03 — Dialects

CrabER reads PostgreSQL, MySQL and SQLite DDL. The dialect decides how the input
is tokenized (backtick quoting, `AUTO_INCREMENT`, engine options), so getting it
wrong usually means a parse error rather than a slightly-off diagram.

```console
$ cargo run -p craber -- export --file examples/03-dialects/inventory.mysql.sql
$ cargo run -p craber -- export --file examples/03-dialects/tasks.sqlite.sql
```

Neither command passes `--dialect`: it is inferred from the filename, which
contains `mysql` / `sqlite`. Dumps are rarely named that conveniently, so be
explicit when they are not:

```console
$ cargo run -p craber -- export --file dump.sql --dialect mysql
```

Accepted spellings: `postgres` (`postgresql`, `pg`), `mysql` (`mariadb`),
`sqlite` (`sqlite3`). PostgreSQL is the default when nothing is given and the
filename says nothing.

## What each file exercises

`inventory.mysql.sql` — backtick-quoted identifiers, `int unsigned`,
`AUTO_INCREMENT`, `ENGINE=InnoDB DEFAULT CHARSET=utf8mb4`, table-level
`UNIQUE KEY`, and named `CONSTRAINT ... FOREIGN KEY`. Note that `stock_moves`
has two foreign keys into `warehouses` (`from_id`, `to_id`) — two separate
edges between the same pair of tables, not one merged edge. MySQL has no schema
namespace, so the tables come out with no `"schema"` field.

`tasks.sqlite.sql` — `INTEGER PRIMARY KEY` (the rowid alias), column-level
`REFERENCES`, a self-reference on `tasks.parent_id`, and a `task_labels` join
table.

## Real dumps

The two files here are hand-written so they stay readable. What you will
actually feed CrabER is a `mysqldump` or a `sqlite3 db .schema`, and those come
with scaffolding: `/*!40101 SET … */` version comments, `DROP TABLE IF EXISTS`,
`LOCK TABLES`, a `KEY`/`FULLTEXT KEY` line per index, `sqlite_sequence`,
`AUTOINCREMENT`, `[bracket]` quoting, `WITHOUT ROWID`. All of it parses, and
none of it warns. `fixtures/dump.mysql.sql` and `fixtures/dump.sqlite.sql` are
those shapes, held to exactly that by `crates/craber/tests/dialects.rs`.

Two things a dump carries that the diagram deliberately does not show:

- **`sqlite_*` tables.** `sqlite_sequence` and `sqlite_stat1` are the engine's
  bookkeeping, not your model, so CrabER drops them under `--dialect sqlite`.
- **`AUTO_INCREMENT` / `AUTOINCREMENT`.** There is nowhere on a node for it yet.
  PostgreSQL's equivalent shows up only because `DEFAULT nextval(…)` is a
  default like any other.

A full-text or spatial `KEY` is skipped too: it is an access path, it makes
nothing unique, and there is no end of a line it could change. An ordinary
`KEY`, on the other hand, is listed with the table's indexes — it just never
turns a `many` into a `one`.
