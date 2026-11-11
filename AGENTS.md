# Agent Instructions

CrabER: SQL DDL or a live DB (PostgreSQL, MySQL, SQLite) → ER diagram (`craber serve`, `craber build`, JSON). One Rust binary with the React frontend compiled in. Pre-alpha.

- Write **CrabER** in prose, UI, docs and commit messages; machine-facing names (crate, command, `CRABER_` env vars, `craber.toml`) are lowercase `craber`.

## Toolchain
- Rust 1.94+, Node 26, **pnpm** (in `web/`), `just`; `direnv allow` provides all via the Nix flake.

## Commands
| Task | Command |
|------|---------|
| List tasks | `just` |
| Before pushing | `just check` (fmt-check + clippy `-D warnings` + `cargo test --workspace` + web typecheck + vitest) |
| Rust single test | `cargo test -p <crate> --test <file> <name>` |
| Accept snapshots | `cargo insta review` |
| Web typecheck + build | `just web-build` |
| Regenerate TS bindings | `just bindings` |
| Quick layout check | `just ui [file]` (then `just sample-reset`) |
| DB introspection tests | `just test-db` (Docker) |
| Everything | `just check-all` |

## External References
| Need | File |
|------|------|
| Setup, crate responsibilities, release steps | `CONTRIBUTING.md` |
| Repository layout | `docs/README.md` |
| Config options | `docs/craber.toml.example` |
| PR checklist | `.github/pull_request_template.md` |
| Frontend commands and conventions | `web/AGENTS.md` |

## Architecture
- Pipeline: input → IR (`Schema`) → filter → layout → output; every command goes through `crates/craber/src/pipeline.rs::load`.
- Layout runs in Rust (`craber-layout`); the web app reads positions from `schema.json` and never lays out tables.
- A new `--format` is added in `craber-export` only.
- CLI options resolve flag > env > `craber.toml` > default (`crates/craber/src/options/`).
- Not-yet-implemented commands/formats name their milestone in `crates/craber/src/milestone.rs`.
- `craber-introspect` is sync over a current-thread tokio runtime.

## Key Conventions
- `schema.json` is a public format: renaming/retyping a field in `crates/craber-core/src/ir/` needs a `SCHEMA_VERSION` bump and a note in the PR (`crates/craber-core/tests/wire_format.rs` snapshots it).
- Never hand-edit `web/src/types` or `web/src/generated`; edit Rust, run `just bindings`, commit the output. `web/src/types/index.ts` is the one hand-written file.
- The diagram is drawn only by `web/`; there is no image export.
- Cross-language rule must agree: `resolveRef` (`web/src/lib/schema.ts`) ↔ `SchemaIndex::resolve`.
- New resolution cases go in `crates/craber-core/tests/cases/resolve.json`, not one side's tests.
- Unsupported SQL yields a warning `Diagnostic` and the statement is skipped; only error-severity diagnostics abort.
- Never put a DSN in an error or log message.
- The web bundle is fail-soft: run `just web-build` before `cargo build` (`just dist`/`just install` do this); only `just nix-build` proves the bundle is in the binary.
- Frontend conventions live in `web/AGENTS.md`.
- Change the version only with `just bump <version>` (`crates/craber/tests/version.rs` catches drift).
- `.github/workflows/release.yml` is generated: edit `dist-workspace.toml`, run `dist generate`.
- After changing `web/pnpm-lock.yaml`, run `just nix-pnpm-hash`.

## Commits
- Conventional Commits; scope = crate name without `craber-` (`core`, `parser`, `introspect`, `layout`, `export`, `site`, `cli`, `web`).
- Explain *why* in the body.
- AI commits MUST end with a `Co-Authored-By:` line naming the agent's model.
