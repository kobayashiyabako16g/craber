//! Which dialect to read a source in.
//!
//! Separate from the rest of resolving because it is the one option that can be
//! *contradicted* rather than merely absent: a file's extension and a DSN's
//! scheme are evidence, and a `--dialect` that disagrees with the evidence is an
//! invocation that cannot be satisfied.
//!
//! Nothing here may put a DSN in a message. A connection string carries a
//! password, and CrabER's errors go to stderr and from there into CI logs.

use anyhow::{Result, anyhow, bail};
use craber_core::Dialect;

use crate::cli;
use crate::config::Config;
use crate::options::{Source, precedence};

/// The dialect to read `source` in.
pub(super) fn of(source: &Source, scope: &cli::Scope, config: &Config) -> Result<Dialect> {
    let asked = precedence::asked(scope, config);
    match source {
        // A file always has an answer: the extension, or the default.
        Source::File(path) => {
            Ok(asked.unwrap_or_else(|| Dialect::from_path(path).unwrap_or_default()))
        }
        Source::Dsn(dsn) => for_dsn(asked, Dialect::from_dsn(dsn)),
    }
}

/// The four ways a connection string and a `--dialect` can meet.
///
/// Flat rather than nested so that the one case which is an error sits beside
/// the three that are not, instead of being reached through them.
fn for_dsn(asked: Option<Dialect>, scheme: Option<Dialect>) -> Result<Dialect> {
    match (asked, scheme) {
        // `--dialect mysql --dsn postgres://...` cannot be satisfied, and
        // picking one of the two silently turns into twenty minutes spent on a
        // connection error.
        (Some(asked), Some(scheme)) if asked != scheme => {
            bail!("--dialect {asked} contradicts the connection string, which names {scheme}")
        }
        (Some(asked), _) => Ok(asked),
        (None, Some(scheme)) => Ok(scheme),
        (None, None) => Err(anyhow!(
            "could not tell the dialect from the connection string; pass --dialect"
        )),
    }
}
