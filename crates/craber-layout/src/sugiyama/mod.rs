//! A Sugiyama-style layered layout.
//!
//! Foreign keys give an ER diagram a natural reading direction — a referenced
//! table before the tables that reference it — so the graph is drawn in layers
//! and the work is in choosing them well. The four classic stages are here:
//!
//! 1. break cycles, so the graph can be layered at all;
//! 2. assign each table to a layer;
//! 3. order the tables within each layer to reduce edge crossings;
//! 4. assign coordinates, keeping each table near its neighbours.
//!
//! Layers run left to right: a table sits to the right of everything it
//! references. That is the direction the notation already reads in — a
//! relationship leaves the referenced table's right edge and arrives at the
//! referencing table's left — so every line travels the same way and none has
//! to doubles back around a node to reach its column.
//!
//! Disconnected components are laid out on their own and then packed, because
//! a real schema is usually one large web plus a scattering of lookup and
//! bookkeeping tables that relate to nothing.
//!
//! # Where each stage lives
//!
//! One module per stage, in the order [`SugiyamaLayout::component`] runs them.
//! The split is along what each stage is allowed to know, which is why it holds:
//!
//! | Module | Stage | Knows about |
//! |---|---|---|
//! | [`graph`] | 1-2 | the schema, but only to turn it into arcs; the rest is `usize` graph theory |
//! | [`ordering`] | 3 | [`Layered`] alone — no schema, no geometry |
//! | [`coords`] | 4 | node sizes and the gap settings |
//! | [`pack`] | — | [`Placement`] alone |
//!
//! This file holds what more than one of them needs: the tuning struct, the two
//! graph types, and the sweep both stage 3 and stage 4 are driven by.

mod coords;
mod graph;
mod ordering;
mod pack;

use std::collections::BTreeSet;

use craber_core::{Detail, Position, Positions, Schema};

use crate::Layout;

/// Directed arcs, deduplicated, pointing from a referenced table to the table
/// that references it. Self-references carry no layering information.
type Arcs = BTreeSet<(usize, usize)>;

#[derive(Debug, Clone)]
pub struct SugiyamaLayout {
    /// Minimum space between two tables in the same layer, across the layer.
    pub node_gap: f32,
    /// Horizontal space either side of a routing dummy. Much smaller than
    /// [`Self::node_gap`]: a dummy is a line passing through, not a box to be
    /// read, and on a schema with many long edges the dummies are what decide
    /// how wide the diagram gets.
    pub dummy_gap: f32,
    /// Space between one layer and the next, along the reading direction.
    pub layer_gap: f32,
    /// Space between disconnected components.
    pub component_gap: f32,
    /// Crossing-reduction sweeps. Past a handful the ordering stops moving.
    pub iterations: usize,
}

impl Default for SugiyamaLayout {
    fn default() -> Self {
        Self {
            node_gap: 60.0,
            dummy_gap: 14.0,
            layer_gap: 90.0,
            component_gap: 120.0,
            iterations: 8,
        }
    }
}

impl Layout for SugiyamaLayout {
    fn apply(&self, schema: &mut Schema) {
        if schema.tables.is_empty() {
            return;
        }

        let arcs = graph::arcs(schema);
        let boxes: Vec<Placement> = graph::components(schema.tables.len(), &arcs)
            .iter()
            .map(|members| self.component(schema, members, &arcs))
            .collect();

        for (placement, (left, tops)) in pack::pack(&boxes, self.component_gap) {
            for &(table, x, ys) in &placement.tables {
                let mut positions = Positions::uniform(Position::new(0.0, 0.0));
                for (level, detail) in Detail::ALL.iter().enumerate() {
                    positions.set(*detail, Position::new(left + x, tops[level] + ys[level]));
                }
                schema.tables[table].positions = Some(positions);
            }
        }
    }
}

impl SugiyamaLayout {
    fn component(&self, schema: &Schema, members: &[usize], arcs: &Arcs) -> Placement {
        let mut graph = graph::layered(schema, members, arcs);
        self.order(&mut graph);
        self.coordinates(schema, &graph)
    }
}

/// The number of detail levels, and the length of every per-level array here.
const LEVELS: usize = Detail::ALL.len();

/// One laid-out component: table positions relative to its own top-left corner.
///
/// A node is shorter at `Table Name` than at `All Fields`, so each table has a
/// `y` per detail level — but only one `x`. Keeping the horizontal result
/// identical across levels is deliberate: switching the level in the frontend
/// then compresses the diagram vertically instead of rearranging it, and the
/// reader does not lose the table they were looking at.
struct Placement {
    tables: Vec<(usize, f32, [f32; LEVELS])>,
    width: f32,
    heights: [f32; LEVELS],
}

/// The layered graph for a single component, in local node ids. Real tables
/// come first, then the dummies inserted for edges that span several layers.
struct Layered {
    /// `Some(global table index)` for a real node, `None` for a dummy.
    table: Vec<Option<usize>>,
    layers: Vec<Vec<usize>>,
    succ: Vec<Vec<usize>>,
    pred: Vec<Vec<usize>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Direction {
    /// Look at the layer before this one — the tables this one references.
    Back,
    /// Look at the layer after it.
    Forward,
}

impl Direction {
    fn neighbours(self, graph: &Layered, node: usize) -> &[usize] {
        match self {
            Self::Back => &graph.pred[node],
            Self::Forward => &graph.succ[node],
        }
    }
}

/// One alternating pass over the layers, the shape stages 3 and 4 are both
/// driven by.
///
/// Both stages work by repeatedly pulling one layer towards the layer next to
/// it — stage 3 reorders, stage 4 moves — and both alternate the sweep
/// direction on each iteration, because a sweep that always ran one way would
/// settle the result into whichever end it started from. That is the whole of
/// what they share, so it is all that lives here: what to do with a layer is
/// the caller's.
fn sweep(iteration: usize, layers: usize, mut visit: impl FnMut(usize, Direction)) {
    if iteration.is_multiple_of(2) {
        for i in 1..layers {
            visit(i, Direction::Back);
        }
    } else {
        for i in (0..layers.saturating_sub(1)).rev() {
            visit(i, Direction::Forward);
        }
    }
}

fn median(sorted: &[f32]) -> f32 {
    match sorted.len() {
        0 => 0.0,
        n if n % 2 == 1 => sorted[n / 2],
        n => f32::midpoint(sorted[n / 2 - 1], sorted[n / 2]),
    }
}
