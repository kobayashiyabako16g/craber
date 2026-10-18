//! The CrabER schema IR.
//!
//! Every CrabER output — the static site, `serve`, the single-file HTML, SVG,
//! Mermaid, DBML — is rendered from the [`Schema`] defined here, serialized as
//! `schema.json`. That makes these types a public wire format rather than an
//! internal convenience: see [`SCHEMA_VERSION`].

// The generator for `web/src/generated`, run by its own unit tests — so it is
// test-only code, and compiled only then. It used to be `pub mod`, which put a
// generator in a published crate's public API so that an integration test
// could reach it. See the module docs for why generating is a test at all.
#[cfg(test)]
mod codegen;
pub mod diagnostic;
pub mod dialect;
pub mod filter;
pub mod geometry;
pub mod index;
pub mod ir;
pub mod normalize;
pub mod theme;

pub use diagnostic::{Diagnostic, Outcome, Severity};
pub use dialect::{Dialect, UnknownDialect};
pub use filter::{Filter, FilterError};
pub use geometry::{metrics, node_size, visible_columns};
pub use index::{Edge, SchemaIndex, TableId};
pub use ir::{
    Action, Cardinality, Column, Detail, EnumType, ForeignKey, Position, Positions, SCHEMA_VERSION,
    Schema, Table, TableIndex, TableRef,
};
pub use normalize::annotate_relationships;
pub use theme::{Chrome, Theme};
