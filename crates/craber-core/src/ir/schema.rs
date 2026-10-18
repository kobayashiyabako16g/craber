//! The schema objects `schema.json` carries: what the database said.
//!
//! Where a table is drawn and how much of it shows are a different kind of fact
//! and live in [`layout`](super::layout).

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::ir::Positions;

/// Bumped whenever `schema.json` changes shape incompatibly.
///
/// The same file is read by the static site, the single-file HTML and the WASM
/// demo, which are versioned independently of the binary that wrote it — so the
/// reader needs a way to reject a file it does not understand.
pub const SCHEMA_VERSION: u32 = 2;

/// A whole database, normalized. The root of `schema.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Schema {
    pub schema_version: u32,
    /// Display name for the diagram — the database name, or the input filename.
    pub name: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tables: Vec<Table>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub enums: Vec<EnumType>,
}

impl Schema {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            name: name.into(),
            tables: Vec::new(),
            enums: Vec::new(),
        }
    }

    pub fn table(&self, name: &str) -> Option<&Table> {
        self.tables.iter().find(|t| t.name == name)
    }
}

impl Default for Schema {
    fn default() -> Self {
        Self::new("")
    }
}

/// A table (or view) and everything drawn on its node.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Table {
    /// The namespace (`public`, `dbo`, …). Absent for dialects without one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub columns: Vec<Column>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub indexes: Vec<TableIndex>,
    /// Primary key column names, in key order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pk: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fks: Vec<ForeignKey>,
    /// Assigned by `craber-layout`; absent until layout has run.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub positions: Option<Positions>,
}

impl Table {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            schema: None,
            name: name.into(),
            comment: None,
            columns: Vec::new(),
            indexes: Vec::new(),
            pk: Vec::new(),
            fks: Vec::new(),
            positions: None,
        }
    }

    /// `schema.name` when namespaced, otherwise just `name`.
    pub fn qualified_name(&self) -> String {
        match &self.schema {
            Some(schema) => format!("{schema}.{}", self.name),
            None => self.name.clone(),
        }
    }

    pub fn as_ref(&self) -> TableRef {
        TableRef {
            schema: self.schema.clone(),
            name: self.name.clone(),
        }
    }

    pub fn column(&self, name: &str) -> Option<&Column> {
        self.columns.iter().find(|c| c.name == name)
    }

    /// True if `columns` is exactly the primary key or exactly a unique index —
    /// i.e. at most one row can carry any given value tuple.
    pub fn is_unique_tuple(&self, columns: &[String]) -> bool {
        if columns.is_empty() {
            return false;
        }
        let same = |other: &[String]| {
            other.len() == columns.len() && other.iter().all(|c| columns.contains(c))
        };
        same(&self.pk) || self.indexes.iter().any(|i| i.unique && same(&i.columns))
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Column {
    pub name: String,
    /// Rendered as written in the source dialect, e.g. `character varying(255)`.
    pub data_type: String,
    pub nullable: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
}

impl Column {
    pub fn new(name: impl Into<String>, data_type: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            data_type: data_type.into(),
            nullable: true,
            default: None,
            comment: None,
        }
    }
}

/// A database index on a table.
///
/// Named `TableIndex` rather than `Index` so the generated TypeScript file does
/// not collide with a barrel `index.ts` on case-insensitive filesystems, and so
/// it is not confused with [`SchemaIndex`](crate::SchemaIndex), which is a
/// lookup structure rather than a schema object.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct TableIndex {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub columns: Vec<String>,
    pub unique: bool,
}

/// Points at a table that may or may not exist in this schema.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct TableRef {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,
    pub name: String,
}

impl TableRef {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            schema: None,
            name: name.into(),
        }
    }

    pub fn qualified(schema: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            schema: Some(schema.into()),
            name: name.into(),
        }
    }

    pub fn qualified_name(&self) -> String {
        match &self.schema {
            Some(schema) => format!("{schema}.{}", self.name),
            None => self.name.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ForeignKey {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub columns: Vec<String>,
    pub ref_table: TableRef,
    pub ref_columns: Vec<String>,
    #[serde(default)]
    pub on_delete: Action,
    #[serde(default)]
    pub on_update: Action,
    /// True when this relation came from a naming convention (`user_id` →
    /// `users.id`) rather than a declared constraint. Drawn as a dashed edge:
    /// production databases very often lack the constraint but still have the
    /// relationship, and the user needs to know which is which.
    #[serde(default)]
    pub inferred: bool,
    #[serde(default)]
    pub cardinality: Cardinality,
    /// True when the foreign key's own columns are nullable, so a row need not
    /// have a parent at all.
    ///
    /// This is the difference between `zero or one` and `one and only one` at
    /// the referenced end of the line, and it is a fact about the schema rather
    /// than about the drawing: `orders.coupon_id NULL` means an order may have
    /// no coupon, and the diagram should say so.
    #[serde(default)]
    pub optional: bool,
}

impl ForeignKey {
    /// A key exactly as the source declared it, with the facts that are not
    /// declared anywhere left at their defaults.
    ///
    /// [`cardinality`](Self::cardinality) and [`optional`](Self::optional) are
    /// filled in by
    /// [`annotate_relationships`](crate::normalize::annotate_relationships),
    /// which needs the whole table — the FK's own columns decide both, and
    /// neither is knowable from the constraint alone. Every producer builds
    /// keys through here and lets that pass finish them, which is what stops
    /// one of them from guessing and drawing a relationship the others do not.
    pub fn declared(columns: Vec<String>, ref_table: TableRef, ref_columns: Vec<String>) -> Self {
        Self {
            name: None,
            columns,
            ref_table,
            ref_columns,
            on_delete: Action::default(),
            on_update: Action::default(),
            inferred: false,
            cardinality: Cardinality::default(),
            optional: false,
        }
    }

    /// [`declared`](Self::declared) for the common single-column case.
    pub fn new(
        column: impl Into<String>,
        ref_table: TableRef,
        ref_column: impl Into<String>,
    ) -> Self {
        Self::declared(vec![column.into()], ref_table, vec![ref_column.into()])
    }
}

/// Referential action for `ON DELETE` / `ON UPDATE`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum Action {
    #[default]
    NoAction,
    Restrict,
    Cascade,
    SetNull,
    SetDefault,
}

/// Which crow's-foot marker to draw at each end of an edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum Cardinality {
    OneToOne,
    /// The default: many rows on the referencing side, one on the referenced.
    #[default]
    ManyToOne,
}

/// A named enum type, so `status` columns show their allowed values.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct EnumType {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,
    pub name: String,
    pub values: Vec<String>,
}
