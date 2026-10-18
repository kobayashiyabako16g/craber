//! The placeholder grid layout, kept as a baseline to measure against.

use craber_core::{Detail, Position, Positions, Schema, SchemaIndex, TableId};

use crate::{Layout, node_size};

/// Places tables on a grid, walking the FK graph so that related tables land
/// next to each other rather than in schema-file order.
///
/// This is deliberately dumb — it is the baseline M1's real layout has to beat.
#[derive(Debug, Clone)]
pub struct GridLayout {
    /// Nodes per row. `None` picks a roughly square grid.
    pub columns: Option<usize>,
    pub gap_x: f32,
    pub gap_y: f32,
}

impl Default for GridLayout {
    fn default() -> Self {
        Self {
            columns: None,
            gap_x: 80.0,
            gap_y: 60.0,
        }
    }
}

impl Layout for GridLayout {
    fn apply(&self, schema: &mut Schema) {
        if schema.tables.is_empty() {
            return;
        }

        let order = connected_order(schema);
        let per_row = self.columns.unwrap_or_else(|| {
            #[allow(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                clippy::cast_precision_loss,
                reason = "table counts are far below f64 precision limits"
            )]
            let square = (schema.tables.len() as f64).sqrt().ceil() as usize;
            square.max(1)
        });

        let mut y = 0.0;
        for row in order.chunks(per_row) {
            let mut x = 0.0;
            let mut tallest: f32 = 0.0;
            for id in row {
                // The baseline ignores the detail level: it reserves room for
                // the tallest node and puts the table in the same spot at every
                // level. Only the real layout compacts.
                let (width, height) = node_size(&schema.tables[id.0], Detail::FULL);
                schema.tables[id.0].positions = Some(Positions::uniform(Position::new(x, y)));
                x += width + self.gap_x;
                tallest = tallest.max(height);
            }
            y += tallest + self.gap_y;
        }
    }
}

/// Table order: connected components first, each explored breadth-first from
/// its most-connected table, so neighbours end up as grid neighbours.
fn connected_order(schema: &Schema) -> Vec<TableId> {
    let index = SchemaIndex::build(schema);
    let adjacency = index.adjacency(schema);

    let mut roots: Vec<usize> = (0..schema.tables.len()).collect();
    roots.sort_by_key(|&i| {
        (
            std::cmp::Reverse(adjacency[i].len()),
            schema.tables[i].name.clone(),
        )
    });

    let mut visited = vec![false; schema.tables.len()];
    let mut order = Vec::with_capacity(schema.tables.len());
    for root in roots {
        if visited[root] {
            continue;
        }
        visited[root] = true;
        let mut queue = std::collections::VecDeque::from([root]);
        while let Some(current) = queue.pop_front() {
            order.push(TableId(current));
            let mut neighbours: Vec<usize> = adjacency[current]
                .iter()
                .map(|id| id.0)
                .filter(|&n| !visited[n])
                .collect();
            neighbours.sort_unstable();
            neighbours.dedup();
            for neighbour in neighbours {
                visited[neighbour] = true;
                queue.push_back(neighbour);
            }
        }
    }
    order
}
