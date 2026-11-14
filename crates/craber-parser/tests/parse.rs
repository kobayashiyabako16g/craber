use craber_core::Dialect;
use craber_core::{Action, Cardinality, Severity};
use craber_parser::parse;

fn postgres(sql: &str) -> craber_core::Outcome<craber_core::Schema> {
    parse(sql, Dialect::Postgres, "test")
}

fn mysql(sql: &str) -> craber_core::Outcome<craber_core::Schema> {
    parse(sql, Dialect::MySql, "test")
}

fn sqlite(sql: &str) -> craber_core::Outcome<craber_core::Schema> {
    parse(sql, Dialect::Sqlite, "test")
}

#[test]
fn reads_columns_nullability_and_defaults() {
    let outcome = postgres(
        "CREATE TABLE users (id bigint PRIMARY KEY, email text NOT NULL, nickname text DEFAULT 'anon');",
    );
    let users = outcome.value.table("users").unwrap();
    assert_eq!(users.pk, ["id"]);
    assert_eq!(users.columns.len(), 3);
    // A column-level PRIMARY KEY implies NOT NULL even unspoken.
    assert!(!users.column("id").unwrap().nullable);
    assert!(!users.column("email").unwrap().nullable);
    assert!(users.column("nickname").unwrap().nullable);
    assert_eq!(
        users.column("nickname").unwrap().default.as_deref(),
        Some("'anon'")
    );
}

#[test]
fn reads_schema_qualified_names() {
    let outcome = postgres("CREATE TABLE analytics.events (id bigint PRIMARY KEY);");
    let events = outcome.value.table("events").unwrap();
    assert_eq!(events.schema.as_deref(), Some("analytics"));
    assert_eq!(events.qualified_name(), "analytics.events");
}

#[test]
fn reads_column_level_references() {
    let outcome = postgres(
        "CREATE TABLE users (id bigint PRIMARY KEY);
         CREATE TABLE orders (
             id bigint PRIMARY KEY,
             user_id bigint REFERENCES users (id) ON DELETE CASCADE
         );",
    );
    let fk = &outcome.value.table("orders").unwrap().fks[0];
    assert_eq!(fk.columns, ["user_id"]);
    assert_eq!(fk.ref_table.name, "users");
    assert_eq!(fk.ref_columns, ["id"]);
    assert_eq!(fk.on_delete, Action::Cascade);
    assert!(!fk.inferred);
}

#[test]
fn reads_composite_primary_keys_and_table_level_foreign_keys() {
    let outcome = postgres(
        "CREATE TABLE order_items (
             order_id bigint NOT NULL,
             product_id bigint NOT NULL,
             PRIMARY KEY (order_id, product_id),
             CONSTRAINT items_order_fkey FOREIGN KEY (order_id) REFERENCES orders (id)
         );",
    );
    let items = outcome.value.table("order_items").unwrap();
    assert_eq!(items.pk, ["order_id", "product_id"]);
    assert_eq!(items.fks[0].name.as_deref(), Some("items_order_fkey"));
    assert_eq!(items.fks[0].columns, ["order_id"]);
}

#[test]
fn a_foreign_key_over_a_unique_column_is_one_to_one() {
    let outcome = postgres(
        "CREATE TABLE users (id bigint PRIMARY KEY);
         CREATE TABLE profiles (
             user_id bigint PRIMARY KEY REFERENCES users (id),
             bio text
         );
         CREATE TABLE orders (
             id bigint PRIMARY KEY,
             user_id bigint REFERENCES users (id)
         );",
    );
    assert_eq!(
        outcome.value.table("profiles").unwrap().fks[0].cardinality,
        Cardinality::OneToOne
    );
    assert_eq!(
        outcome.value.table("orders").unwrap().fks[0].cardinality,
        Cardinality::ManyToOne
    );
}

/// The pg_dump shape: the constraint arrives long after the tables do, and
/// refers backwards. A single-pass translator would drop it.
#[test]
fn reads_foreign_keys_added_by_alter_table() {
    let outcome = postgres(
        "CREATE TABLE orders (id bigint PRIMARY KEY);
         CREATE TABLE order_items (order_id bigint NOT NULL);
         ALTER TABLE ONLY public.order_items
             ADD CONSTRAINT items_order_fkey
             FOREIGN KEY (order_id) REFERENCES public.orders (id) ON DELETE CASCADE;",
    );
    let fk = &outcome.value.table("order_items").unwrap().fks[0];
    assert_eq!(fk.name.as_deref(), Some("items_order_fkey"));
    assert_eq!(fk.ref_table.name, "orders");
    assert_eq!(fk.on_delete, Action::Cascade);
    assert!(outcome.diagnostics.is_empty(), "{:?}", outcome.diagnostics);
}

