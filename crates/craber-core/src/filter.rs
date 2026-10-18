//! Table selection for `--include` / `--exclude` / `--depth`.

use std::collections::VecDeque;

use globset::{Glob, GlobSet, GlobSetBuilder};
use thiserror::Error;

use crate::index::SchemaIndex;
use crate::ir::Schema;

#[derive(Debug, Error)]
pub enum FilterError {
    #[error("invalid glob pattern `{pattern}`: {source}")]
    Glob {
        pattern: String,
        #[source]
        source: globset::Error,
    },
}

/// Narrows a schema down to the tables the user asked about.
///
/// Patterns are matched against both the bare name (`users`) and the qualified
/// name (`public.users`), so `--include 'user_*'` works regardless of whether
/// the input carried schema qualifiers.
#[derive(Debug, Default)]
pub struct Filter {
    include: Option<GlobSet>,
    exclude: Option<GlobSet>,
    depth: Option<usize>,
}

impl Filter {
    pub fn new(
        include: &[String],
        exclude: &[String],
        depth: Option<usize>,
    ) -> Result<Self, FilterError> {
        Ok(Self {
            include: build(include)?,
            exclude: build(exclude)?,
            depth,
        })
    }

    /// Nothing to select on, so [`apply`](Self::apply) has nothing to do.
    fn is_noop(&self) -> bool {
        self.include.is_none() && self.exclude.is_none() && self.depth.is_none()
    }

    /// Drop unselected tables, then drop foreign keys that now dangle.
    pub fn apply(&self, schema: &mut Schema) {
        if self.is_noop() {
            return;
        }
        let keep = self.select(schema);
        let mut i = 0;
        schema.tables.retain(|_| {
            let keep = keep[i];
            i += 1;
            keep
        });

        let index = SchemaIndex::build(schema);
        for table in &mut schema.tables {
            table
                .fks
                .retain(|fk| index.resolve(&fk.ref_table).is_some());
        }
    }

    /// A `keep` flag per table, parallel to `schema.tables`.
    fn select(&self, schema: &Schema) -> Vec<bool> {
        let matches = |set: &GlobSet, i: usize| {
            let table = &schema.tables[i];
            set.is_match(&table.name) || set.is_match(table.qualified_name())
        };

        // Seeds: everything --include allows, minus anything --exclude rejects.
        let mut keep: Vec<bool> = (0..schema.tables.len())
            .map(|i| {
                let included = self.include.as_ref().is_none_or(|set| matches(set, i));
                let excluded = self.exclude.as_ref().is_some_and(|set| matches(set, i));
                included && !excluded
            })
            .collect();

        // --depth grows the seed set outwards over the FK graph. --exclude still
        // wins: an excluded table is a wall, not just an unselected seed.
        if let Some(depth) = self.depth {
            let index = SchemaIndex::build(schema);
            let adjacency = index.adjacency(schema);
            let mut queue: VecDeque<(usize, usize)> = keep
                .iter()
                .enumerate()
                .filter_map(|(i, k)| k.then_some((i, 0)))
                .collect();
            while let Some((i, hops)) = queue.pop_front() {
                if hops == depth {
                    continue;
                }
                for neighbor in &adjacency[i] {
                    let n = neighbor.0;
                    if keep[n] || self.exclude.as_ref().is_some_and(|set| matches(set, n)) {
                        continue;
                    }
                    keep[n] = true;
                    queue.push_back((n, hops + 1));
                }
            }
        }

        keep
    }
}

fn build(patterns: &[String]) -> Result<Option<GlobSet>, FilterError> {
    if patterns.is_empty() {
        return Ok(None);
    }
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        let glob = Glob::new(pattern).map_err(|source| FilterError::Glob {
            pattern: pattern.clone(),
            source,
        })?;
        builder.add(glob);
    }
    // Both globs above are already validated, so this cannot fail in practice.
    builder
        .build()
        .map(Some)
        .map_err(|source| FilterError::Glob {
            pattern: patterns.join(","),
            source,
        })
}
