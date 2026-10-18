//! Resolving [`TableRef`]s to concrete tables, and the FK graph built from that.

use std::collections::HashMap;

use crate::ir::{Schema, TableRef};

/// Position of a table in [`Schema::tables`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TableId(pub usize);

/// One resolved foreign key, as a graph edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Edge {
    /// The referencing table.
    pub from: TableId,
    /// The referenced table.
    pub to: TableId,
    /// Index into the referencing table's `fks`.
    pub fk: usize,
}

/// Name lookup over a [`Schema`].
///
/// DDL files reference tables inconsistently — sometimes `public.users`,
/// sometimes bare `users` — so an unqualified reference resolves by bare name,
/// but only when that name is unambiguous across schemas.
pub struct SchemaIndex {
    qualified: HashMap<String, usize>,
    bare: HashMap<String, Vec<usize>>,
}

impl SchemaIndex {
    pub fn build(schema: &Schema) -> Self {
        let mut qualified = HashMap::new();
        let mut bare: HashMap<String, Vec<usize>> = HashMap::new();
        for (i, table) in schema.tables.iter().enumerate() {
            qualified.insert(table.qualified_name(), i);
            bare.entry(table.name.clone()).or_default().push(i);
        }
        Self { qualified, bare }
    }

    /// Resolve a reference, or `None` if the target is missing or ambiguous.
    pub fn resolve(&self, target: &TableRef) -> Option<TableId> {
        if target.schema.is_some() {
            return self
                .qualified
                .get(&target.qualified_name())
                .copied()
                .map(TableId);
        }
        match self.bare.get(&target.name)?.as_slice() {
            [only] => Some(TableId(*only)),
            _ => None,
        }
    }

    /// Every foreign key whose target exists in this schema. Dangling
    /// references are silently dropped: there is nothing to draw an edge to.
    pub fn edges(&self, schema: &Schema) -> Vec<Edge> {
        let mut edges = Vec::new();
        for (i, table) in schema.tables.iter().enumerate() {
            for (fk_index, fk) in table.fks.iter().enumerate() {
                if let Some(to) = self.resolve(&fk.ref_table) {
                    edges.push(Edge {
                        from: TableId(i),
                        to,
                        fk: fk_index,
                    });
                }
            }
        }
        edges
    }

    /// Undirected adjacency over the FK graph. Relationships are navigated in
    /// both directions in the UI, so `--depth` must not care about direction.
    pub fn adjacency(&self, schema: &Schema) -> Vec<Vec<TableId>> {
        let mut adjacency = vec![Vec::new(); schema.tables.len()];
        for edge in self.edges(schema) {
            if edge.from == edge.to {
                continue;
            }
            adjacency[edge.from.0].push(edge.to);
            adjacency[edge.to.0].push(edge.from);
        }
        adjacency
    }
}