/// An `ALTER TABLE` finds its table by the rule a `REFERENCES` does
/// (`SchemaIndex::resolve`): `audit.users` is not `public.users`, however
/// unambiguous the bare name is.
#[test]
fn alter_table_does_not_amend_a_table_in_another_schema() {
    let outcome = postgres(
        "CREATE TABLE public.users (id bigint PRIMARY KEY);
         ALTER TABLE audit.users ADD COLUMN note text;",
    );
    let users = outcome.value.table("users").unwrap();
    assert_eq!(users.columns.len(), 1);
}

/// Cardinality is derived from uniqueness, so it has to be computed after the
/// second pass — the PRIMARY KEY that makes this one-to-one arrives late.
#[test]
fn alter_table_primary_keys_still_decide_cardinality() {
    let outcome = postgres(
        "CREATE TABLE users (id bigint PRIMARY KEY);
         CREATE TABLE profiles (user_id bigint NOT NULL);
         ALTER TABLE ONLY profiles ADD CONSTRAINT profiles_pkey PRIMARY KEY (user_id);
         ALTER TABLE ONLY profiles
             ADD CONSTRAINT profiles_user_fkey FOREIGN KEY (user_id) REFERENCES users (id);",
    );
    let profiles = outcome.value.table("profiles").unwrap();
    assert_eq!(profiles.pk, ["user_id"]);
    assert_eq!(profiles.fks[0].cardinality, Cardinality::OneToOne);
}

#[test]
fn reads_indexes_created_after_the_table() {
    let outcome = postgres(
        "CREATE TABLE orders (id bigint PRIMARY KEY, user_id bigint);
         CREATE INDEX orders_user_id_idx ON orders (user_id);
         CREATE UNIQUE INDEX orders_id_key ON public.orders (id);",
    );
    let orders = outcome.value.table("orders").unwrap();
    assert_eq!(orders.indexes.len(), 2);
    assert_eq!(
        orders.indexes[0].name.as_deref(),
        Some("orders_user_id_idx")
    );
    assert!(!orders.indexes[0].unique);
    assert!(orders.indexes[1].unique);
    assert!(outcome.diagnostics.is_empty(), "{:?}", outcome.diagnostics);
}

#[test]
fn reads_enum_types_and_comments() {
    let outcome = postgres(
        "CREATE TYPE public.order_status AS ENUM ('pending', 'paid');
         CREATE TABLE orders (id bigint PRIMARY KEY, status public.order_status);
         COMMENT ON TABLE public.orders IS 'One per checkout.';
         COMMENT ON COLUMN public.orders.status IS 'Where it is in the flow.';",
    );
    let status = &outcome.value.enums[0];
    assert_eq!(status.schema.as_deref(), Some("public"));
    assert_eq!(status.name, "order_status");
    assert_eq!(status.values, ["pending", "paid"]);

    let orders = outcome.value.table("orders").unwrap();
    assert_eq!(orders.comment.as_deref(), Some("One per checkout."));
    assert_eq!(
        orders.column("status").unwrap().comment.as_deref(),
        Some("Where it is in the flow.")
    );
}

/// Dumps alter things CrabER never drew — views, partitions, tables the user
/// filtered out. Those must not become warnings, or the noise buries the ones
/// that matter.
#[test]
fn statements_about_unknown_tables_are_ignored() {
    let outcome = postgres(
        "CREATE TABLE users (id bigint PRIMARY KEY);
         ALTER TABLE ONLY public.audit ADD CONSTRAINT audit_pkey PRIMARY KEY (id);
         CREATE INDEX audit_at_idx ON public.audit (at);
         COMMENT ON TABLE public.audit IS 'not a table CrabER knows';",
    );
    assert_eq!(outcome.value.tables.len(), 1);
    assert!(outcome.diagnostics.is_empty(), "{:?}", outcome.diagnostics);
}

#[test]
fn unsupported_statements_warn_but_do_not_lose_the_schema() {
    let outcome = postgres(
        "CREATE TABLE users (id bigint PRIMARY KEY, nickname text);
         ALTER TABLE ONLY users ALTER COLUMN nickname SET NOT NULL;",
    );
    assert_eq!(outcome.value.tables.len(), 1);
    assert!(!outcome.has_errors());
    assert_eq!(outcome.diagnostics.len(), 1);
    assert_eq!(outcome.diagnostics[0].severity, Severity::Warning);
}

