//! End-to-end: the fixture goes in as SQL and comes out as the `schema.json`
//! every other CrabER output is built from.

mod common;

use common::{assert_success, column, craber, fixture, scratch, table, table_names};

#[test]
fn exports_the_fixture_as_json() {
    let output = craber(&["export", "--file", fixture("tiny.sql").to_str().unwrap()]);
    assert_success(&output, "craber export");

    let schema: serde_json::Value = serde_json::from_slice(&output.stdout).expect("valid JSON");
    insta::assert_json_snapshot!(schema);
}

#[test]
fn warnings_go_to_stderr_so_stdout_stays_pipeable() {
    let output = craber(&["export", "--file", fixture("tiny.sql").to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("warning:"),
        "expected a warning about the unapplied ALTER COLUMN, got: {stderr}"
    );
    serde_json::from_slice::<serde_json::Value>(&output.stdout).expect("stdout is pure JSON");
}

#[test]
fn depth_narrows_the_export_to_a_neighbourhood() {
    let output = craber(&[
        "export",
        "--file",
        fixture("tiny.sql").to_str().unwrap(),
        "--include",
        "users",
        "--depth",
        "1",
    ]);
    assert_success(&output, "craber export");
    let schema: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let names: Vec<&str> = schema["tables"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    // One hop from `users`: the order that references it and the profile that
    // does — but not `coupons`, which is two hops away through `orders`.
    assert_eq!(names, ["users", "user_profiles", "orders"]);
}

/// The diagram is drawn by the browser only; an image format would be a second
/// renderer drawing a different picture of the same schema.
#[test]
fn image_formats_are_not_offered() {
    for format in ["svg", "png"] {
        let output = craber(&[
            "export",
            "--file",
            fixture("tiny.sql").to_str().unwrap(),
            "--format",
            format,
        ]);
        assert!(!output.status.success(), "--format {format} should fail");
        assert!(output.stdout.is_empty(), "nothing should reach stdout");
    }
}

/// `--dialect POSTGRES` works, so `--format JSON` has to as well.
#[test]
fn the_format_flag_ignores_case() {
    let output = craber(&[
        "export",
        "--file",
        fixture("tiny.sql").to_str().unwrap(),
        "--format",
        "JSON",
    ]);
    assert_success(&output, "craber export --format JSON");
}

/// A format that is declared but not built yet has to fail loudly. Writing an
/// empty file, or silently falling back to another format, would be worse than
/// the error.
#[test]
fn unimplemented_formats_say_so_instead_of_writing_nothing() {
    for format in ["mermaid", "dbml"] {
        let output = craber(&[
            "export",
            "--file",
            fixture("tiny.sql").to_str().unwrap(),
            "--format",
            format,
        ]);
        assert!(!output.status.success(), "--format {format} should fail");
        assert!(String::from_utf8_lossy(&output.stderr).contains("not implemented"));
        assert!(output.stdout.is_empty(), "nothing should reach stdout");
    }
}

/// The fixture is also the notation sample: every relationship end CrabER can
/// derive from DDL appears in it exactly once. The ends themselves are drawn
/// from these two facts (`craber_core::normalize`), so this is
/// where a change in how they are derived shows up — in the diagram it would
/// only look like a different picture.
#[test]
fn the_fixture_covers_every_relationship_end() {
    let output = craber(&["export", "--file", fixture("tiny.sql").to_str().unwrap()]);
    let schema: serde_json::Value = serde_json::from_slice(&output.stdout).expect("valid JSON");

    let ends: Vec<(String, &str, bool)> = schema["tables"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|table| {
            table["fks"].as_array().into_iter().flatten().map(|fk| {
                (
                    format!(
                        "{}.{}",
                        table["name"].as_str().unwrap(),
                        fk["columns"][0].as_str().unwrap()
                    ),
                    fk["cardinality"].as_str().unwrap(),
                    fk["optional"].as_bool().unwrap(),
                )
            })
        })
        .collect();
    let end = |name: &str| {
        ends.iter()
            .find(|(column, ..)| column == name)
            .unwrap_or_else(|| panic!("no foreign key on {name}"))
    };

    // Not unique, not nullable: many rows here, and each has exactly one there.
    assert_eq!(end("orders.user_id").1, "manyToOne");
    assert!(!end("orders.user_id").2);
    // Nullable: the row it points at may not be there at all.
    assert_eq!(end("orders.coupon_id").1, "manyToOne");
    assert!(end("orders.coupon_id").2);
    // Unique and not nullable: one row at each end, and both are required.
    assert_eq!(end("user_profiles.user_id").1, "oneToOne");
    assert!(!end("user_profiles.user_id").2);
    // Unique and nullable: one row at each end, and either may be missing.
    assert_eq!(end("order_receipts.order_id").1, "oneToOne");
    assert!(end("order_receipts.order_id").2);
    // A composite primary key does not make its first column unique, and its
    // columns are NOT NULL whether or not the DDL said so per column.
    assert_eq!(end("order_items.order_id").1, "manyToOne");
    assert!(!end("order_items.order_id").2);
}

/// Comments are the only free-form prose in the IR, and the web app's details
/// panel is built around them — it decides per value whether it fits beside its
/// label or needs the panel's full width. `fixtures/comments.sql` carries the
/// shapes that decision exists for, so the pipeline has to hand them over whole:
/// truncating a comment here would look like a frontend bug.
#[test]
fn comments_and_defaults_survive_the_pipeline_whole() {
    let output = craber(&[
        "export",
        "--file",
        fixture("comments.sql").to_str().unwrap(),
    ]);
    assert_success(&output, "craber export");
    assert_eq!(
        String::from_utf8_lossy(&output.stderr).trim(),
        "",
        "the fixture is ordinary PostgreSQL and should parse cleanly"
    );

    let schema: serde_json::Value = serde_json::from_slice(&output.stdout).expect("valid JSON");
    let invoices = table(&schema, "invoices");

    let table_comment = invoices["comment"].as_str().expect("table comment");
    assert!(
        table_comment.starts_with("One row per issued invoice.")
            && table_comment.ends_with("for the tax authority."),
        "the paragraph should arrive whole: {table_comment}"
    );

    // Non-ASCII, and with no spaces to break at: anything along the way that
    // sliced bytes rather than characters would cut this one in half.
    let cjk = column(invoices, "total_cents")["comment"]
        .as_str()
        .expect("column comment");
    assert!(cjk.starts_with('請') && cjk.ends_with('。'), "{cjk}");

    let short = column(invoices, "number")["comment"].as_str().unwrap();
    assert_eq!(short, "Human-facing invoice number.");

    let default = column(invoices, "external_ref")["default"]
        .as_str()
        .unwrap();
    assert!(default.contains("nextval("), "{default}");
}

#[test]
fn a_missing_source_is_rejected_by_the_argument_parser() {
    let output = craber(&["export"]);
    assert!(!output.status.success());
}

#[test]
fn flows_are_exported_one_workflow_per_file() {
    let output = craber(&[
        "export",
        "--file",
        fixture("tiny.sql").to_str().unwrap(),
        "--flow",
        fixture("flows/checkout.sql").to_str().unwrap(),
        "--flow",
        fixture("flows/account-closure.sql").to_str().unwrap(),
        "--format",
        "flows",
    ]);
    assert_success(&output, "craber export --format flows");

    let flows: serde_json::Value = serde_json::from_slice(&output.stdout).expect("valid JSON");
    let names: Vec<&str> = flows["workflows"]
        .as_array()
        .unwrap()
        .iter()
        .map(|w| w["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["checkout", "account-closure"]);
    assert_eq!(flows["workflows"][0]["steps"].as_array().unwrap().len(), 6);
}

#[test]
fn a_workflow_table_the_diagram_lacks_is_a_warning() {
    let output = craber(&[
        "export",
        "--file",
        fixture("tiny.sql").to_str().unwrap(),
        "--exclude",
        "coupons",
        "--flow",
        fixture("flows/checkout.sql").to_str().unwrap(),
        "--format",
        "flows",
    ]);
    assert_success(&output, "craber export --format flows");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("public.coupons"),
        "expected a warning naming the excluded table, got: {stderr}"
    );
}

/// The names of the workflows in `flows.json` on `output`'s stdout.
fn workflow_names(output: &std::process::Output) -> Vec<String> {
    let flows: serde_json::Value = serde_json::from_slice(&output.stdout).expect("valid JSON");
    flows["workflows"]
        .as_array()
        .unwrap()
        .iter()
        .map(|w| w["name"].as_str().unwrap().to_owned())
        .collect()
}

fn export_flows(flows: &[&std::path::Path]) -> std::process::Output {
    let tiny = fixture("tiny.sql");
    let mut args = vec!["export", "--file", tiny.to_str().unwrap()];
    for flow in flows {
        args.extend(["--flow", flow.to_str().unwrap()]);
    }
    args.extend(["--format", "flows"]);
    craber(&args)
}

#[test]
fn a_flow_directory_reads_every_sql_file_beneath_it_named_by_its_path() {
    let dir = scratch("flow-dir");
    let write = |path: &str| {
        let path = dir.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, "SELECT * FROM public.users;").unwrap();
    };
    write("checkout.sql");
    write("billing/refund.sql");
    write("billing/nested/checkout.sql");
    write("notes.txt");
    write(".drafts/hidden.sql");

    let output = export_flows(&[&dir]);
    assert_success(&output, "craber export --flow <dir>");
    assert_eq!(
        workflow_names(&output),
        ["billing/nested/checkout", "billing/refund", "checkout"]
    );
}

#[test]
fn a_flow_directory_and_a_file_can_be_given_together() {
    let output = export_flows(&[&fixture("flows"), &fixture("flows/checkout.sql")]);
    assert_success(&output, "craber export --flow <dir> --flow <file>");
    assert_eq!(
        workflow_names(&output),
        ["account-closure", "checkout", "checkout"]
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("two workflows are named checkout"),
        "expected a warning about the repeated name, got: {stderr}"
    );
}

#[test]
fn an_empty_flow_directory_is_a_warning() {
    let dir = scratch("flow-dir-empty");
    let output = export_flows(&[&dir]);
    assert_success(&output, "craber export --flow <empty dir>");
    assert_eq!(workflow_names(&output), Vec::<String>::new());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("no .sql files under workflow directory"),
        "expected a warning about the empty directory, got: {stderr}"
    );
}

/// A directory of migrations is one script in path order: a later file's
/// `ALTER TABLE` amends the table an earlier one created, and a file that is
/// hidden or not `.sql` is not read.
#[test]
fn a_file_directory_reads_every_sql_file_beneath_it_as_one_schema() {
    let dir = scratch("schema-dir").join("migrations");
    let write = |path: &str, sql: &str| {
        let path = dir.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, sql).unwrap();
    };
    write(
        "001_users.sql",
        "CREATE TABLE users (id bigint PRIMARY KEY);",
    );
    write(
        "002_orders.sql",
        "CREATE TABLE orders (id bigint PRIMARY KEY, user_id bigint NOT NULL);",
    );
    write(
        "billing/003_order_fk.sql",
        "-- only a comment up here\n\
         ALTER TABLE orders ADD FOREIGN KEY (user_id) REFERENCES users (id);",
    );
    write("004_empty.sql", "-- nothing yet\n");
    write("notes.txt", "CREATE TABLE notes (id bigint);");
    write(".drafts/005_hidden.sql", "CREATE TABLE hidden (id bigint);");

    let output = craber(&["export", "--file", dir.to_str().unwrap()]);
    assert_success(&output, "craber export --file <dir>");
    assert_eq!(String::from_utf8_lossy(&output.stderr).trim(), "");
    let schema: serde_json::Value = serde_json::from_slice(&output.stdout).expect("valid JSON");
    assert_eq!(schema["name"], "migrations");
    let mut names = table_names(&schema);
    names.sort_unstable();
    assert_eq!(names, ["orders", "users"]);
    assert_eq!(
        table(&schema, "orders")["fks"][0]["refTable"]["name"],
        "users"
    );
}

