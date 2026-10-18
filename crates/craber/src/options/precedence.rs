//! What beats what, when a flag and `craber.toml` both have an opinion.
//!
//! One rule — the flag wins outright, the config file supplies what the flag
//! omits — applied to the two kinds of value where "outright" needs saying.
//! Scalars need no help: `flag.or(config.x)` already means this, and the
//! `resolve` functions spell it inline.

use craber_core::Dialect;

use crate::cli;
use crate::config::Config;
use crate::options::Source;

/// Which source wins, before anything is read.
///
/// A source named on the command line replaces the config file's outright
/// rather than merging with it: falling back per field would mean that
/// `craber export --dsn ...`, in a repository whose `craber.toml` names a
/// `file`, quietly exported the file instead of connecting.
pub(super) fn pick_source(source: &cli::Source, config: &Config) -> Option<Source> {
    match (source.file.as_deref(), source.dsn.as_deref()) {
        (Some(file), _) => Some(Source::File(file.to_path_buf())),
        (None, Some(dsn)) => Some(Source::Dsn(dsn.to_owned())),
        (None, None) => config
            .file
            .as_ref()
            .map(|path| Source::File(path.resolve(config)))
            .or_else(|| config.dsn.as_deref().map(|dsn| Source::Dsn(dsn.to_owned()))),
    }
}

/// CLI values win outright when present; the config file is not merged into
/// them, so `--exclude` on the command line replaces the file's list rather
/// than adding to it.
pub(super) fn pick<'a>(cli: &'a [String], config: &'a [String]) -> &'a [String] {
    if cli.is_empty() { config } else { cli }
}

/// The dialect asked for, by either route.
pub(super) fn asked(scope: &cli::Scope, config: &Config) -> Option<Dialect> {
    scope.dialect.or(config.dialect)
}
