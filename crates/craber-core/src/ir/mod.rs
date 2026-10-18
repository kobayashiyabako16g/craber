//! The schema intermediate representation, and the `schema.json` wire format.
//!
//! Split by what the parts are facts *about*: [`schema`] is what the database
//! said — tables, columns, keys — and [`layout`] is what CrabER decided about
//! drawing it. Both halves are the wire format, and both are re-exported from
//! here and from the crate root, because `schema.json` is one document and a
//! reader of it should not have to know which half a type came from.

pub mod layout;
pub mod schema;

pub use crate::ir::layout::{Detail, Position, Positions};
pub use crate::ir::schema::{
    Action, Cardinality, Column, EnumType, ForeignKey, SCHEMA_VERSION, Schema, Table, TableIndex,
    TableRef,
};
