<div align="center">
  <img src="https://raw.githubusercontent.com/kobayashiyabako16g/craber/main/docs/assets/craber-icon.svg" width="128" height="128" alt="CrabER">
  <h3>Visualize ER diagrams from a SQL schema file — or straight from a live database.</h3>
  <p>CrabER turns DDL or a running database into an interactive ER diagram. It is one
  binary with the web frontend compiled in, so <code>craber serve</code> should feel like
  reaching for <code>psql</code>: nothing to install, a diagram in front of you. The same
  binary generates a static site for CI to publish.</p>
  <a href="https://crates.io/crates/craber"><img src="https://img.shields.io/crates/v/craber.svg" alt="crates.io"></a>
  <a href="https://github.com/kobayashiyabako16g/craber/blob/main/LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="MIT License"></a>
</div>

**Status: pre-alpha.** PostgreSQL, MySQL and SQLite — from DDL or from a live
connection — the layout algorithm, SVG output, the interactive UI and the static
site all work. Expect breaking changes between releases.

## Install

### From crates.io

Needs Rust 1.94+. The web frontend ships inside the crate, so no Node is
involved:

```console
$ cargo install craber
```

### Prebuilt binaries

Each [GitHub release](https://github.com/kobayashiyabako16g/craber/releases)
carries archives for Linux (x86\_64 and aarch64, static musl), macOS (Apple
Silicon) and Windows, plus installer scripts that fetch the right one:

```console
$ curl --proto '=https' --tlsv1.2 -LsSf https://github.com/kobayashiyabako16g/craber/releases/latest/download/craber-installer.sh | sh
```

```powershell
PS> powershell -ExecutionPolicy Bypass -c "irm https://github.com/kobayashiyabako16g/craber/releases/latest/download/craber-installer.ps1 | iex"
```

### With Nix

```console
$ nix run github:kobayashiyabako16g/craber -- export --file schema.sql --format svg -o er.svg
$ nix profile install github:kobayashiyabako16g/craber
```

### With Docker

```console
$ docker run --rm -v "$PWD:/work" --user "$(id -u):$(id -g)" \
    ghcr.io/kobayashiyabako16g/craber build --file /work/schema.sql -o /work/dist
```

### From a checkout

Needs Rust 1.94+, Node 26 and pnpm (or just Nix — `direnv allow` brings all
three):

```console
$ just install     # builds the frontend, then cargo install --path crates/craber
```

[docs/README.md](https://github.com/kobayashiyabako16g/craber/blob/main/docs/README.md#install-routes-in-detail) has the details for every route.

## Usage

Open a diagram in the browser:

```console
$ craber serve --file shop.sql
CrabER is serving shop at http://127.0.0.1:4300/
```

A table list with search, relationship lines drawn column-to-column with
crow's-foot ends, a details panel per table, draggable tables, and a URL that
carries the whole view state so a link lands where you were. `--watch` re-reads
the file when it changes.

Publish a static site:

```console
$ craber build --file schema.sql -o dist/
```

Deploys as-is — relative asset URLs, so the same directory works at `/` and
under GitHub Pages' `/<repo>/`.

Export an image or the intermediate JSON:

```console
$ craber export --file schema.sql --format svg -o er.svg
$ craber export --file schema.sql --format json
```

Read a running database instead of a file — every command takes `--dsn`, and the
scheme picks the dialect:

```console
$ export CRABER_DSN=postgres://user@host/shop
$ craber serve --dsn "$CRABER_DSN"
```

Prefer the environment variable over the command line: a DSN carries a password,
and argv is visible to every process on the machine.

Narrow a noisy dump down to the part you care about:

```console
$ craber export --file pg_dump.sql --exclude 'audit_*' --include orders --depth 1 --format svg -o er.svg
```

Anything CrabER cannot parse yet is reported on stderr and skipped, so one
unsupported statement never costs you the whole diagram.

[`examples/`](https://github.com/kobayashiyabako16g/craber/tree/main/examples) walks through all of it on schemas you can read — start
with `just example`.

## Configuration

Anything you would otherwise retype in every CI job can live in a `craber.toml`
next to the schema:

```toml
file    = "db/schema.sql"      # or dsn = "postgres://..."
exclude = ["schema_migrations", "audit_log*"]

[build]
output = "dist/"
```

With that committed, `craber build` and `craber export` take no arguments at
all. Precedence is **CLI flag > environment variable > config file > default**,
and the file is looked up in the working directory and every directory above it.
The full key reference is in [docs/README.md](https://github.com/kobayashiyabako16g/craber/blob/main/docs/README.md#configuration-reference).

## Not implemented yet

`build --single-file`, `diff`, and the `png` / `mermaid` / `dbml` export
formats. Those commands name the milestone they are scheduled for rather than
failing obscurely.

## Documentation

- [docs/README.md](https://github.com/kobayashiyabako16g/craber/blob/main/docs/README.md) — configuration reference, command details, repository layout
- [CONTRIBUTING.md](https://github.com/kobayashiyabako16g/craber/blob/main/CONTRIBUTING.md) — setup, the gates CI runs, how a release is cut

## Acknowledgements

CrabER is deeply influenced by [Liam ERD](https://github.com/liam-hq/liam). The
idea of turning a schema into a beautiful, interactive ER diagram that anyone can
open in a browser comes from Liam, and this project would not exist without it.
Our sincere gratitude and respect go to the Liam maintainers and contributors.

## License

[MIT](https://github.com/kobayashiyabako16g/craber/blob/main/LICENSE).
