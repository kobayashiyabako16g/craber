//! What beats what, when a flag and `craber.toml` both have an opinion.
//!
//! One rule — the flag wins outright, the config file supplies what the flag
//! omits — applied to the two kinds of value where "outright" needs saying.
//! Scalars need no help: `flag.or(config.x)` already means this, and the
//! `resolve` functions spell it inline.

use crate::cli::SourceArgs;
use crate::config::Config;
use crate::options::Source;

/// Which source wins, before anything is read.
///
/// A source named on the command line replaces the config file's outright
/// rather than merging with it: falling back per field would mean that
/// `craber export --dsn ...`, in a repository whose `craber.toml` names a
/// `file`, quietly exported the file instead of connecting.
pub(super) fn pick_source(source: &SourceArgs, config: &Config) -> Option<Source> {
    match (&source.file, &source.dsn) {
        (Some(file), _) => Some(Source::File(file.clone())),
        (None, Some(dsn)) => Some(Source::Dsn(dsn.clone())),
        (None, None) => config
            .file
            .clone()
            .map(Source::File)
            .or_else(|| config.dsn.clone().map(Source::Dsn)),
    }
}

/// A list from the command line replaces the config file's outright rather
/// than adding to it, so `--exclude` or `--flow` means exactly what it says.
pub(super) fn pick<T: Clone>(cli: &[T], config: &[T]) -> Vec<T> {
    if cli.is_empty() { config } else { cli }.to_vec()
}
