# 02 — Narrowing the diagram

A 13-table storefront schema, four of which nobody wants on an ER diagram:
`schema_migrations`, `sessions`, `audit_log`, `audit_log_archive`.

Start by seeing all of it:

```console
$ cargo run -p craber -- export --file examples/02-filtering/shop.sql --format json
```

## `--exclude`

```console
$ cargo run -p craber -- export --file examples/02-filtering/shop.sql \
    --exclude schema_migrations --exclude sessions --exclude 'audit_log*'
```

Nine tables left. Patterns are globs, matched against both the bare name
(`orders`) and the qualified one (`public.orders`), so the same pattern works
whether or not the dump carried schema qualifiers. `--exclude` and `--include`
are repeatable.

Dropping a table also drops the foreign keys pointing at it — `audit_log` had an
FK to `customers`, and no edge is left dangling behind it.

## `craber.toml`

Those three exclusions belong in the repository, not in everyone's shell
history. That is what [`craber.toml`](craber.toml) is for — and since it names
the schema too, the whole invocation collapses to:

```console
$ cargo run -p craber -- export --config examples/02-filtering/craber.toml --format json
```

Same nine tables. `file = "shop.sql"` is relative to `craber.toml` rather than
to the working directory, so that command draws the same diagram from anywhere.

`--config` is a global flag and works with every subcommand. Without it CrabER
looks for `craber.toml` in the working directory and then upwards, so running
inside a repository that has one needs no flag at all.

CLI flags *replace* the file's lists rather than adding to them: passing
`--exclude sessions` on the command line means the three patterns in the file no
longer apply. A `--file` or `--dsn` on the command line likewise replaces the
file's source outright.

## `--include`

```console
$ cargo run -p craber -- export --file examples/02-filtering/shop.sql \
    --include 'order*' --include products
```

Three tables: `products`, `orders`, `order_items`. `orders` keeps its FK to
`products` via `order_items`, but its FKs to `customers` and `addresses` are
gone with those tables.

## `--depth`

A hand-written `--include` list goes stale the moment someone adds a table.
`--depth` grows the selection outwards over the foreign-key graph instead:

```console
$ cargo run -p craber -- export --file examples/02-filtering/shop.sql \
    --include orders --depth 1
```

Six tables — `orders` plus everything one FK hop away in either direction:
`customers` and `addresses` (which `orders` references), and `order_items`,
`payments`, `shipments` (which reference `orders`). `products` is two hops out,
so it stays off the diagram.

`--exclude` still wins at any depth: an excluded table is a wall, not merely an
unselected one, so the walk does not travel through it.

## Cardinality

Look at `payments` in the output. Its FK to `orders` reports
`"cardinality": "oneToOne"`, while every other FK here is `"manyToOne"` — the
difference is the `UNIQUE` on `payments.order_id`. CrabER reads uniqueness off
the DDL rather than asking you to annotate the relationship.
