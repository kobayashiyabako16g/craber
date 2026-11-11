//! The CrabER schema IR.
//!
//! Every CrabER output — the static site, `serve`, the single-file HTML,
//! Mermaid, DBML — is rendered from the [`Schema`] defined here, serialized as
//! `schema.json`. That makes these types a public wire format rather than an
//! internal convenience: see [`SCHEMA_VERSION`].

// The generator for `web/src/generated`, run by its own unit tests — so it is
// test-only code, and compiled only then. It used to be `pub mod`, which put a
// generator in a published crate's public API so that an integration test
// could reach it. See the module docs for why generating is a test at all.
#[cfg(test)]
mod codegen;
// Private, and re-exported below: one path to each item, `craber_core::Table`,
// rather than three.
mod diagnostic;
mod dialect;
mod filter;
mod flow;
mod geometry;
mod index;
mod ir;
mod normalize;
mod theme;

pub use diagnostic::{Diagnostic, Outcome, Severity};
pub use dialect::{Dialect, UnknownDialect};
pub use filter::{Filter, FilterError};
pub use flow::{
    Access, ColumnRef, ColumnValue, FLOWS_FILE, FLOWS_VERSION, Flows, Join, Step, StepKind, Touch,
    ValueKind, Workflow,
};
pub use geometry::{metrics, node_size, visible_columns};
pub use index::{Edge, SchemaIndex, TableId};
pub use ir::{
    Action, Cardinality, Column, Detail, EnumType, ForeignKey, Position, Positions, SCHEMA_FILE,
    SCHEMA_VERSION, Schema, Table, TableIndex, TableRef,
};
pub use normalize::normalize;
pub use theme::{Chrome, Theme};
