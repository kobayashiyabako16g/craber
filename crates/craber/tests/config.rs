//! `craber.toml`: what it can carry, where it is found, and what beats it.
//!
//! These tests all run the binary from a scratch directory rather than the
//! crate directory. Discovery walks up from the working directory, so a test
//! left in the repository would be reading whatever config happens to sit above
//! it — which is also why the repository root deliberately has no `craber.toml`.

mod common;

use common::{
    Server, assert_exists, assert_success, bundle_or_skip, craber_in, craber_json_in, fixture,
    scratch, table, table_names,
};

/// Write `craber.toml` into `dir` and put a copy of `tiny.sql` beside it.
fn project(name: &str, config: &str) -> std::path::PathBuf {
    let dir = scratch(name);
    std::fs::copy(fixture("tiny.sql"), dir.join("tiny.sql")).expect("fixture copy");
    std::fs::write(dir.join("craber.toml"), config).expect("config file");
    dir
}

/// The regression this file exists for. `file` in the config used to be
/// unreachable: `Source` was a clap group marked `required = true`, so the
/// command failed at argument parsing and the config was never read.
#[test]
fn the_config_can_supply_the_source() {
    let dir = project("source", "file = \"tiny.sql\"\n");
    let schema = craber_json_in(&dir, &["export", "--format", "json"]);
    assert!(
        !table_names(&schema).is_empty(),
        "the config's `file` was not read"
    );
}

/// Neither flag, nothing in the config, and the refusal has to name both ways
/// of fixing it.
#[test]
fn no_source_anywhere_is_refused() {
    let dir = scratch("no-source");
    let output = craber_in(&dir, &["export", "--format", "json"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("no schema source"), "{stderr}");
    assert!(stderr.contains("craber.toml"), "{stderr}");
}

/// A relative path in the config is resolved against the config file, not the
/// working directory, so the same repository draws the same diagram from any
/// subdirectory. This runs from `sub/`, where `./tiny.sql` does not exist.
#[test]
fn a_relative_path_resolves_against_the_config_file() {
    let dir = project("relative", "file = \"tiny.sql\"\n");
    let sub = dir.join("sub");
    std::fs::create_dir_all(&sub).expect("subdirectory");

    let schema = craber_json_in(&sub, &["export", "--format", "json"]);
    assert!(
        !table_names(&schema).is_empty(),
        "a relative `file` should resolve against the config, not the working directory"
    );
}

/// Discovery walks up, which is what made the test above run at all — stated
/// separately so a regression says which half broke.
#[test]
fn discovery_walks_up_to_the_nearest_config() {
    let dir = project("discovery", "file = \"tiny.sql\"\nexclude = [\"orders\"]\n");
    let deep = dir.join("a/b/c");
    std::fs::create_dir_all(&deep).expect("nested directories");

    let schema = craber_json_in(&deep, &["export", "--format", "json"]);
    assert!(!table_names(&schema).contains(&"orders"));
}

/// The nearer file wins outright rather than layering onto the one above it:
/// two configs in scope would otherwise make the effective settings depend on
/// how deep the command was run.
#[test]
fn the_nearest_config_wins() {
    let dir = project("nearest", "file = \"tiny.sql\"\nexclude = [\"orders\"]\n");
    let sub = dir.join("sub");
    std::fs::create_dir_all(&sub).expect("subdirectory");
    std::fs::copy(dir.join("tiny.sql"), sub.join("tiny.sql")).expect("fixture copy");
    std::fs::write(sub.join("craber.toml"), "file = \"tiny.sql\"\n").expect("nested config");

    let schema = craber_json_in(&sub, &["export", "--format", "json"]);
    assert!(table_names(&schema).contains(&"orders"));
}

/// `--file` beats the config's `file`.
#[test]
fn the_file_flag_beats_the_config() {
    let dir = project("file-flag", "file = \"missing.sql\"\n");
    let schema = craber_json_in(&dir, &["export", "--format", "json", "--file", "tiny.sql"]);
    assert!(
        !table_names(&schema).is_empty(),
        "--file should beat the config's `file`"
    );
}

/// A source named on the command line replaces the config's source outright
/// rather than merging field by field — otherwise `--dsn` in a repository whose
/// config names a `file` would quietly export the file instead of connecting.
#[test]
fn a_dsn_flag_replaces_the_configs_file() {
    let dir = project("dsn-replaces", "file = \"tiny.sql\"\n");
    let output = craber_in(
        &dir,
        &[
            "export",
            "--format",
            "json",
            "--dsn",
            "mysql://127.0.0.1:1/db",
        ],
    );
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("mysql database"), "{stderr}");
}

/// `--config` naming a file that is not there is an error, where simply having
/// no `craber.toml` anywhere is not.
#[test]
fn a_named_config_that_is_missing_is_an_error() {
    let dir = scratch("missing-config");
    let output = craber_in(
        &dir,
        &["--config", "nope.toml", "export", "--file", "tiny.sql"],
    );
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("nope.toml"), "{stderr}");
}

