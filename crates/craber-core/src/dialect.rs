//! The SQL dialects CrabER can read.
//!
//! This lives in `craber-core` rather than in `craber-parser` because a dialect
//! is not a parsing concept: it is what `--dialect` names, what a filename
//! implies, and what a connection string's scheme says. `craber-introspect`
//! needs it and must not depend on `craber-parser`, which would drag
//! sqlparser-rs into a crate that never parses SQL.

use std::fmt;
use std::path::Path;
use std::str::FromStr;

use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Dialect {
    #[default]
    Postgres,
    MySql,
    Sqlite,
}

/// Every way a dialect is spelled, in one place per dialect.
///
/// `--dialect`, a connection string and a filename each accept a different
/// set of words — `pg` names a dialect in a filename but is not a connection
/// scheme, and `file:` is a scheme but not a `--dialect` value — so the sets
/// are kept apart, but side by side, where adding a dialect means filling in
/// one of these rather than editing four `match`es.
struct Spelling {
    /// What `Display` writes and `--help` lists.
    name: &'static str,
    /// Other values `--dialect` and `craber.toml` accept.
    aliases: &'static [&'static str],
    /// Connection string schemes.
    schemes: &'static [&'static str],
    /// Substrings of a filename that imply this dialect.
    filename_hints: &'static [&'static str],
}

impl Dialect {
    /// Every dialect, in the order error messages list them.
    pub const ALL: [Self; 3] = [Self::Postgres, Self::MySql, Self::Sqlite];

    /// An exhaustive `match`, so a new variant does not compile until it is
    /// spelled.
    const fn spelling(self) -> Spelling {
        match self {
            Self::Postgres => Spelling {
                name: "postgres",
                aliases: &["postgresql", "pg"],
                schemes: &["postgres", "postgresql"],
                filename_hints: &["postgres", "pg"],
            },
            Self::MySql => Spelling {
                name: "mysql",
                aliases: &["mariadb"],
                schemes: &["mysql", "mariadb"],
                filename_hints: &["mysql"],
            },
            Self::Sqlite => Spelling {
                name: "sqlite",
                aliases: &["sqlite3"],
                schemes: &["sqlite", "sqlite3", "file"],
                filename_hints: &["sqlite"],
            },
        }
    }

    /// Guess from a file extension or filename, for when `--dialect` is omitted.
    ///
    /// Postgres is asked last: its `pg` hint is short enough to turn up inside
    /// another dialect's name.
    pub fn from_path(path: &Path) -> Option<Self> {
        let name = path.file_name()?.to_str()?.to_ascii_lowercase();
        [Self::MySql, Self::Sqlite, Self::Postgres]
            .into_iter()
            .find(|dialect| {
                dialect
                    .spelling()
                    .filename_hints
                    .iter()
                    .any(|hint| name.contains(hint))
            })
    }

    /// Guess from a connection string's scheme, for when `--dialect` is omitted.
    ///
    /// Both `sqlite://app.db` and `sqlite:app.db` are forms sqlx accepts, which
    /// is why the bare `scheme:` case is looked at too. A path with no scheme at
    /// all is not decided here — the caller knows whether it is holding a file
    /// path, and [`from_path`](Self::from_path) is the function for that.
    pub fn from_dsn(dsn: &str) -> Option<Self> {
        let scheme = match dsn.split_once("://") {
            Some((scheme, _)) => scheme,
            None => dsn.split_once(':')?.0,
        }
        .to_ascii_lowercase();
        Self::ALL
            .into_iter()
            .find(|dialect| dialect.spelling().schemes.contains(&scheme.as_str()))
    }

    /// True for a table the engine maintains rather than the schema author.
    ///
    /// `sqlite3 .schema` prints `sqlite_sequence` next to the real tables
    /// whenever anything uses AUTOINCREMENT, and `sqlite_stat1` once ANALYZE has
    /// run. They are part of the database, not part of the model, and drawing
    /// them puts a typeless node in the middle of the diagram. A file and a
    /// live connection both skip them by this rule.
    pub fn is_engine_table(self, name: &str) -> bool {
        match self {
            Self::Sqlite => name.to_ascii_lowercase().starts_with("sqlite_"),
            Self::Postgres | Self::MySql => false,
        }
    }
}

impl fmt::Display for Dialect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.spelling().name)
    }
}

/// Built from [`Dialect::ALL`] rather than written out, so a new dialect is
/// listed the moment it exists.
#[derive(Debug, Error)]
#[error("unknown dialect `{0}`, expected one of: {expected}", expected = expected())]
pub struct UnknownDialect(String);

fn expected() -> String {
    Dialect::ALL.map(|dialect| dialect.to_string()).join(", ")
}

impl FromStr for Dialect {
    type Err = UnknownDialect;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let lower = s.to_ascii_lowercase();
        Self::ALL
            .into_iter()
            .find(|dialect| {
                let spelling = dialect.spelling();
                spelling.name == lower || spelling.aliases.contains(&lower.as_str())
            })
            .ok_or(UnknownDialect(lower))
    }
}
