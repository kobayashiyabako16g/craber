# Fixtures

`tiny.sql` is the smoke-test schema, shaped like a `pg_dump`: six tables, an
enum type, a composite primary key, a column-level `REFERENCES`, and then the
trailing `ALTER TABLE ... ADD CONSTRAINT`, `CREATE INDEX` and `COMMENT ON` that
a dump puts after every table exists. One statement is left unapplied on
purpose, so the warn-and-continue path stays covered.

It is what the introspection round-trip loads into a real PostgreSQL:
`crates/craber-introspect/tests/postgres.rs` reads this file, cuts it at the
`-- Not applied:` marker and loads the rest, then asserts that introspecting
the result describes the same diagram as parsing the file. That marker line is
therefore load-bearing — the statement under it is rejected by a real server,
and one rejected statement rolls the whole batch back.

It is also the notation sample. Its four foreign keys are every combination of
"is the key unique" and "are its columns nullable", which is all a dialect says
about a relationship — so between them they draw every end CrabER can put on a
line: `one and only one`, `zero or one`, and `zero or many`. The table at the
top of the file maps each key to the ends it produces, and
`crates/craber/tests/export.rs` holds the derivation to it. It is also what
makes the round-trip above worth as much as it is: if a live read and a parsed
file agree on this schema, they agree on every end CrabER can draw.

`comments.sql` is two tables and a lot of prose: a paragraph-length table
comment, a column comment with no spaces to wrap at (CJK), a one-line comment,
and a `nextval(...)` default too long to sit beside its label. Comments are the
only free-form text in the IR, and they are what breaks a layout — the web app's
details panel decides per value whether it fits beside its label or needs the
panel's full width, and this is the schema that exercises that.
`just ui fixtures/comments.sql`, then click a table.
`crates/craber/tests/export.rs` holds the pipeline to handing them over whole.

`wide.sql` is the M1 layout checkpoint: 100 tables and 136 foreign keys,
synthesised to look like a multi-tenant SaaS. It has the shape that makes layout
hard — hub tables almost everything points at, long chains, join tables, several
namespaces, and reference tables with no relationships at all. `crates/craber/
tests/scale.rs` holds it to the measurable half of "is this readable"; the other
half is `just svg fixtures/wide.sql` and your own eyes.

`dump.mysql.sql` and `dump.sqlite.sql` are the non-Postgres pair, and they are
shaped like what the tools actually emit rather than like DDL anyone would
write: `mysqldump`'s `/*!40101 … */` version comments, `DROP TABLE` /
`LOCK TABLES` scaffolding and one `KEY` line per index; `sqlite3 .schema`'s
`sqlite_sequence`, `AUTOINCREMENT`, `[bracket]` quoting and `WITHOUT ROWID` /
`STRICT` tails. The hand-written schemas in `examples/03-dialects` stay readable
on purpose and so exercise none of that. What
`crates/craber/tests/dialects.rs` mostly asserts about these two is that
**stderr stays empty**: the failure worth guarding against is CrabER warning
about ordinary dump syntax, once per index, which is how a warning that matters
gets lost.

## Third-party schemas

Real schemas are worth testing against too, but they are not committed here —
they are large, and their licenses are not ours to redistribute.
Fetch them into this directory when you need them; the entries below are already
covered by `.gitignore` conventions, so drop them here and they stay untracked
only if you avoid `git add -f`.

| Schema | Tables | Where |
|---|---|---|
| Pagila (PostgreSQL) | ~22 | <https://github.com/devrimgunduz/pagila> — `pagila-schema.sql` |
| Sakila (MySQL) | ~16 | <https://dev.mysql.com/doc/index-other.html> — `sakila-schema.sql` |
| Chinook (multi-dialect) | ~11 | <https://github.com/lerocha/chinook-database> |

None of those reach 100 tables, which is why `wide.sql` exists. For a real
schema at that size, use a mature Rails or Django `schema.sql`.