/// `deny_unknown_fields` is what makes a typo say so instead of doing nothing.
#[test]
fn an_unknown_key_is_refused() {
    let dir = project(
        "unknown-key",
        "file = \"tiny.sql\"\nexcludes = [\"orders\"]\n",
    );
    let output = craber_in(&dir, &["export", "--format", "json"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("excludes"), "{stderr}");
}

/// `base-path` moved under `[build]`. A config still spelling it at the top
/// level has to be told, rather than having the key silently ignored.
#[test]
fn a_top_level_base_path_is_refused() {
    let dir = project(
        "old-base-path",
        "file = \"tiny.sql\"\nbase-path = \"/x/\"\n",
    );
    let output = craber_in(&dir, &["export", "--format", "json"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("base-path"), "{stderr}");
}

/// A bad dialect is reported when the file is read, not at whichever use site
/// happens to be reached first.
#[test]
fn a_bad_dialect_is_refused_when_the_config_is_read() {
    let dir = project("bad-dialect", "file = \"tiny.sql\"\ndialect = \"oracle\"\n");
    let output = craber_in(&dir, &["export", "--format", "json"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("oracle"), "{stderr}");
}

/// The config takes the same spellings `--dialect` does, which is why the
/// dialect is parsed through `FromStr` rather than a serde derive.
#[test]
fn the_config_takes_the_dialects_aliases() {
    let dir = project(
        "dialect-alias",
        "file = \"tiny.sql\"\ndialect = \"postgresql\"\n",
    );
    let schema = craber_json_in(&dir, &["export", "--format", "json"]);
    assert!(
        !table_names(&schema).is_empty(),
        "`dialect = \"postgresql\"` was not accepted"
    );
}

/// Scope keys still reach the filter — the behaviour that already worked, kept
/// honest now that the struct has grown sections around it.
#[test]
fn scope_keys_still_reach_the_filter() {
    let dir = project(
        "scope",
        "file = \"tiny.sql\"\ninclude = [\"orders\"]\ndepth = 1\n",
    );
    let schema = craber_json_in(&dir, &["export", "--format", "json"]);
    let names = table_names(&schema);
    assert!(names.contains(&"orders"), "{names:?}");
    assert!(
        names.len() > 1,
        "depth = 1 should pull in neighbours: {names:?}"
    );
}

/// `[export] format` is honoured.
#[test]
fn the_export_section_sets_the_format() {
    let dir = project(
        "export-format",
        "file = \"tiny.sql\"\n\n[export]\nformat = \"svg\"\n",
    );
    let output = craber_in(&dir, &["export"]);
    assert_success(&output, "craber export");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.starts_with("<svg"), "{stdout:.60}");
}

/// ...and the flag still beats it. `--format` carries no clap default for this
/// reason: a default would be indistinguishable from the user asking for json,
/// and the config could never win.
#[test]
fn the_format_flag_beats_the_export_section() {
    let dir = project(
        "format-flag",
        "file = \"tiny.sql\"\n\n[export]\nformat = \"svg\"\n",
    );
    let schema = craber_json_in(&dir, &["export", "--format", "json"]);
    assert!(
        !table_names(&schema).is_empty(),
        "--format json should beat `[export] format`"
    );
}

/// With no `[export] output`, `export` writes to stdout exactly as before. This
/// is the reason command-specific keys are sectioned: a top-level `output`
/// would silently divert this into a file and break every pipeline.
#[test]
fn export_still_writes_to_stdout_by_default() {
    let dir = project(
        "stdout",
        "file = \"tiny.sql\"\n\n[build]\noutput = \"dist/\"\n",
    );
    let schema = craber_json_in(&dir, &["export", "--format", "json"]);
    assert!(
        !table_names(&schema).is_empty(),
        "export should still write the schema to stdout"
    );
    assert!(
        !dir.join("dist").exists(),
        "[build] output leaked into export"
    );
}

/// `[export] output` redirects to a file, resolved against the config.
#[test]
fn the_export_section_sets_the_output_file() {
    let dir = project(
        "export-output",
        "file = \"tiny.sql\"\n\n[export]\noutput = \"out/er.json\"\n",
    );
    let output = craber_in(&dir.join("."), &["export", "--format", "json"]);
    assert_success(&output, "craber export");
    assert_exists(&dir.join("out/er.json"));
    assert!(
        output.stdout.is_empty(),
        "stdout should be empty when the config names an output file"
    );
}

/// `[serve] port` reaches the listener.
///
/// `port = 0` is the one value that proves the key was read without pinning the
/// test to a port that might already be in use: no `--port` flag is passed, so
/// a config that did nothing would leave the default and the server would
/// announce 4300. The server prints the port it actually bound, which is what
/// this reads back.
#[test]
fn the_serve_section_sets_the_port() {
    if !bundle_or_skip() {
        return;
    }
    let dir = project("serve-port", "file = \"tiny.sql\"\n\n[serve]\nport = 0\n");
    let server = Server::start_in(&dir, &["serve"]);
    assert_ne!(server.port, 0, "the OS always hands back a real port");
    assert_ne!(server.port, 4300, "the config's port was ignored");
}

/// The scope flags keep replacing the config's lists rather than adding to
/// them, which is the rule `pipeline::pick` encodes.
#[test]
fn a_scope_flag_replaces_the_configs_list() {
    let dir = project("replace", "file = \"tiny.sql\"\nexclude = [\"orders\"]\n");
    let schema = craber_json_in(
        &dir,
        &["export", "--format", "json", "--exclude", "customers"],
    );
    let names = table_names(&schema);
    assert!(names.contains(&"orders"), "{names:?}");
    assert!(!names.contains(&"customers"), "{names:?}");
}

/// Keeps `table` in use from this file's imports and checks the schema is a
/// real one rather than an empty shell.
#[test]
fn the_config_sourced_schema_is_the_same_one_the_flag_produces() {
    let dir = project("equivalence", "file = \"tiny.sql\"\n");
    let from_config = craber_json_in(&dir, &["export", "--format", "json"]);
    let from_flag = craber_json_in(&dir, &["export", "--format", "json", "--file", "tiny.sql"]);
    assert_eq!(from_config, from_flag);
    assert!(
        table(&from_config, "orders")["columns"]
            .as_array()
            .is_some()
    );
}