/// pg_dump emits these by the dozen and none of them change the picture.
#[test]
fn bookkeeping_alterations_are_skipped_silently() {
    let outcome = postgres(
        "CREATE TABLE users (id bigint PRIMARY KEY);
         ALTER TABLE public.users OWNER TO postgres;
         ALTER TABLE ONLY public.users ENABLE ROW LEVEL SECURITY;",
    );
    assert!(outcome.diagnostics.is_empty(), "{:?}", outcome.diagnostics);
}

#[test]
fn irrelevant_statements_are_skipped_silently() {
    let outcome = postgres("GRANT SELECT ON users TO readonly;");
    assert!(outcome.diagnostics.is_empty(), "{:?}", outcome.diagnostics);
}

#[test]
fn unparsable_input_is_an_error_not_a_panic() {
    let outcome = postgres("this is not sql at all");
    assert!(outcome.has_errors());
    assert!(
        outcome.value.tables.is_empty(),
        "{:?}",
        outcome.value.tables
    );
}

#[test]
fn dialect_parses_from_its_cli_spelling() {
    assert_eq!("postgres".parse::<Dialect>().unwrap(), Dialect::Postgres);
    assert_eq!("mariadb".parse::<Dialect>().unwrap(), Dialect::MySql);
    assert_eq!("sqlite3".parse::<Dialect>().unwrap(), Dialect::Sqlite);
    assert!("oracle".parse::<Dialect>().is_err());
}

// MySQL. The dialect decides the tokenizer, but everything below is about what
// the translator does with what comes out of it.

/// A `mysqldump` has one `KEY` line per index, and they are the bulk of a
/// CREATE TABLE. Losing them lost the details panel's index list; warning about
/// them buried the diagnostics that matter under one line per index.
#[test]
fn mysql_key_is_a_plain_index_and_not_a_complaint() {
    let outcome = mysql(
        "CREATE TABLE `posts` (
           `id` bigint unsigned NOT NULL AUTO_INCREMENT,
           `user_id` bigint unsigned NOT NULL,
           `slug` varchar(190) NOT NULL,
           PRIMARY KEY (`id`),
           UNIQUE KEY `posts_slug_uniq` (`slug`),
           KEY `posts_user_idx` (`user_id`)
         ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;",
    );
    assert!(outcome.diagnostics.is_empty(), "{:?}", outcome.diagnostics);

    let posts = outcome.value.table("posts").unwrap();
    // Backticks are the tokenizer's business: the identifiers arrive bare.
    assert_eq!(posts.name, "posts");
    assert_eq!(posts.pk, ["id"]);

    let unique = posts.indexes.iter().find(|i| i.unique).unwrap();
    assert_eq!(unique.name.as_deref(), Some("posts_slug_uniq"));
    assert_eq!(unique.columns, ["slug"]);

    let plain = posts.indexes.iter().find(|i| !i.unique).unwrap();
    assert_eq!(plain.name.as_deref(), Some("posts_user_idx"));
    assert_eq!(plain.columns, ["user_id"]);
}

/// `ALTER TABLE ... ADD KEY` arrives as the same constraint, so it has to land
/// the same way — a dump puts some indexes in the table and some in the tail.
#[test]
fn mysql_add_key_after_the_fact_is_the_same_index() {
    let outcome = mysql(
        "CREATE TABLE `posts` (`id` bigint NOT NULL, `slug` varchar(190) NOT NULL, PRIMARY KEY (`id`));
         ALTER TABLE `posts` ADD KEY `posts_slug_idx` (`slug`);",
    );
    assert!(outcome.diagnostics.is_empty(), "{:?}", outcome.diagnostics);
    let posts = outcome.value.table("posts").unwrap();
    assert_eq!(posts.indexes.len(), 1);
    assert!(!posts.indexes[0].unique);
    assert_eq!(posts.indexes[0].columns, ["slug"]);
}

