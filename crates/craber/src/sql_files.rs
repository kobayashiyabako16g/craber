//! The `.sql` files beneath a directory, for the two options that take one:
//! `--file`, where they are read as one schema, and `--flow`, where each is a
//! workflow.

use std::path::{Path, PathBuf};

/// Every `.sql` file beneath `dir`, in order of their paths. Hidden entries are
/// passed over, and so are symlinked directories, which could loop.
pub(crate) fn sorted_under(dir: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut found = Vec::new();
    under(dir, &mut found)?;
    found.sort();
    Ok(found)
}

/// The `.sql` files beneath `dir` that make up a schema: [`sorted_under`]
/// without the migrations that roll one back.
pub(crate) fn schema_under(dir: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut found = sorted_under(dir)?;
    found.retain(|path| !is_rollback(path));
    Ok(found)
}

/// Whether `path` undoes a migration rather than applying one, by the naming
/// of the tools that keep the two in separate files: sqlx and golang-migrate
/// (`0001_x.down.sql`), diesel (`0001_x/down.sql`) and Flyway (`U1__x.sql`).
///
/// Read with the rest, one would put back a table a migration dropped, or take
/// out a column another added — and `.down.sql` sorts before `.up.sql`.
fn is_rollback(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return false;
    };
    let lower = name.to_ascii_lowercase();
    let flyway_undo = name.starts_with('U')
        && name[1..].starts_with(|c: char| c.is_ascii_digit())
        && name.contains("__");
    lower.ends_with(".down.sql") || lower == "down.sql" || flyway_undo
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
