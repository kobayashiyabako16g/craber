//! Workflows: DML read as a sequence of steps over the schema, the root of
//! `flows.json`.
//!
//! A workflow is a script of `SELECT`/`INSERT`/`UPDATE`/`DELETE` statements,
//! and each step records which tables and columns its statement reads, writes
//! or deletes. Nothing is executed — this is what the SQL says it touches, so
//! the viewer can play a workflow back over the diagram one step at a time.
//!
//! This is its own file rather than a field of [`Schema`](crate::Schema)
//! because `schema.json` is a public format that older sites read, and a
//! diagram without workflows is the common case. `flows.json` carries its own
//! [`FLOWS_VERSION`] for the same reason `schema.json` does.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::ir::TableRef;

/// Bumped whenever `flows.json` changes shape incompatibly.
pub const FLOWS_VERSION: u32 = 1;

/// Every workflow given to one run. The root of `flows.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Flows {
    pub flows_version: u32,
    #[serde(default)]
    pub workflows: Vec<Workflow>,
}

impl Flows {
    pub fn new(workflows: Vec<Workflow>) -> Self {
        Self {
            flows_version: FLOWS_VERSION,
            workflows,
        }
    }
}

impl Default for Flows {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}

/// One script of DML, in the order it runs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Workflow {
    /// Display name — normally the source filename.
    pub name: String,
    #[serde(default)]
    pub steps: Vec<Step>,
}

impl Workflow {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            steps: Vec::new(),
        }
    }
}

/// One statement and what it touches.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Step {
    pub kind: StepKind,
    /// The statement, re-rendered from the parse rather than copied from the
    /// source, so it is one line however it was written.
    pub sql: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub touches: Vec<Touch>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub joins: Vec<Join>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum StepKind {
    Select,
    Insert,
    Update,
    Delete,
}

/// A table a step reads, writes or deletes from, and the columns involved.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Touch {
    pub table: TableRef,
    pub access: Access,
    /// Empty when the statement names no columns (`SELECT *`, `DELETE`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub columns: Vec<String>,
}

/// What a step does to a table. A table both read and written in one statement
/// (`UPDATE … WHERE`) is touched once, as the stronger of the two.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum Access {
    Read,
    Write,
    Delete,
}

/// An equality join, `left = right`, as written in an `ON` clause.
///
/// The viewer lights the relationship this matches, when there is one.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Join {
    pub left: ColumnRef,
    pub right: ColumnRef,
}

/// A column of a table.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ColumnRef {
    pub table: TableRef,
    pub column: String,
}