/// `REFERENCES users` with no column list means the primary key, in every
/// dialect. Left empty, the line has no column to attach to and the details
/// panel's SQL would read `REFERENCES "users" ()`.
#[test]
fn a_reference_without_columns_means_the_primary_key() {
    let outcome = postgres(
        "CREATE TABLE users (id bigint PRIMARY KEY);
         CREATE TABLE logs (id bigint);
         CREATE TABLE posts (user_id bigint REFERENCES users, log_id bigint REFERENCES logs);",
    );
    let posts = outcome.value.table("posts").unwrap();
    assert_eq!(
        posts.fks.len(),
        1,
        "the key into a table with no primary key is dropped"
    );
    assert_eq!(posts.fks[0].ref_columns, ["id"]);
    assert_eq!(outcome.diagnostics.len(), 1, "{:?}", outcome.diagnostics);
    assert_eq!(outcome.diagnostics[0].severity, Severity::Warning);
}

/// A key that names no columns can only be given its target's primary key
/// when there is one of the right width. Otherwise it is skipped with a
/// warning, never left with an empty or mismatched column list.
#[test]
fn a_reference_without_columns_is_skipped_when_no_key_pairs_with_it() {
    let outcome = postgres(
        "CREATE TABLE pairs (a int, b int, PRIMARY KEY (a, b));
         CREATE TABLE c (x int REFERENCES pairs, y int REFERENCES gone);",
    );
    let c = outcome.value.table("c").unwrap();
    assert!(c.fks.is_empty(), "{:?}", c.fks);
    let messages: Vec<&str> = outcome
        .diagnostics
        .iter()
        .map(|d| d.message.as_str())
        .collect();
    assert_eq!(messages.len(), 2, "{messages:?}");
    assert!(
        messages.iter().any(|m| m.contains("2 columns, not 1")),
        "{messages:?}"
    );
    assert!(
        messages.iter().any(|m| m.contains("not in the schema")),
        "{messages:?}"
    );
}

/// The web app keys tables by qualified name and would see one of two
/// same-named tables, while `SchemaIndex` would see both and call every
/// reference to them ambiguous. Keeping the first is what makes them agree.
#[test]
fn a_table_defined_twice_keeps_its_first_definition() {
    let outcome = postgres(
        "CREATE TABLE users (id bigint PRIMARY KEY);
         CREATE TABLE users (id bigint PRIMARY KEY, email text);
         CREATE TABLE orders (user_id bigint REFERENCES public.users (id));",
    );
    let users: Vec<_> = outcome
        .value
        .tables
        .iter()
        .filter(|t| t.name == "users")
        .collect();
    assert_eq!(users.len(), 1);
    assert_eq!(users[0].columns.len(), 1, "the first definition wins");
    assert_eq!(outcome.diagnostics.len(), 1, "{:?}", outcome.diagnostics);
    let index = craber_core::SchemaIndex::build(&outcome.value);
    assert_eq!(
        index.edges(&outcome.value).len(),
        1,
        "the reference resolves"
    );
}

/// A partial unique index constrains only the rows its `WHERE` matches, so the
/// tuple is not unique. Introspection already reads it that way; a file has to
/// agree, or the same schema draws a different relationship end.
#[test]
fn a_partial_unique_index_is_not_uniqueness() {
    let outcome = postgres(
        "CREATE TABLE users (id bigint PRIMARY KEY);
         CREATE TABLE posts (id bigint PRIMARY KEY, user_id bigint REFERENCES users (id), live boolean);
         CREATE UNIQUE INDEX posts_live_user ON posts (user_id) WHERE live;",
    );
    let posts = outcome.value.table("posts").unwrap();
    assert!(!posts.indexes[0].unique);
    assert_eq!(posts.fks[0].cardinality, Cardinality::ManyToOne);
}

/// A non-unique index must not be mistaken for uniqueness, or every FK through
/// an indexed column would come out one-to-one.
#[test]
fn a_plain_index_does_not_make_a_relationship_one_to_one() {
    let outcome = mysql(
        "CREATE TABLE `users` (`id` bigint NOT NULL, PRIMARY KEY (`id`));
         CREATE TABLE `posts` (
           `id` bigint NOT NULL,
           `user_id` bigint NOT NULL,
           PRIMARY KEY (`id`),
           KEY `posts_user_idx` (`user_id`),
           CONSTRAINT `posts_user_fk` FOREIGN KEY (`user_id`) REFERENCES `users` (`id`)
         );",
    );
    let posts = outcome.value.table("posts").unwrap();
    assert!(!posts.is_unique_tuple(&["user_id".to_string()]));
    assert_eq!(posts.fks[0].cardinality, Cardinality::ManyToOne);
}

