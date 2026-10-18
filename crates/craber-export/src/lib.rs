//! A laid-out [`Schema`] as a file: the formats `craber export` writes.
//!
//! This crate is the dispatch that `--format` selects, and the one place a new
//! output format has to be taught about. It is deliberately the only crate that
//! knows the whole set: `craber-render` knows SVG and nothing else, the binary
//! knows where to put the bytes and nothing about how they were produced.
//!
//! Everything here takes a schema that has already been laid out. Positions are
//! read, never computed — layout runs in `craber-layout` — so a schema that
//! never went through it encodes as a diagram with every table at the origin.

mod format;

pub use crate::format::{Format, UnknownFormat};

use craber_core::Schema;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("serializing schema.json")]
    Json(#[from] serde_json::Error),

    /// A format `--format` accepts but this build cannot produce.
    ///
    /// Declared rather than omitted from [`Format`] so that the CLI surface is
    /// settled before anyone scripts against it, and so asking for one fails
    /// loudly instead of silently falling back to another format
    /// (`crates/craber/src/cli.rs` says the same about its subcommands).
    #[error("`--format {format}` is not implemented yet ({how})")]
    Unsupported {
        format: Format,
        /// What is missing and when it is due, e.g. "resvg, M3".
        how: &'static str,
    },
}

/// Encode a laid-out schema in `format`.
///
/// Returns text because every format CrabER writes today is text. PNG is the
/// one that will not be, and it is unimplemented — when it lands, this gains a
/// bytes-returning sibling rather than having PNG smuggled through a `String`.
pub fn encode(schema: &Schema, format: Format) -> Result<String, Error> {
    match format {
        Format::Json => Ok(serde_json::to_string_pretty(schema)?),
        Format::Svg => Ok(craber_render::to_svg(schema)),
        Format::Png => Err(Error::Unsupported {
            format,
            how: "resvg, M3; export SVG and convert it",
        }),
        Format::Mermaid | Format::Dbml => Err(Error::Unsupported { format, how: "M3" }),
    }
}

#[cfg(test)]
mod tests {
    use super::{Error, Format, encode};
    use craber_core::Schema;

    #[test]
    fn json_is_the_schema_itself() {
        let encoded = encode(&Schema::new("shop"), Format::Json).expect("json");
        assert!(encoded.contains(r#""name": "shop""#), "{encoded}");
    }

    #[test]
    fn svg_is_a_document() {
        let encoded = encode(&Schema::new("shop"), Format::Svg).expect("svg");
        assert!(encoded.starts_with("<svg"), "{encoded}");
    }

    /// The message has to name the format the way the user spelled it. It used
    /// to be `{format:?}`, which printed `Mermaid` for a CLI that only accepts
    /// `mermaid`.
    #[test]
    fn an_unsupported_format_is_named_as_the_cli_spells_it() {
        for format in [Format::Png, Format::Mermaid, Format::Dbml] {
            let error = encode(&Schema::new("shop"), format).unwrap_err();
            assert!(matches!(error, Error::Unsupported { .. }));
            let message = error.to_string();
            assert!(message.contains("not implemented"), "{message}");
            assert!(message.contains(format.as_str()), "{message}");
        }
    }
}
