//! The internal crates' dependency versions stay equal to the workspace version.
//!
//! `[workspace.dependencies]` has to repeat the version literally for every
//! internal crate (see the comment there), and nothing in a normal build notices
//! when one is left behind: `^0.1.1` is satisfied by a workspace at `0.1.2`, so
//! the drift only surfaces as a published crate depending on an older sibling.

use std::path::PathBuf;

#[test]
fn internal_dependencies_match_the_workspace_version() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Cargo.toml");
    let manifest: toml::Table = std::fs::read_to_string(&path)
        .expect("read the workspace Cargo.toml")
        .parse()
        .expect("parse the workspace Cargo.toml");
    let workspace = &manifest["workspace"];
    let version = workspace["package"]["version"].as_str().unwrap();

    let stale: Vec<String> = workspace["dependencies"]
        .as_table()
        .unwrap()
        .iter()
        .filter(|(_, dep)| dep.get("path").is_some())
        .filter_map(|(name, dep)| {
            let pinned = dep.get("version").and_then(toml::Value::as_str);
            (pinned != Some(version)).then(|| format!("{name} = {pinned:?}"))
        })
        .collect();

    assert!(
        stale.is_empty(),
        "workspace is at {version}, but these internal dependencies are not: {stale:?}\n\
         change the version only with `just bump <version>`",
    );
}