/// A warning names the file it came from, since `line 2` alone could be any
/// of them.
#[test]
fn a_warning_from_a_file_directory_names_the_file() {
    let dir = scratch("schema-dir-warning");
    std::fs::write(
        dir.join("a.sql"),
        "CREATE TABLE users (id bigint PRIMARY KEY, nickname text);",
    )
    .unwrap();
    std::fs::write(
        dir.join("b.sql"),
        "\nCREATE TABLE broken (id bigint) FROBNICATE;",
    )
    .unwrap();

    let output = craber(&["export", "--file", dir.to_str().unwrap()]);
    assert_success(&output, "craber export --file <dir>");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("b.sql line 2"),
        "expected the warning to name b.sql, got: {stderr}"
    );
}

/// One file that cannot be read loses only that file, as one statement that
/// cannot be read loses only that statement.
#[test]
fn an_unreadable_file_in_a_directory_is_a_warning() {
    let dir = scratch("schema-dir-unreadable");
    std::fs::write(
        dir.join("a.sql"),
        "CREATE TABLE users (id bigint PRIMARY KEY);",
    )
    .unwrap();
    std::fs::write(dir.join("b.sql"), "this is not sql at all").unwrap();

    let output = craber(&["export", "--file", dir.to_str().unwrap()]);
    assert_success(&output, "craber export --file <dir>");
    let schema: serde_json::Value = serde_json::from_slice(&output.stdout).expect("valid JSON");
    assert_eq!(table_names(&schema), ["users"]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("(b.sql)"),
        "expected a warning naming b.sql, got: {stderr}"
    );
}

