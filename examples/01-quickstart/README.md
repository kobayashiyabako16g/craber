# 01 — Quickstart

The smallest end-to-end run: a `.sql` file in, `schema.json` out.

```console
$ cargo run -p craber -- export --file examples/01-quickstart/blog.sql --format json
```

`blog.sql` is a five-table PostgreSQL blogging schema — authors, posts, tags, a
`post_tags` join table, and self-referencing comments.

## What to look for

**Diagnostics go to stderr, the diagram to stdout.** The first thing printed is:

```
craber: warning: statement carries schema information CrabER does not read yet (CREATE INDEX ...)
```

CrabER cannot read `CREATE INDEX` yet, so it says so and draws the five tables
anyway. That split is why `... export --format json > schema.json` gives you a
clean file even when part of the input was skipped.

**Keys and relationships are resolved, not guessed.** In the output:

- `post_tags` has `"pk": ["post_id", "tag_id"]` — the composite key is kept in
  key order.
- Both of its foreign keys carry `"cardinality": "manyToOne"`, derived from the
  fact that neither FK column is unique on its own.
- `comments.parent_id` points back at `comments`; self-references are ordinary
  edges.
- `"onDelete": "cascade"` / `"setNull"` are read off the DDL, so the diagram can
  show what a delete actually does.

**Positions are already in the file.** Every table has a `"position"`, assigned
by `craber-layout` before anything is written. The frontend draws those
coordinates and never runs a layout of its own — that is what lets the CLI and
the web view agree, and what will let `--format svg` work with no browser.

## Writing to a file

```console
$ cargo run -p craber -- export --file examples/01-quickstart/blog.sql -o /tmp/blog.json
```

Missing parent directories are created. See [04-web-viewer](../04-web-viewer)
for putting this on screen.
