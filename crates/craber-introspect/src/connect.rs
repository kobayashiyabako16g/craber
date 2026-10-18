//! Getting a connection, the same way for all three dialects.
//!
//! Each backend wrote this out itself: parse the DSN, scrub the DSN out of the
//! error, turn off statement logging, and impose a connect timeout sqlx does
//! not have. Three copies of four steps, where only the tuning in the middle —
//! PostgreSQL's empty `search_path`, SQLite's read-only — differs.
//!
//! Generic over [`ConnectOptions`] rather than written per dialect: the trait
//! already requires `FromStr` and already names the connection type, which is
//! exactly the shape of the repetition.

use craber_core::Dialect;
use sqlx::ConnectOptions;

use crate::{CONNECT_TIMEOUT, Error};

/// Parse `dsn` into connection options, with logging off.
///
/// # Errors
///
/// [`Error::Dsn`] carries only the dialect. sqlx wraps the `url` crate's parse
/// failure, which echoes its input — and its input is a connection string with
/// a password in it.
pub(crate) fn options<O: ConnectOptions>(dsn: &str, dialect: Dialect) -> Result<O, Error> {
    dsn.parse::<O>()
        .map(ConnectOptions::disable_statement_logging)
        .map_err(|_| Error::Dsn { dialect })
}

/// Connect, giving up after [`CONNECT_TIMEOUT`].
///
/// sqlx's `ConnectOptions` has no connect timeout of its own, and the default
/// behaviour against a host that drops packets is to hang for about two minutes
/// with nothing on screen. A dev tool should say something sooner than that.
pub(crate) async fn connect<O>(options: &O) -> Result<O::Connection, Error>
where
    O: ConnectOptions,
    O::Connection: Sized,
{
    tokio::time::timeout(CONNECT_TIMEOUT, options.connect())
        .await
        .map_err(|_| Error::ConnectTimeout(CONNECT_TIMEOUT))?
        .map_err(Error::from)
}