#[test]
fn a_directory_of_which_no_file_can_be_read_is_an_error() {
    let dir = scratch("schema-dir-nothing");
    std::fs::write(dir.join("a.sql"), "this is not sql at all").unwrap();

    let output = craber(&["export", "--file", dir.to_str().unwrap()]);
    assert!(!output.status.success(), "expected failure");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("(a.sql)"),
        "expected the error to name a.sql, got: {stderr}"
    );
}

#[test]
fn an_empty_file_directory_is_an_error() {
    let dir = scratch("schema-dir-empty");
    let output = craber(&["export", "--file", dir.to_str().unwrap()]);
    assert!(!output.status.success(), "expected failure");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("no .sql files under schema directory"),
        "expected an error about the empty directory, got: {stderr}"
    );
}

/// The directory's name is evidence of the dialect, as a file's is: backticks
/// would not parse as Postgres.
#[test]
fn a_file_directory_infers_the_dialect_from_its_name() {
    let dir = scratch("schema-dir-dialect").join("mysql");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("a.sql"),
        "CREATE TABLE `users` (`id` bigint PRIMARY KEY);",
    )
    .unwrap();

    let output = craber(&["export", "--file", dir.to_str().unwrap()]);
    assert_success(&output, "craber export --file mysql/");
    let schema: serde_json::Value = serde_json::from_slice(&output.stdout).expect("valid JSON");
    assert_eq!(table_names(&schema), ["users"]);
}

