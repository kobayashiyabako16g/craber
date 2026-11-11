//! The roadmap milestones a not-yet-built feature is refused with.

use std::fmt;

/// A milestone on the roadmap, named the way refusals quote it.
///
/// An enum rather than a string at each refusal, so that moving a feature's
/// date — or finishing a milestone — is one edit the compiler follows to every
/// place that quotes it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Milestone {
    /// Single-file HTML, Mermaid and DBML.
    M3,
    /// `craber diff` and the WASM demo.
    M7,
}

impl fmt::Display for Milestone {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::M3 => "M3",
            Self::M7 => "M7",
        })
    }
}
