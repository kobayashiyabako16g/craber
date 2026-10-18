# 04 — Seeing it on screen

`schema.json` is the whole contract between the Rust side and the frontend, so
putting an example on screen is just a matter of exporting one where the web app
looks for it: `web/public/schema.json`.

```console
$ just web-install                                # once
$ just sample examples/01-quickstart/blog.sql     # export → web/public/schema.json
$ just web-dev                                    # http://localhost:5173
```

Without `just`, that middle step is:

```console
$ cargo run -p craber -- export --file examples/01-quickstart/blog.sql \
    -o web/public/schema.json
```

Any of the example schemas works — swap in `examples/02-filtering/shop.sql`, or
add `--include orders --depth 1` and reload to watch the diagram shrink.

`web/public/schema.json` is a committed file (the `fixtures/tiny.sql` export), so
put it back when you are done:

```console
$ git restore web/public/schema.json
```

## What the page is doing

The frontend fetches `schema.json`, checks `schemaVersion` against the version
it was built to read, and hands React Flow one node per table and one edge per
foreign key. It never computes a layout: `x`/`y` come from `craber-layout` and
nodes are not draggable, which is what keeps the browser view and the (still to
come) `--format svg` export showing the same picture.

Node contents are read straight from the IR — key columns marked from `pk`, FK
columns from `fks`, types and nullability from `columns`.

## Editing the schema

`just web-dev` runs Vite with hot reload for the frontend, but nothing watches
the `.sql` file yet — re-run the export and reload the page. `craber serve
--watch` is what removes that step; it arrives in M3.

## Static build

```console
$ just web-build     # typecheck + production bundle into web/dist
```

That is the bundle `craber build` will eventually ship alongside a schema.
Serving `web/dist` today needs a `schema.json` next to `index.html`.

## Without opening a browser yourself

`just ui` folds the two steps above into one — export, then dev server:

```console
$ just ui examples/01-quickstart/blog.sql
$ just sample-reset    # puts web/public/schema.json back
```

That is also the entry point Claude Code uses. `.claude/skills/ui-check`
describes the rest of the loop it follows: open the page through the Chrome
extension, screenshot it, read the console, and check the rendered node count
against the schema — so a frontend change gets looked at rather than assumed.