/// `UNIQUE (parent_id, position)` lets a parent have many children; only the
/// pair is unique. liam-hq/liam#4173 drew this one-to-one.
#[test]
fn a_composite_unique_does_not_make_its_prefix_one_to_one() {
    let outcome = postgres(
        "CREATE TABLE parents (id integer PRIMARY KEY);
         CREATE TABLE children (
           id integer PRIMARY KEY,
           parent_id integer NOT NULL REFERENCES parents(id),
           position integer NOT NULL,
           UNIQUE (parent_id, position)
         );",
    );
    let children = outcome.value.table("children").unwrap();
    assert!(children.is_unique_tuple(&["parent_id".to_string(), "position".to_string()]));
    assert_eq!(children.fks[0].cardinality, Cardinality::ManyToOne);
    assert!(!children.fks[0].optional);
}

/// liam-hq/liam#4110 showed `text[]` as `text`.
#[test]
fn array_types_keep_their_brackets() {
    let outcome = postgres("CREATE TABLE addresses (email_to TEXT[] NOT NULL, grid int[][]);");
    let addresses = outcome.value.table("addresses").unwrap();
    assert_eq!(addresses.column("email_to").unwrap().data_type, "TEXT[]");
    assert_eq!(addresses.column("grid").unwrap().data_type, "INT[][]");
}

/// A full-text (or spatial) key is an access path with nothing to draw. Not
/// showing it is right; warning about it is not.
#[test]
fn mysql_fulltext_key_is_skipped_silently() {
    let outcome = mysql(
        "CREATE TABLE `users` (`id` bigint NOT NULL, `bio` text, PRIMARY KEY (`id`), FULLTEXT KEY `users_bio_ft` (`bio`));",
    );
    assert!(outcome.diagnostics.is_empty(), "{:?}", outcome.diagnostics);
    let indexes = &outcome.value.table("users").unwrap().indexes;
    assert!(indexes.is_empty(), "{indexes:?}");
}

/// MySQL hides the table comment among the trailing table options rather than
/// in the `COMMENT` clause, which is why it used to vanish.
#[test]
fn mysql_table_comment_comes_from_the_table_options() {
    let outcome = mysql(
        "CREATE TABLE `users` (
           `id` bigint NOT NULL,
           `bio` text COMMENT 'free-form profile text',
           PRIMARY KEY (`id`)
         ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COMMENT='One row per person who can sign in.';",
    );
    let users = outcome.value.table("users").unwrap();
    assert_eq!(
        users.comment.as_deref(),
        Some("One row per person who can sign in.")
    );
    assert_eq!(
        users.column("bio").unwrap().comment.as_deref(),
        Some("free-form profile text")
    );
}

// SQLite.

/// `sqlite3 .schema` prints the engine's own bookkeeping table next to the real
/// ones. It is part of the file, not part of the model.
#[test]
fn sqlite_internal_tables_stay_out_of_the_diagram() {
    let outcome = sqlite(
        "CREATE TABLE sqlite_sequence(name,seq);
         CREATE TABLE IF NOT EXISTS \"authors\" (\"id\" integer PRIMARY KEY AUTOINCREMENT NOT NULL);",
    );
    assert!(outcome.diagnostics.is_empty(), "{:?}", outcome.diagnostics);
    let names: Vec<_> = outcome.value.tables.iter().map(|t| &t.name).collect();
    assert_eq!(names, ["authors"]);
}

/// The same name is an ordinary table anywhere else, so the rule is the
/// dialect's, not the name's.
#[test]
fn a_sqlite_prefixed_table_survives_in_another_dialect() {
    let outcome = postgres("CREATE TABLE sqlite_sequence (name text, seq bigint);");
    assert_eq!(outcome.value.tables.len(), 1);
}

/// `INTEGER PRIMARY KEY` is SQLite's rowid alias, and column-level `REFERENCES`
/// is how its foreign keys are almost always written — including self-joins.
#[test]
fn sqlite_rowid_alias_and_inline_references() {
    let outcome = sqlite(
        "CREATE TABLE [books] (
           [id] INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
           [title] TEXT NOT NULL UNIQUE,
           [sequel_to] INTEGER REFERENCES [books]([id]) ON DELETE SET NULL
         ) WITHOUT ROWID;",
    );
    assert!(outcome.diagnostics.is_empty(), "{:?}", outcome.diagnostics);

    // Bracket quoting is the tokenizer's business; the names arrive bare.
    let books = outcome.value.table("books").unwrap();
    assert_eq!(books.pk, ["id"]);
    assert!(!books.column("id").unwrap().nullable);

    assert_eq!(books.fks.len(), 1);
    assert_eq!(books.fks[0].columns, ["sequel_to"]);
    assert_eq!(books.fks[0].ref_table.name, "books");
    assert_eq!(books.fks[0].ref_columns, ["id"]);
    assert_eq!(books.fks[0].on_delete, Action::SetNull);
    // Nullable FK column: the line ends "zero or one" on the parent side.
    assert!(books.fks[0].optional);
}

