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

impl Dialect {
    /// Guess from a file extension or filename, for when `--dialect` is omitted.
    pub fn from_path(path: &Path) -> Option<Self> {
        let name = path.file_name()?.to_str()?.to_ascii_lowercase();
        if name.contains("mysql") {
            Some(Self::MySql)
        } else if name.contains("sqlite") {
            Some(Self::Sqlite)
        } else if name.contains("postgres") || name.contains("pg") {
            Some(Self::Postgres)
        } else {
            None
        }
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
        };
        match scheme.to_ascii_lowercase().as_str() {
            "postgres" | "postgresql" => Some(Self::Postgres),
            "mysql" | "mariadb" => Some(Self::MySql),
            "sqlite" | "sqlite3" | "file" => Some(Self::Sqlite),
            _ => None,
        }
    }
}

impl fmt::Display for Dialect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Postgres => "postgres",
            Self::MySql => "mysql",
            Self::Sqlite => "sqlite",
        })
    }
}

#[derive(Debug, Error)]
#[error("unknown dialect `{0}`, expected one of: postgres, mysql, sqlite")]
pub struct UnknownDialect(String);

impl FromStr for Dialect {
    type Err = UnknownDialect;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "postgres" | "postgresql" | "pg" => Ok(Self::Postgres),
            "mysql" | "mariadb" => Ok(Self::MySql),
            "sqlite" | "sqlite3" => Ok(Self::Sqlite),
            other => Err(UnknownDialect(other.to_string())),
        }
    }
}
