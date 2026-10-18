# CrabER development tasks. Run `just` to list them.

default:
    @just --list

# Format Rust sources
fmt:
    cargo fmt --all

# Verify formatting without writing
fmt-check:
    cargo fmt --all -- --check

# Lint with clippy, warnings are errors
lint:
    cargo clippy --workspace --all-targets -- -D warnings

# Also regenerates web/src/types and web/src/generated — see `bindings`.
#
# Run the Rust test suite
test:
    cargo test --workspace

# Runs the whole craber-core suite rather than naming the two generator tests:
# `cargo test` takes only one filter, and a new generator should not have to
# remember to add itself here.
#
# Regenerate the TypeScript types and design tokens the web app gets from Rust
bindings:
    cargo test -p craber-core

# Everything CI runs on the Rust side
check: fmt-check lint test

# The pure modules only — `graph.ts`, `anchors.ts`, `schema.ts`, `sql.ts`,
# `viewState.ts`. Those are React-free by design; this is what checks them.
# A component needs a browser, which is what `.claude/skills/ui-check` is for.
#
# Run the frontend unit tests
web-test:
    cd web && pnpm test

# Everything CI runs, both sides
check-all: check web-build web-test

# Emit schema.json for a fixture, to eyeball the pipeline
demo file="fixtures/tiny.sql":
    cargo run -p craber -- export --file {{file}} --format json

# Render a schema to SVG, the quickest way to eyeball the layout
svg file="fixtures/tiny.sql" out="er.svg":
    cargo run -p craber -- export --file {{file}} --format svg -o {{out}}

# Run one of the examples in examples/ (see examples/README.md)
example file="examples/01-quickstart/blog.sql":
    cargo run -p craber -- export --file {{file}} --format json

# Refresh the sample schema.json the web dev server reads
sample file="fixtures/tiny.sql":
    cargo run -p craber -- export --file {{file}} -o web/public/schema.json

# Serve a schema interactively, the way `craber serve` does for a user
serve file="fixtures/tiny.sql":
    cargo run -p craber -- serve --file {{file}}

# Ports are offset from the defaults so these never collide with a real server
# someone has running. Nothing here is reachable from off the machine.
#
# Start throwaway PostgreSQL and MySQL servers for the --dsn recipes
db-up:
    docker run -d --rm --name craber-pg \
        -e POSTGRES_PASSWORD=craber -p 55432:5432 postgres:17-alpine
    docker run -d --rm --name craber-mysql \
        -e MYSQL_ROOT_PASSWORD=craber -e MYSQL_DATABASE=craber -p 53306:3306 mysql:8.4

# Two things this has to handle. The tail of tiny.sql is a statement left
# unapplied on purpose, which a real server rejects — and one rejected
# statement rolls the whole batch back, hence the sed. And `postgres`'s
# entrypoint runs a temporary server during initialisation that answers
# `pg_isready` and then shuts down, so waiting means waiting for a connection
# that stays up, not for the first one that answers.
#
# Load a fixture into the PostgreSQL from `just db-up`
db-load file="fixtures/tiny.sql":
    #!/usr/bin/env bash
    set -euo pipefail
    for _ in $(seq 1 60); do
        if docker exec craber-pg psql -q -U postgres -c 'SELECT 1' >/dev/null 2>&1; then
            sleep 1
            sed '/^-- Not applied:/,$d' {{file}} | docker exec -i craber-pg psql -q -U postgres
            exit 0
        fi
        sleep 1
    done
    echo "craber-pg never became reachable; is it running? (just db-up)" >&2
    exit 1

db-down:
    -docker rm -f craber-pg craber-mysql

# `just demo`, but reading a live database instead of a file
introspect dsn="postgres://postgres:craber@127.0.0.1:55432/postgres":
    cargo run -p craber -- export --dsn {{dsn}} --format json

# `just ui`, but reading a live database. `just sample-reset` afterwards.
ui-dsn dsn="postgres://postgres:craber@127.0.0.1:55432/postgres":
    cargo run -p craber -- export --dsn {{dsn}} -o web/public/schema.json
    cd web && pnpm dev

# Needs a Docker daemon; `just test` skips these and reports them as ignored.
#
# Run the introspection tests against real PostgreSQL and MySQL servers
test-db:
    cargo test -p craber-introspect -- --ignored --test-threads=1

# Depends on web-build because the bundle is compiled into the binary: a
# stale web/dist would be published as the site.
#
# Build the deployable static site for a schema, into dist/
site file="fixtures/tiny.sql" out="dist": web-build
    cargo run -p craber -- build --file {{file}} -o {{out}}

# `just serve` serves from memory and never writes a directory, so this is
# still the only way to check what `craber build` actually put on disk.
#
# Serve a built directory the way a static host would
site-serve out="dist":
    cd {{out}} && python3 -m http.server 4321

# The order matters and is otherwise written down nowhere: the web bundle is
# compiled into the binary, so `cargo build` has to run after
# `web-build` or it produces a binary that cannot write or serve a site.
#
# Build a release binary, frontend included
dist: web-build
    cargo build --release
    @echo "built target/release/craber"

