# CrabER documentation

Reference material for working on CrabER, or for driving it harder than the
[README](../README.md) covers.

| Document | What it is |
|---|---|
| [CONTRIBUTING.md](../CONTRIBUTING.md) | Setup, the gates CI runs, naming, how a release is cut |
| [examples/](../examples) | Runnable walkthroughs of the pipeline |
| [craber.toml.example](craber.toml.example) | An annotated config file |

## Repository layout

```
crates/
  craber-core       Schema IR, name resolution, filtering, diagnostics
  craber-parser     SQL DDL → IR, via sqlparser-rs
  craber-introspect A live database → IR, via sqlx
  craber-layout     IR → table positions (Sugiyama-style layered layout)
  craber-render     IR + positions → SVG
  craber-site       The static site: the web bundle + one schema.json
  craber            The `craber` CLI
web/              React + React Flow frontend
fixtures/         Schemas used by the tests
examples/         Runnable walkthroughs of what CrabER does today
```

`craber-export` and `craber-wasm` arrive with the milestones that need them.
[CONTRIBUTING.md](../CONTRIBUTING.md) has the per-crate responsibilities and the
two invariants that are easy to break by accident.

## `schema.json` is the contract

Every output — static site, `serve`, single-file HTML, SVG, Mermaid — is
rendered from the same `schema.json`, with table coordinates already baked in by
`craber-layout`. The frontend receives positions and draws them; it never runs a
layout of its own. Each table carries one position per detail level, because a
node showing only its name is shorter than one showing every column. That is
what lets `export --format svg` work with no browser in the loop, and what keeps
the CLI and the web view showing the same picture.

The TypeScript types in `web/src/types` are generated from the Rust IR by
`ts-rs`, so the two cannot drift apart. Change `crates/craber-core/src/ir.rs`,
then run `just bindings`.

## Configuration reference

Anything you would otherwise retype in every CI job can live in a `craber.toml`
next to the schema. With one committed, `craber build` and `craber export` take
no arguments at all.

```toml
# Where the schema comes from and what to draw — every command reads these.
file    = "db/schema.sql"      # or dsn = "postgres://..."
dialect = "postgres"           # inferred from the filename or DSN when omitted
exclude = ["schema_migrations", "audit_log*"]
depth   = 1

# Anything one command alone understands lives in that command's section.
[build]
output    = "dist/"
base-path = "/myrepo/"

[serve]
port = 4300

[export]
format = "svg"
output = "er.svg"
```

| Key | Where | What |
|---|---|---|
| `file` / `dsn` | top level | the schema source — one or the other |
| `dialect` | top level | `postgres` / `mysql` / `sqlite`, same spellings `--dialect` takes |
| `include` / `exclude` | top level | table globs |
| `depth` | top level | foreign-key hops to grow the selection by |
| `output` | `[build]`, `[export]` | directory for `build`, file for `export` |
| `base-path` | `[build]` | URL prefix for the published assets |
| `port` | `[serve]` | port to listen on |
| `format` | `[export]` | `json` / `svg` / … |

Command-specific keys are sectioned rather than shared so that a key one
command needs cannot change another's behaviour: a top-level `output` would
mean `craber export --format json` wrote a file instead of stdout the moment a
config file appeared beside it, which would break anything reading CrabER's
output through a pipe.

Precedence is **CLI flag > environment variable > config file > default**. A
flag replaces the file's value rather than merging with it — `--exclude
sessions` means the file's exclusions no longer apply, and `--dsn` in a
repository whose config names a `file` connects rather than reading the file.

CrabER looks for `craber.toml` in the working directory and then in each
directory above it, so a command run deep inside a repository still draws what
the repository committed. `--config PATH` names one explicitly and is an error
if it is not there; having no `craber.toml` anywhere is not. Paths inside the
file are resolved against **the file**, not the working directory, so `file =
"db/schema.sql"` means the same thing from anywhere. Unknown keys are rejected
rather than ignored, so a typo says so.

## What each command does today

### `export`

Reads a PostgreSQL, MySQL or SQLite schema — including the scaffolding a real
dump wraps it in, like the `ALTER TABLE ... ADD CONSTRAINT` / `CREATE INDEX` /
`COMMENT ON` tail of a `pg_dump` and the `KEY` lines of a `mysqldump` — narrows
the result with `--include` / `--exclude` / `--depth`, lays the tables out, and
writes either `schema.json` or a finished SVG. Anything CrabER cannot read yet
is reported on stderr and skipped, so a single unsupported statement never costs
you the whole diagram.

### `build`

Writes a static site that deploys as-is — the diagram, the schema it was built
for, and nothing else to install:

```text
dist/
  index.html               # <title> names the schema
  assets/index-<hash>.js   # compiled into the binary
  assets/index-<hash>.css
  favicon.svg
  schema.json              # IR + table coordinates, one set per detail level
  serve.json               # cache headers for hosts that read it
  .craber-build.json       # what this build wrote, so the next one can
                           # clear the previous bundle and nothing else
