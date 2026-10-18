//! Every schema in `examples/` still exports.
//!
//! The examples are documentation, so they rot silently: nothing else in the
//! test suite reads them, and a parser regression would only show up the next
//! time someone followed the README. This walks them instead.

mod common;

use std::path::{Path, PathBuf};

use common::{assert_success, craber, examples_dir, table_names};

/// Every `*.sql` under `examples/`, one level deep — the layout the directory
/// documents, so a schema added outside it is worth failing on.
fn example_schemas() -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = std::fs::read_dir(examples_dir())
        .expect("examples/ exists")
        .filter_map(Result::ok)
        .filter(|entry| entry.path().is_dir())
        .flat_map(|entry| std::fs::read_dir(entry.path()).expect("readable example"))
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "sql"))
        .collect();
    found.sort();
    assert!(!found.is_empty(), "examples/ has no .sql files");
    found
}

fn export(path: &Path, extra: &[&str]) -> serde_json::Value {
    let mut args = vec!["export", "--file", path.to_str().unwrap()];
    args.extend_from_slice(extra);
    let output = craber(&args);
    assert_success(&output, &format!("exporting {}", path.display()));
    serde_json::from_slice(&output.stdout)
        .unwrap_or_else(|e| panic!("{} did not produce JSON: {e}", path.display()))
}

/// Dialects are inferred from the filename, exactly as the READMEs claim — the
/// commands there pass no `--dialect`, so neither does this.
#[test]
fn every_example_exports_a_laid_out_schema() {
    for path in example_schemas() {
        let schema = export(&path, &[]);
        let tables = schema["tables"].as_array().expect("tables");
        assert!(!tables.is_empty(), "{} produced no tables", path.display());
        for table in tables {
            assert!(
                table["positions"]["allFields"].is_object(),
                "{}: table {} came out without a position",
                path.display(),
                table["name"]
            );
        }
    }
}

/// The walkthrough in examples/02-filtering/README.md, asserted so the prose and
/// the tool cannot disagree.
#[test]
fn the_filtering_walkthrough_still_holds() {
    let shop = examples_dir().join("02-filtering/shop.sql");

    let all = export(&shop, &[]);
    assert_eq!(table_names(&all).len(), 13);

    let trimmed = export(
        &shop,
        &[
            "--exclude",
            "schema_migrations",
            "--exclude",
            "sessions",
            "--exclude",
            "audit_log*",
        ],
    );
    assert_eq!(
        table_names(&trimmed),
        [
            "customers",
            "addresses",
            "products",
            "categories",
            "product_categories",
            "orders",
            "order_items",
            "payments",
            "shipments",
        ]
    );

    let neighbourhood = export(&shop, &["--include", "orders", "--depth", "1"]);
    assert_eq!(
        table_names(&neighbourhood),
        [
            "customers",
            "addresses",
            "orders",
            "order_items",
            "payments",
            "shipments",
        ]
    );
}

/// `craber.toml` is only worth committing if it is equivalent to the flags it
/// replaces.
#[test]
fn the_committed_config_matches_the_flags_it_stands_in_for() {
    let shop = examples_dir().join("02-filtering/shop.sql");
    let config = examples_dir().join("02-filtering/craber.toml");

    let from_config = export(
        &shop,
        &["--config", config.to_str().unwrap(), "--format", "json"],
    );
    let from_flags = export(
        &shop,
        &[
            "--exclude",
            "schema_migrations",
            "--exclude",
            "sessions",
            "--exclude",
            "audit_log*",
        ],
    );
    assert_eq!(from_config, from_flags);
}