/// Read with the rest, a down migration would put back a table an up dropped
/// and take out a column another added — and `.down.sql` sorts before
/// `.up.sql`. Each tool's naming is passed over: sqlx and golang-migrate,
/// diesel, Flyway.
#[test]
fn a_file_directory_skips_down_migrations() {
    let dir = scratch("schema-dir-down");
    let write = |path: &str, sql: &str| {
        let path = dir.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, sql).unwrap();
    };
    write(
        "0001_users.up.sql",
        "CREATE TABLE users (id bigint PRIMARY KEY, nickname text);",
    );
    write("0001_users.down.sql", "DROP TABLE users;");
    write("0002_ghost.up.sql", "DROP TABLE ghost;");
    write(
        "0002_ghost.down.sql",
        "CREATE TABLE ghost (id bigint PRIMARY KEY);",
    );
    write(
        "0003_nickname.down.sql",
        "ALTER TABLE users DROP COLUMN nickname;",
    );
    write(
        "2024_posts/up.sql",
        "CREATE TABLE posts (id bigint PRIMARY KEY);",
    );
    write(
        "2024_posts/down.sql",
        "CREATE TABLE diesel_ghost (id bigint PRIMARY KEY);",
    );
    write(
        "U4__flyway.sql",
        "CREATE TABLE flyway_ghost (id bigint PRIMARY KEY);",
    );

    let output = craber(&["export", "--file", dir.to_str().unwrap()]);
    assert_success(&output, "craber export --file <migrations>");
    assert_eq!(String::from_utf8_lossy(&output.stderr).trim(), "");
    let schema: serde_json::Value = serde_json::from_slice(&output.stdout).expect("valid JSON");
    let mut names = table_names(&schema);
    names.sort_unstable();
    assert_eq!(names, ["posts", "users"]);
    column(table(&schema, "users"), "nickname");
}

#[test]
fn a_directory_of_only_down_migrations_is_an_error() {
    let dir = scratch("schema-dir-only-down");
    std::fs::write(dir.join("0001_x.down.sql"), "DROP TABLE x;").unwrap();

    let output = craber(&["export", "--file", dir.to_str().unwrap()]);
    assert!(!output.status.success(), "expected failure");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("other than down migrations"),
        "expected the error to mention down migrations, got: {stderr}"
    );
}