fn bigquery(sql: &str) -> craber_core::Outcome<craber_core::Schema> {
    parse(sql, Dialect::BigQuery, "test")
}

/// A project is not a namespace the diagram draws, so `project.dataset.table`
/// keeps the last two parts, quoted as one identifier or not.
#[test]
fn bigquery_names_keep_dataset_and_table() {
    let outcome = bigquery(
        "CREATE TABLE `my-project.shop.users` (id INT64);
         CREATE TABLE `my-project`.shop.orders (id INT64);",
    );
    assert!(outcome.diagnostics.is_empty(), "{:?}", outcome.diagnostics);
    for name in ["users", "orders"] {
        let table = outcome.value.table(name).unwrap();
        assert_eq!(table.schema.as_deref(), Some("shop"), "{name}");
    }
}

/// BigQuery keys are always `NOT ENFORCED`. They still describe the model, so
/// they are drawn like any other key.
#[test]
fn bigquery_unenforced_keys_are_still_keys() {
    let outcome = bigquery(
        "CREATE TABLE shop.users (id INT64 PRIMARY KEY NOT ENFORCED);
         CREATE TABLE shop.orders (
             id INT64,
             user_id INT64 REFERENCES shop.users (id) NOT ENFORCED,
             PRIMARY KEY (id) NOT ENFORCED
         );
         CREATE TABLE shop.items (id INT64, order_id INT64);
         ALTER TABLE shop.items ADD PRIMARY KEY (id) NOT ENFORCED;
         ALTER TABLE shop.items ADD CONSTRAINT items_order_fk
             FOREIGN KEY (order_id) REFERENCES shop.orders (id) NOT ENFORCED;",
    );
    assert!(outcome.diagnostics.is_empty(), "{:?}", outcome.diagnostics);

    assert_eq!(outcome.value.table("users").unwrap().pk, ["id"]);

    let orders = outcome.value.table("orders").unwrap();
    assert_eq!(orders.pk, ["id"]);
    assert_eq!(orders.fks[0].columns, ["user_id"]);
    assert_eq!(orders.fks[0].ref_table.qualified_name(), "shop.users");

    let items = outcome.value.table("items").unwrap();
    assert_eq!(items.pk, ["id"]);
    assert_eq!(items.fks[0].name.as_deref(), Some("items_order_fk"));
    assert_eq!(items.fks[0].ref_table.qualified_name(), "shop.orders");
}

#[test]
fn bigquery_nested_types_are_shown_as_written() {
    let outcome = bigquery(
        "CREATE TABLE shop.events (
             tags ARRAY<STRING>,
             payload STRUCT<kind STRING, n INT64>
         ) PARTITION BY DATE(_PARTITIONTIME) CLUSTER BY kind;",
    );
    assert!(outcome.diagnostics.is_empty(), "{:?}", outcome.diagnostics);
    let events = outcome.value.table("events").unwrap();
    assert_eq!(events.column("tags").unwrap().data_type, "ARRAY<STRING>");
    assert_eq!(
        events.column("payload").unwrap().data_type,
        "STRUCT<kind STRING, n INT64>"
    );
}

#[test]
fn bigquery_descriptions_are_comments() {
    let outcome = bigquery(
        "CREATE TABLE shop.users (
             id INT64 OPTIONS(description = 'surrogate key'),
             email STRING OPTIONS(description = \"where receipts go\")
         ) OPTIONS(description = 'One row per customer', labels = [('team', 'growth')]);",
    );
    assert!(outcome.diagnostics.is_empty(), "{:?}", outcome.diagnostics);
    let users = outcome.value.table("users").unwrap();
    assert_eq!(users.comment.as_deref(), Some("One row per customer"));
    assert_eq!(
        users.column("id").unwrap().comment.as_deref(),
        Some("surrogate key")
    );
    assert_eq!(
        users.column("email").unwrap().comment.as_deref(),
        Some("where receipts go")
    );
}
