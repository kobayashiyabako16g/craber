//! The `.sql` files beneath a directory, for an option that takes one —
//! `--flow`, where each is a workflow.

use std::path::{Path, PathBuf};

/// Every `.sql` file beneath `dir`, in order of their paths. Hidden entries are
/// passed over, and so are symlinked directories, which could loop.
pub(crate) fn sorted_under(dir: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut found = Vec::new();
    under(dir, &mut found)?;
    found.sort();
    Ok(found)
}

fn under(dir: &Path, found: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        if entry.file_name().to_string_lossy().starts_with('.') {
            continue;
        }
        let path = entry.path();
        if entry.file_type()?.is_dir() {
            under(&path, found)?;
        } else if path.extension().is_some_and(|e| e == "sql") && path.is_file() {
            found.push(path);
        }
    }
    Ok(())
}

/// The path of `file` from `dir`, `/`-separated and without the extension —
/// `billing/refund` — so that two subdirectories can each have a
/// `checkout.sql` and still be told apart.
pub(crate) fn name_within(dir: &Path, file: &Path) -> String {
    file.strip_prefix(dir)
        .unwrap_or(file)
        .with_extension("")
        .components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}
