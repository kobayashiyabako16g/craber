# Examples

Runnable schemas for driving CrabER end to end. Every command below is meant to
be run from the repository root; nothing here needs a database, though `--dsn`
takes one in place of any `--file` you see here.

| | What it shows |
|---|---|
| [01-quickstart](01-quickstart) | `.sql` in, `schema.json` out — keys, relationships, positions, and the warn-and-continue path |
| [02-filtering](02-filtering) | `--include` / `--exclude` / `--depth` and `craber.toml`, on a schema with the usual infrastructure noise |
| [03-dialects](03-dialects) | MySQL and SQLite input, and how the dialect is chosen |
| [04-web-viewer](04-web-viewer) | Putting an example on screen in the React Flow frontend |

Start here:

```console
$ cargo run -p craber -- export --file examples/01-quickstart/blog.sql --format json
```

Or, with the [`just`](../justfile) tasks:

```console
$ just example                                     # 01-quickstart
$ just example examples/02-filtering/shop.sql
```

## Scope

These exercise reading DDL from a file, narrowing it, laying it out, and
writing `schema.json`. Reading a live database is the same pipeline with
`--dsn` in place of `--file`, and is covered in the README rather than here:
these files are meant to be runnable from a clone with nothing installed.

The commands that are declared but not built yet — `diff`, `build
--single-file`, and the `png` / `mermaid` / `dbml` export formats — fail with
the milestone they are scheduled for.

Examples are also a regression net: they are small enough to read, and each one
is a schema shape the parser is expected to keep handling.

## Fixtures vs. examples

[`fixtures/`](../fixtures) holds the schemas the test suite asserts against —
minimal, and awkward on purpose so they cover edge cases. The files here are
meant to be read by people and to look like schemas you might actually have.