# `web/dist` sits outside the package directory, so a `.crate` tarball cannot
# contain it. This is the copy that ships. Left untracked: `web/` is the source
# of truth and `just vendor-bundle` regenerates it.
#
# Copy the built frontend into craber-site, where `cargo package` can see it
vendor-bundle: web-build
    rm -rf crates/craber-site/bundle
    cp -R web/dist crates/craber-site/bundle
    # Whatever schema the dev server last looked at is not part of the bundle;
    # `craber build` writes the real one. Matches the skip in build.rs.
    rm -f crates/craber-site/bundle/schema.json

# Install `craber` onto this machine from the working tree.
install: web-build
    cargo install --path crates/craber --locked

# Check the crate would package correctly, without publishing anything.
package-check: vendor-bundle
    cargo package -p craber-site --allow-dirty --list | grep bundle/ | head

# Install web dependencies
web-install:
    cd web && pnpm install

# Vite dev server
web-dev: bindings
    cd web && pnpm dev

# Typecheck + production build of the web app
web-build: bindings
    cd web && pnpm build

# Unlike web-dev this skips bindings (a cargo test), so getting a diagram on
# screen does not wait on the Rust side when nothing there moved.
#
# Export a schema into the web app and start the dev server, to look at it
ui file="fixtures/tiny.sql": (sample file)
    cd web && pnpm dev

# Put web/public/schema.json back after looking at another schema
sample-reset:
    git restore web/public/schema.json

# Extends `package-check`: the same vendored bundle, but it also compiles each
# packaged crate against the others as crates.io will see them. dist's dry-run
# path never calls the publish job, and a published version can only be yanked —
# never replaced — so this is the only place a mistake is still cheap.
#
# Rehearse the crates.io publish without uploading anything
publish-check: vendor-bundle
    cargo publish --workspace --allow-dirty --dry-run

# The only path that proves the bundle actually reached the binary: build.rs
# merely warns when it finds none and tests/build.rs passes by
# skipping, so the flake's installCheckPhase renders a real site instead.
#
# Build the release binary through the flake, frontend included
nix-build:
    nix build --print-build-logs .#craber

# pnpmDeps is a fixed-output derivation, so its hash pins the exact pnpm store
# for web/pnpm-lock.yaml and goes stale the moment that lockfile moves. There is
# no way to ask Nix for the hash except to let the fetch fail and read it off
# the mismatch.
#
# Rewrite the pnpmDeps hash in flake.nix after a lockfile change
nix-pnpm-hash:
    #!/usr/bin/env bash
    set -euo pipefail
    old=$(grep -oE 'sha256-[A-Za-z0-9+/=]+' flake.nix | head -1)
    # Ask for a hash that cannot be right, so the fetch fails and reports the one
    # that is. lib.fakeHash is the conventional spelling of "I do not know yet".
    perl -pi -e "s/\Qhash = \"$old\";\E/hash = lib.fakeHash;/" flake.nix
    new=$(nix build .#craber-web 2>&1 | grep -oE 'got: +sha256-[A-Za-z0-9+/=]+' | grep -oE 'sha256-[A-Za-z0-9+/=]+' || true)
    # Either way flake.nix must not be left holding fakeHash.
    perl -pi -e "s/\Qhash = lib.fakeHash;\E/hash = \"${new:-$old}\";/" flake.nix
    if [ -z "$new" ]; then
        echo "nix reported no mismatch: $old was already correct" >&2
    elif [ "$new" = "$old" ]; then
        echo "unchanged: $old" >&2
    else
        echo "$old -> $new" >&2
    fi

# The image copies binaries dist built on native Linux runners; nothing here can
# produce a static musl binary from a Darwin checkout. So this checks the parts
# that are this repository's to get wrong — that the Dockerfile parses and that
# both architectures resolve into one manifest list — and leaves the binaries to
# CI. Populate docker-stage/{amd64,arm64}/craber from a release to go further.
#
# Build the container image for both architectures, pushing nothing
docker-check:
    #!/usr/bin/env bash
    set -euo pipefail
    for arch in amd64 arm64; do
        if [ ! -e "docker-stage/$arch/craber" ]; then
            mkdir -p "docker-stage/$arch"
            printf '#!/bin/sh\nexit 0\n' > "docker-stage/$arch/craber"
            echo "docker-stage/$arch/craber was missing; stubbed it for this check" >&2
        fi
    done
    # The default driver cannot emit a multi-platform manifest list.
    docker buildx create --name craber-check --driver docker-container --bootstrap 2>/dev/null || true
    docker buildx build . --builder craber-check \
        --platform linux/amd64,linux/arm64 --provenance=false --output=type=cacheonly

# Bumps [workspace.package] and the eight path dependencies in
# [workspace.dependencies] together — `cargo publish` refuses a path dependency
# with no version, so the number lives in nine places and hand-editing it is how
# a release ships a crate depending on a version that was never published.
# Refuses to go backwards, which is the behaviour you want out of a release step.
#
# Set the workspace version everywhere it appears
bump version:
    cargo set-version --workspace {{version}}
    @echo "now tag it: git tag v{{version}} && git push origin v{{version}}"
