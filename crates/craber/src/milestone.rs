//! Refusing what the CLI declares but does not do yet.
//!
//! `cli.rs` declares the whole command surface so that what users script
//! against is settled before anyone depends on it, which means some of it has
//! to be refused at run time. Those refusals used to be written wherever the
//! flag happened to be read — two in `main`, two more in the export command —
//! in three different phrasings. One function means a user sees the same
//! sentence whichever unbuilt thing they asked for, and that the roadmap is
//! quoted from one place when it moves.
//!
//! Formats are not here: `--format png` is refused by
//! [`craber_export::Error::Unsupported`], because which formats exist is that
//! crate's to know.

/// Reject `feature`, naming the milestone it is scheduled for.
///
/// `hint` is the thing to do instead, when there is one — "drop the flag to
/// write a directory" is more use than the refusal alone.
pub fn not_yet(feature: &str, milestone: &str, hint: Option<&str>) -> anyhow::Error {
    match hint {
        Some(hint) => anyhow::anyhow!("{feature} is not implemented yet ({milestone}); {hint}"),
        None => anyhow::anyhow!("{feature} is not implemented yet ({milestone})"),
    }
}
