//! The sqlparser-rs dialect behind each [`Dialect`].
//!
//! [`Dialect`] itself lives in `craber-core`: it names what `--dialect`, a
//! filename and a connection string scheme all select, and `craber-introspect`
//! needs it without needing sqlparser. Only the mapping to a parser stays here.

pub use craber_core::{Dialect, UnknownDialect};

/// A free function rather than a method: `Dialect` is a foreign type now, so it
/// cannot grow an inherent impl in this crate.
pub(crate) fn sqlparser_dialect(dialect: Dialect) -> Box<dyn sqlparser::dialect::Dialect> {
    match dialect {
        Dialect::Postgres => Box::new(sqlparser::dialect::PostgreSqlDialect {}),
        Dialect::MySql => Box::new(sqlparser::dialect::MySqlDialect {}),
        Dialect::Sqlite => Box::new(sqlparser::dialect::SQLiteDialect {}),
    }
}
