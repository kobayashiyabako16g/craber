//! One module per subcommand.
//!
//! Each `run` takes an already-resolved options struct from [`crate::options`],
//! so nothing in here decides what beats what — by the time a command runs, the
//! flags and `craber.toml` have already been reconciled and the invocation has
//! been checked. What is left is the work the command names.

pub mod build;
pub mod export;
pub mod serve;

use craber_core::Diagnostic;

/// Diagnostics go to stderr so that `craber export ... > schema.json` stays
/// clean while still telling the user what was skipped.
pub fn report(diagnostics: &[Diagnostic]) {
    for diagnostic in diagnostics {
        eprintln!("craber: {diagnostic}");
    }
}
