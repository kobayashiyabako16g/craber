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
/// sometimes bare `users` — so:
///
/// - a qualified reference matches that exact table, and failing that the one
///   *unqualified* table of that name: an unqualified table lives in whatever
///   the default schema is, which may well be the one named. A table in a
///   different named schema is a different table, and never matches;
/// - an unqualified reference matches by bare name, but only when that name is
///   unambiguous across schemas.
///
/// `web/src/lib/schema.ts`'s `resolveRef` is the same rule, and both are held
/// to `tests/cases/resolve.json`. Every lookup in CrabER goes through here —
/// the parser's `ALTER TABLE` included — so a reference and an amendment never
/// disagree about which table a name means.
pub struct SchemaIndex {
    exact: HashMap<TableRef, Vec<usize>>,
    bare: HashMap<String, Vec<usize>>,
}

impl SchemaIndex {
    pub fn build(schema: &Schema) -> Self {
        let mut exact: HashMap<TableRef, Vec<usize>> = HashMap::new();
        let mut bare: HashMap<String, Vec<usize>> = HashMap::new();
        for (i, table) in schema.tables.iter().enumerate() {
            exact.entry(table.to_ref()).or_default().push(i);
            bare.entry(table.name.clone()).or_default().push(i);
        }
        Self { exact, bare }
    }

    /// Resolve a reference, or `None` if the target is missing or ambiguous.
    pub fn resolve(&self, target: &TableRef) -> Option<TableId> {
        let target = target.normalized();
        let candidates = if target.schema.is_some() {
            match self.exact.get(&target) {
                Some(candidates) => candidates,
                None => self.exact.get(&TableRef::new(target.name))?,
            }
        } else {
            self.bare.get(&target.name)?
        };
        match candidates.as_slice() {
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
