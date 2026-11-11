//! The formats `craber export --format` names.
//!
//! [`Format`] follows [`Dialect`](craber_core::Dialect): it is a domain type
//! that happens to be spellable on a command line, not a command-line type.
//! Every spelling of it — `Display`, `FromStr`, `craber.toml` and, with the
//! `clap` feature, `--format` — comes from [`Format::as_str`], so there is one
//! list of names and one case-insensitive parser.
//!
//! That split is what lets `craber.toml` be deserialized without the config
//! module depending on the CLI module — the direction the dependency used to
//! run, back when this enum lived in `craber/src/cli.rs`.

use std::fmt;
use std::str::FromStr;

use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Format {
    // `Json` is the IR itself, which every other format is derived from, and so
    // the default: it is the format that cannot be wrong about the schema.
    #[default]
    Json,
    Mermaid,
    Dbml,
    /// The workflows given with `--flow`, as `flows.json`. Not a drawing of the
    /// schema, but the file the viewer plays back over one.
    Flows,
}

impl Format {
    /// Every format, in the order `--help` lists them.
    pub const ALL: [Self; 4] = [Self::Json, Self::Mermaid, Self::Dbml, Self::Flows];

    /// The spelling the CLI and `craber.toml` both accept.
    ///
    /// One function rather than a `match` in each of `Display` and `FromStr`,
    /// so a new format cannot be given two different names by accident.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Json => "json",
            Self::Mermaid => "mermaid",
            Self::Dbml => "dbml",
            Self::Flows => "flows",
        }
    }
}

/// Written out rather than derived: the derive spells each variant itself, and
/// it and [`Format::as_str`] were two lists of names that could disagree. The
/// argument opts into `ignore_case`, which is what `FromStr` does too.
#[cfg(feature = "clap")]
impl clap::ValueEnum for Format {
    fn value_variants<'a>() -> &'a [Self] {
        &Self::ALL
    }

    fn to_possible_value(&self) -> Option<clap::builder::PossibleValue> {
        Some(clap::builder::PossibleValue::new(self.as_str()))
    }
}

impl fmt::Display for Format {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The value `--format` was given, when it is not a format.
///
/// Carries the list rather than a bare "unknown format" so the message is
/// actionable without running `--help` — the same shape as
/// [`UnknownDialect`](craber_core::UnknownDialect).
///
/// The list is built from [`Format::ALL`] rather than written out, so a new
/// format is listed the moment it exists.
#[derive(Debug, Error)]
#[error("unknown format `{0}`, expected one of: {expected}", expected = expected())]
pub struct UnknownFormat(String);

fn expected() -> String {
    Format::ALL.map(Format::as_str).join(", ")
}

impl FromStr for Format {
    type Err = UnknownFormat;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let lower = s.to_ascii_lowercase();
        Self::ALL
            .into_iter()
            .find(|format| format.as_str() == lower)
            .ok_or_else(|| UnknownFormat(s.to_owned()))
    }
}

#[cfg(test)]
mod tests {
    use super::Format;

    /// `as_str` is what both `Display` and `FromStr` are built on, so a format
    /// that lost its round trip would be one the config file could not name.
    #[test]
    fn every_format_round_trips_through_its_name() {
        for format in Format::ALL {
            assert_eq!(format.to_string().parse::<Format>().unwrap(), format);
        }
    }

    /// `--dialect` accepts `POSTGRES`; `--format` should not be stricter.
    #[test]
    fn parsing_ignores_case() {
        assert_eq!("JSON".parse::<Format>().unwrap(), Format::Json);
    }

    /// `ALL` is written out, so this is what notices a format missing from
    /// it: the `match` stops compiling when a variant is added, and the
    /// assertion fails until `ALL` lists it too.
    #[test]
    fn all_lists_every_format_once() {
        let position = |format: Format| match format {
            Format::Json => 0,
            Format::Mermaid => 1,
            Format::Dbml => 2,
            Format::Flows => 3,
        };
        assert_eq!(Format::ALL.map(position), [0, 1, 2, 3]);
    }

    #[test]
    fn an_unknown_format_lists_the_known_ones() {
        let error = "yaml".parse::<Format>().unwrap_err().to_string();
        assert!(error.contains("`yaml`"), "{error}");
        assert!(error.contains("mermaid"), "{error}");
    }
}
