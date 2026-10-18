//! A live database to [`Schema`].
//!
//! The twin of `craber-parser`: same output, a different source. Where the
//! parser reads a dump, this connects and asks the server's own catalog.
//!
//! # The runtime stops here
//!
//! [`introspect`] is synchronous. `craber` is synchronous top to bottom —
//! `main() -> Result<()>`, a parked thread in `serve` — and the async runtime
//! is confined to this crate. A current-thread runtime is built inside that
//! function, used for a handful of catalog queries, and dropped before it
//! returns.
//!
//! Every query is a runtime [`sqlx::query()`]. The compile-time-checked macros
//! want a live `DATABASE_URL` at build time, which would mean CrabER could not
//! be built without a database in reach; sqlx's `macros` feature is off so they
//! cannot be reached for by accident.
//!
//! # What a live read and a parsed dump disagree about
//!
//! `craber export --file dump.sql` and `craber export --dsn` against the
//! database that dump built describe the same schema — the same tables,
//! columns, nullability, keys, cardinalities and index uniqueness. They do not
//! produce the same file, and the differences are worth knowing before diffing
//! the two outputs:
//!
//! | | From a file | From a connection |
//! |---|---|---|
//! | the order of `tables` | the order the DDL declared them | the catalog's, which is by name |
//! | `dataType` | `BIGINT`, `TIMESTAMPTZ` — sqlparser's rendering, uppercase | `bigint`, `timestamp with time zone` — the catalog's |
//! | a typed literal `default` | `'pending'` | `'pending'::order_status` |
//! | the index behind an inline `UNIQUE` | unnamed | `users_email_key` |
//! | a SQLite foreign key's name | `fk_…`, when the DDL named it | absent; `PRAGMA` does not report it |
//!
//! **Table order is not cosmetic.** `craber-layout` breaks ties within a layer
//! by the order it was handed the tables, so the two sources lay the same graph
//! out differently: identical layers and identical edges, with the rows inside a
//! layer stacked in a different order. There is no fix, only a choice of which
//! arbitrary order to use — a running database has no record of the order its
//! DDL was written in, and `pg_class.oid` only approximates it until the first
//! dump-and-restore. By name is at least stable and reproducible.
//!
//! `dataType` is the difference a reader notices, because it is in every column
//! of every node, and the IR is on the catalog's side of it:
//! [`Column::data_type`](craber_core::Column::data_type) is documented as
//! "rendered as written in the source dialect, e.g. `character varying(255)`".
//! Reconciling them means one canonical rendering used by both crates, which
//! moves every snapshot in the repository and so is its own change.

use std::time::Duration;

use craber_core::{Dialect, Outcome, Schema};
use thiserror::Error;

mod assemble;
mod connect;

#[cfg(feature = "mysql")]
mod mysql;
#[cfg(feature = "postgres")]
mod postgres;
#[cfg(feature = "sqlite")]
mod sqlite;

/// How long to wait for a connection before giving up.
///
/// sqlx's `ConnectOptions` has no connect timeout of its own, and the default
/// behaviour against a host that drops packets is to hang for about two minutes
/// with nothing on screen. A dev tool should say something sooner than that.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, Error)]
pub enum Error {
    /// Deliberately carries a fixed string rather than the source error.
    ///
    /// sqlx wraps the `url` crate's parse failure, which echoes its input — and
    /// its input is a connection string with a password in it.
    #[error("could not read the connection string as a {dialect} one")]
    Dsn { dialect: Dialect },

    #[error("the connection string names no database")]
    NoDatabase,

    #[error("timed out connecting to the database after {0:?}")]
    ConnectTimeout(Duration),

    #[error("this binary was built without {dialect} support")]
    NotCompiled { dialect: Dialect },

    #[error("could not start a runtime: {0}")]
    Runtime(#[source] std::io::Error),

    #[error(transparent)]
    Database(#[from] sqlx::Error),
}

/// Connect to `dsn` and read its schema.
///
/// # Panics
///
/// Panics if called from inside a tokio runtime — that is tokio's rule about
/// nested runtimes, not one of ours. CrabER never does: nothing in the binary is
/// async.
pub fn introspect(dsn: &str, dialect: Dialect) -> Result<Outcome<Schema>, Error> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(Error::Runtime)?;
    runtime.block_on(introspect_async(dsn, dialect))
}

/// [`introspect`] for a caller that already has a runtime.
pub async fn introspect_async(dsn: &str, dialect: Dialect) -> Result<Outcome<Schema>, Error> {
    match dialect {
        #[cfg(feature = "postgres")]
        Dialect::Postgres => postgres::introspect(dsn).await,
        #[cfg(feature = "mysql")]
        Dialect::MySql => mysql::introspect(dsn).await,
        #[cfg(feature = "sqlite")]
        Dialect::Sqlite => sqlite::introspect(dsn).await,
        #[cfg(not(all(feature = "postgres", feature = "mysql", feature = "sqlite")))]
        other => Err(Error::NotCompiled { dialect: other }),
    }
}
