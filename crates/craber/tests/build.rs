//! End-to-end: `craber build` writes a directory a static host can serve.
//!
//! The assertions are about the deployable shape — relative asset URLs, one
//! `schema.json` next to `index.html` — rather than about the bundle's
//! contents, which belong to `web/` and change with every frontend build.

mod common;

use std::path::Path;

use common::{
    assert_exists, assert_success, asset_references, bundle_or_skip, craber, fixture, out_dir,
};

fn build(out: &Path, extra: &[&str]) -> std::process::Output {
    let tiny = fixture("tiny.sql");
    let mut args = vec![
        "build",
        "--file",
        tiny.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
    ];
    args.extend_from_slice(extra);
    craber(&args)
}

#[test]
fn writes_a_servable_directory() {
    if !bundle_or_skip() {
        return;
    }
    let out = out_dir("servable");
    let output = build(&out, &[]);
    assert_success(&output, "craber build");

    let index = std::fs::read_to_string(out.join("index.html")).expect("index.html");
    // Relative asset URLs are what lets the same directory be served from `/`
    // and from `/<repo>/` without rebuilding.
    assert!(
        index.contains("./assets/"),
        "assets should be referenced relatively: {index}"
    );
    for reference in asset_references(&index) {
        assert!(
            out.join(&reference).is_file(),
            "index.html references {reference}, which was not written"
        );
    }

    // The site must carry the schema it was built for, not the sample the dev
    // server happened to be pointed at when the bundle was built.
    let schema: serde_json::Value =
        serde_json::from_slice(&std::fs::read(out.join("schema.json")).expect("schema.json"))
            .expect("valid JSON");
    assert_eq!(schema["name"], "tiny");
    assert_eq!(schema["schemaVersion"], 2);
    let tables = schema["tables"].as_array().expect("tables");
    assert_eq!(tables.len(), 6, "every table in fixtures/tiny.sql");
    // Coordinates are baked in by craber-layout — one set per detail level,
    // since a node's height depends on how much of the table is drawn. The
    // frontend picks a level; it never lays out.
    assert!(
        tables.iter().all(|t| ["tableName", "keyOnly", "allFields"]
            .iter()
            .all(|level| t["positions"][level].is_object())),
        "every table needs a position per detail level: {schema}"
    );

    assert_exists(&out.join("serve.json"));
    std::fs::remove_dir_all(&out).ok();
}

#[test]
fn base_path_rewrites_the_asset_urls() {
    if !bundle_or_skip() {
        return;
    }
    let out = out_dir("base-path");
    let output = build(&out, &["--base-path", "/myrepo/"]);
    assert_success(&output, "craber build");

    let index = std::fs::read_to_string(out.join("index.html")).expect("index.html");
    assert!(
        !index.contains("./assets/"),
        "no relative asset left: {index}"
    );
    for reference in asset_references(&index) {
        let relative = reference.strip_prefix("/myrepo/").unwrap_or_else(|| {
            panic!("{reference} should sit under the base path");
        });
        assert_exists(&out.join(relative));
    }
    std::fs::remove_dir_all(&out).ok();
}

/// `craber.toml` carries `base-path` under `[build]` so that a repository
/// publishing to a subdirectory states it once instead of in every CI
/// invocation. The flag and the file have to reach the same code, or the
/// documented key does nothing.
#[test]
fn base_path_can_come_from_the_config_file() {
    if !bundle_or_skip() {
        return;
    }
    let out = out_dir("base-path-config");
    std::fs::create_dir_all(&out).expect("output directory");
    let config = out.join("craber.toml");
    std::fs::write(&config, "[build]\nbase-path = \"/fromconfig/\"\n").expect("config file");

    let output = build(&out, &["--config", config.to_str().unwrap()]);
    assert_success(&output, "building with a config-supplied base path");

    let index = std::fs::read_to_string(out.join("index.html")).expect("index.html");
    assert!(
        !index.contains("./assets/"),
        "the config's base path was ignored: {index}"
    );
    for reference in asset_references(&index) {
        assert!(
            reference.starts_with("/fromconfig/"),
            "{reference} should sit under the config's base path"
        );
    }
    std::fs::remove_dir_all(&out).ok();
}

/// The flag still wins when both name a prefix.
#[test]
fn the_base_path_flag_overrides_the_config_file() {
    if !bundle_or_skip() {
        return;
    }
    let out = out_dir("base-path-precedence");
    std::fs::create_dir_all(&out).expect("output directory");
    let config = out.join("craber.toml");
    std::fs::write(&config, "[build]\nbase-path = \"/fromconfig/\"\n").expect("config file");

    let output = build(
        &out,
        &[
            "--config",
            config.to_str().unwrap(),
            "--base-path",
            "/fromflag/",
        ],
    );
    assert_success(&output, "building with both a flag and a config");

    let index = std::fs::read_to_string(out.join("index.html")).expect("index.html");
    for reference in asset_references(&index) {
        assert!(
            reference.starts_with("/fromflag/"),
            "{reference} should come from the flag, not the config"
        );
    }
    std::fs::remove_dir_all(&out).ok();
}

/// Asset filenames carry a content hash, so a new bundle lands beside the old
/// one. A directory published into repeatedly has to end up with the current
/// build, not with every build it has ever held — and without touching files
/// that were not CrabER's to begin with.
#[test]
fn rebuilding_clears_the_previous_bundle_but_not_other_files() {
    if !bundle_or_skip() {
        return;
    }
    let out = out_dir("rebuild");
    assert!(build(&out, &[]).status.success());

    // A leftover from an older bundle: recorded as ours, so it goes.
    let stale = out.join("assets/index-STALE.js");
    std::fs::write(&stale, b"// an older build").unwrap();
    let manifest = out.join(".craber-build.json");
    let mut recorded: Vec<String> =
        serde_json::from_slice(&std::fs::read(&manifest).unwrap()).unwrap();
    recorded.push("assets/index-STALE.js".to_owned());
    std::fs::write(&manifest, serde_json::to_vec(&recorded).unwrap()).unwrap();

    // Somebody else's file, which the build has no business deleting.
    let theirs = out.join("CNAME");
    std::fs::write(&theirs, b"er.example.com").unwrap();

    assert!(build(&out, &[]).status.success());
    assert!(
        !stale.exists(),
        "the previous bundle's asset should be gone"
    );
    assert!(theirs.exists(), "CNAME was not CrabER's to delete");
    assert_exists(&out.join("index.html"));
    std::fs::remove_dir_all(&out).ok();
}

/// `--single-file` is declared but not built. Writing a directory instead —
/// which is what the user would find — would be worse than the error.
#[test]
fn single_file_says_it_is_not_implemented() {
    let out = out_dir("single-file");
    let output = build(&out, &["--single-file"]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("not implemented"));
    assert!(!out.exists(), "nothing should have been written");
}

#[test]
fn a_missing_bundle_is_reported_rather_than_written_as_a_broken_site() {
    if craber_site::is_embedded() {
        return;
    }
    let out = out_dir("no-bundle");
    let output = build(&out, &[]);
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("just web-build"),
        "the error should say how to get a bundle: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
