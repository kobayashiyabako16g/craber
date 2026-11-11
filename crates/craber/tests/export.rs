//! End-to-end: the fixture goes in as SQL and comes out as the `schema.json`
//! every other CrabER output is built from.

mod common;

use common::{assert_success, column, craber, fixture, scratch, table};

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