```

Asset URLs are relative, so the same directory works at `/` and under GitHub
Pages' `/<repo>/`; `--base-path /myrepo/` rewrites them absolutely when a host
needs that instead. The binary has to be built with the frontend present for
this to work (`just site` does both) — otherwise `build` says so.

### `serve`

Serves the same page `build` publishes, from memory, on loopback. `--watch`
re-reads the schema when the file changes — reload the tab to see it. The server
is written on the standard library rather than a web framework; it is a
development server and is not meant for anything else.

### The web view

`web/` renders the same `schema.json` with React Flow, at the same coordinates:
a table list with search and per-table visibility, relationship lines that run
from the foreign key's column to the column it references, with the crow's-foot
end each key's uniqueness and nullability imply, dimming of everything outside
the selected table's neighbourhood, a details panel — columns, indexes and both
directions of every relationship — for the table you click, an address bar that
carries the whole view state so a link out of it lands where you were, and an
`Export SQL` that downloads the table definitions — structure only, no
privileges or data. Tables can be dragged out of the way — an anchor stays on
its column and the line re-attaches to whichever sides now face each other —
and `Tidy up` puts them back where `craber-layout` placed them. Every column is
always drawn: the lines attach to columns, so there is nothing to collapse them
into.

### `--dsn`: reading a live database

`--dsn` takes the same three dialects and works with every command — the scheme
picks the dialect, so `--dialect` is only for a connection string that does not
carry one. Prefer `CRABER_DSN` over the command line: a DSN carries a password,
and argv is visible to every process on the machine. CrabER never puts it in an
error message either.

A live read and a parsed dump describe the same schema, but do not produce the
same file. Everything the relationships are drawn from — keys, nullability,
cardinalities, comments — matches. What differs is how the server spells things
it was never asked to remember verbatim: a type renders its own way (`timestamp
with time zone`, where the file said `TIMESTAMPTZ`), a default keeps its cast
(`'pending'::order_status`), and the index behind an inline `UNIQUE` has a name.

Tables also come back in a different order — by name, because a running
database has no record of the order its DDL was written in — and since layout
breaks ties within a layer by that order, the two sources stack the same graph
differently. Same layers, same edges, different rows inside a layer.
`crates/craber-introspect` documents all of it.

### Not implemented yet

`build --single-file`, `diff`, and the `png` / `mermaid` / `dbml` export
formats. Those commands tell you which milestone they are scheduled for rather
than failing obscurely. `--watch` is for files; there is nothing to watch on a
connection.

## Install routes in detail

### From crates.io

```console
$ cargo install craber
```

`web/dist` lives outside the package directory, so the published `craber-site`
carries its own copy in `bundle/` (written by `just vendor-bundle` before each
publish). `cargo install` therefore compiles the frontend in without needing
Node or pnpm — only Rust 1.94+.

### From a checkout

```console
$ just dist        # builds the frontend, then target/release/craber
$ just install     # the same, then cargo install --path crates/craber
```

Both build `web/dist` first, because a binary compiled without it can parse and
export but cannot write or serve a site. It says so if that happens.

Building from a checkout therefore needs Node and pnpm as well as Rust. That is
also why there is no `cargo install --git` yet: `web/dist` is generated, so it
is not in the repository for a clone to pick up.

### From a release

Each release carries archives for Linux (x86\_64 and aarch64, statically linked
against musl), macOS (Apple Silicon) and Windows, plus a `.sha256` for each and
a `sha256.sum` over all of them, alongside the generated installer scripts.
`releases/latest/download/craber-installer.sh` (or `.ps1` on Windows) always
points at the newest one.

### With Docker

`ghcr.io/kobayashiyabako16g/craber` is a `distroless/static` image holding the
static musl binary and nothing else. It runs as uid 65532 and works with a
read-only root filesystem, so the output directory has to be writable by uid
65532 — pass `--user "$(id -u):$(id -g)"` otherwise.

`craber serve` is **not** usable in a container: it binds loopback inside the
container and offers no flag to change that, so a published port cannot reach
it. `export` and `build` are what the image is for.

## Development tasks

Enter the dev shell (Nix + direnv), then:

```console
$ just             # list the tasks
$ just check       # fmt, clippy and tests — what CI runs
$ just demo        # export the fixture schema as JSON
$ just svg         # export the fixture schema as an SVG you can open
$ just example     # export an examples/ schema as JSON
$ just web-dev     # Vite dev server against web/public/schema.json
$ just site        # build the static site into dist/, then `just site-serve`
$ just bindings    # regenerate web/src/types from the Rust IR
$ just nix-build   # the only check that the bundle really is in the binary
```

Without Nix you need a Rust toolchain (1.94+), Node 26 and pnpm.
