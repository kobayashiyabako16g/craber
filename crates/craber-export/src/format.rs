//! The formats `craber export --format` names.
//!
//! [`Format`] follows [`Dialect`](craber_core::Dialect) rather than deriving
//! clap's `ValueEnum` unconditionally: it is a domain type that happens to be
//! spellable on a command line, not a command-line type. `FromStr` and
//! `Display` are always available, and the `clap` feature adds `ValueEnum` for
//! the binary.
//!
//! That split is what lets `craber.toml` be deserialized without the config
//! module depending on the CLI module — the direction the dependency used to
//! run, back when this enum lived in `craber/src/cli.rs`.

use std::fmt;
use std::str::FromStr;

use serde::Deserialize;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
#[cfg_attr(feature = "clap", value(rename_all = "lower"))]
pub enum Format {
    // `Json` is the IR itself, which every other format is derived from, and so
    // the default: it is the format that cannot be wrong about the schema.
    #[default]
    Json,
    Svg,
    Png,
    Mermaid,
    Dbml,
}

impl Format {
    /// Every format, in the order `--help` lists them.
    pub const ALL: [Self; 5] = [Self::Json, Self::Svg, Self::Png, Self::Mermaid, Self::Dbml];

    /// The spelling the CLI and `craber.toml` both accept.
    ///
    /// One function rather than a `match` in each of `Display` and `FromStr`,
    /// so a new format cannot be given two different names by accident.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Json => "json",
            Self::Svg => "svg",
            Self::Png => "png",
            Self::Mermaid => "mermaid",
            Self::Dbml => "dbml",
        }
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
#[derive(Debug, Error)]
#[error("unknown format `{0}`, expected one of: json, svg, png, mermaid, dbml")]
pub struct UnknownFormat(String);

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
        assert_eq!("SVG".parse::<Format>().unwrap(), Format::Svg);
    }

    #[test]
    fn an_unknown_format_lists_the_known_ones() {
        let error = "yaml".parse::<Format>().unwrap_err().to_string();
        assert!(error.contains("`yaml`"), "{error}");
        assert!(error.contains("mermaid"), "{error}");
    }
}
