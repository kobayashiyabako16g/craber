//! Stage 4: turn an ordering into coordinates.
//!
//! The only stage that knows how big a node is. Everything before it works in
//! node ids; from here on the numbers are diagram units.

use craber_core::{Detail, Schema};

use super::{Direction, LEVELS, Layered, Placement, SugiyamaLayout, median, sweep};
use crate::{NODE_WIDTH, node_size};

/// Space reserved across a layer for a routing dummy — the placeholder a long
/// edge occupies in each layer it crosses. Narrow enough that edges thread
/// between tables rather than reserving a whole table's worth of room.
const DUMMY_SPAN: f32 = 24.0;

impl SugiyamaLayout {
    /// Stage 4: the layer decides x, the neighbours decide y.
    ///
    /// The two axes are independent here, which is what lets the detail levels
    /// share a layout. A layer's x comes from node widths, and those never
    /// change; only the spread down a layer depends on how tall the nodes are,
    /// so each level gets its own pass over that axis and none of them moves a
    /// table sideways.
    pub(super) fn coordinates(&self, schema: &Schema, graph: &Layered) -> Placement {
        // x: one column per layer, each as wide as a node. A layer holding
        // nothing but routing dummies is a corridor for edges, not a column of
        // tables, and takes only a dummy's width.
        let mut lefts = Vec::with_capacity(graph.layers.len());
        let mut left = 0.0;
        for layer in &graph.layers {
            lefts.push(left);
            let span = if layer.iter().any(|&n| graph.table[n].is_some()) {
                NODE_WIDTH
            } else {
                DUMMY_SPAN
            };
            left += span + self.layer_gap;
        }
        let width = left - self.layer_gap;

        // y: down each layer, per detail level, since that is the axis node
        // heights are measured on.
        let spans: [Vec<f32>; LEVELS] = std::array::from_fn(|level| {
            graph
                .table
                .iter()
                .map(|node| {
                    node.map_or(DUMMY_SPAN, |t| {
                        node_size(&schema.tables[t], Detail::ALL[level]).1
                    })
                })
                .collect()
        });
        let centres: [Vec<f32>; LEVELS] =
            std::array::from_fn(|level| self.down_each_layer(graph, &spans[level]));

        let mut tables = Vec::new();
        let mut tops = [f32::MAX; LEVELS];
        let mut heights = [f32::MIN; LEVELS];
        for (i, layer) in graph.layers.iter().enumerate() {
            for &node in layer {
                let Some(table) = graph.table[node] else {
                    continue;
                };
                let ys: [f32; LEVELS] =
                    std::array::from_fn(|level| centres[level][node] - spans[level][node] / 2.0);
                tables.push((table, lefts[i], ys));
                for (level, (top, height)) in tops.iter_mut().zip(&mut heights).enumerate() {
                    *top = top.min(ys[level]);
                    *height = height.max(ys[level] + spans[level][node]);
                }
            }
        }
        // Top-left of the diagram to the origin, at every level.
        for entry in &mut tables {
            for (y, top) in entry.2.iter_mut().zip(tops) {
                *y -= top;
            }
        }
        for (height, top) in heights.iter_mut().zip(tops) {
            *height -= top;
        }
        // Draw order follows the schema, so a caller reading positions back out
        // sees them in the order the tables were declared.
        tables.sort_by_key(|&(table, ..)| table);

        Placement {
            tables,
            width,
            heights,
        }
    }

    /// Where each node sits down its layer, for one set of node spans.
    ///
    /// Spacing first, then the alignment sweeps that pull a node towards the
    /// middle of its neighbours in the layer before or after it — which is what
    /// turns a legal layout into a readable one.
    fn down_each_layer(&self, graph: &Layered, spans: &[f32]) -> Vec<f32> {
        // Centres, not edges: a node wants to sit level with the middle of its
        // neighbours, and a median over centres is what says where that is.
        let mut centres = vec![0.0_f32; graph.table.len()];
        for layer in &graph.layers {
            let mut at = 0.0;
            for (i, &node) in layer.iter().enumerate() {
                if i > 0 {
                    at += self.gap(graph, spans, layer[i - 1], node) - spans[node] / 2.0;
                }
                centres[node] = at + spans[node] / 2.0;
                at = centres[node];
            }
        }

        for iteration in 0..self.iterations {
            sweep(iteration, graph.layers.len(), |i, direction| {
                self.align_layer(graph, spans, &mut centres, i, direction);
            });
        }
        centres
    }

    /// Minimum centre-to-centre distance between two neighbours in a layer.
    fn gap(&self, graph: &Layered, spans: &[f32], a: usize, b: usize) -> f32 {
        let clearance = if graph.table[a].is_some() && graph.table[b].is_some() {
            self.node_gap
        } else {
            self.dummy_gap
        };
        f32::midpoint(spans[a], spans[b]) + clearance
    }

    /// Pull one layer towards the layer next to it, then re-establish spacing.
    fn align_layer(
        &self,
        graph: &Layered,
        spans: &[f32],
        centres: &mut [f32],
        index: usize,
        direction: Direction,
    ) {
        let layer = &graph.layers[index];
        if layer.is_empty() {
            return;
        }

        let desired: Vec<f32> = layer
            .iter()
            .map(|&node| {
                let neighbours = direction.neighbours(graph, node);
                if neighbours.is_empty() {
                    // Nothing to pull towards: leave it where it is rather than
                    // dragging it to the origin.
                    centres[node]
                } else {
                    let mut neighbour_centres: Vec<f32> =
                        neighbours.iter().map(|&n| centres[n]).collect();
                    neighbour_centres.sort_by(f32::total_cmp);
                    median(&neighbour_centres)
                }
            })
            .collect();

        // Minimum centre-to-centre distance for each adjacent pair.
        let gaps: Vec<f32> = layer
            .windows(2)
            .map(|pair| self.gap(graph, spans, pair[0], pair[1]))
            .collect();

        // Packing from the left and from the right each yields a valid layout;
        // each is also biased towards the end it started from. Since the
        // spacing constraints are linear, the average of two valid layouts is
        // itself valid — so take it, and lose the bias.
        let mut from_left = desired.clone();
        for i in 1..from_left.len() {
            from_left[i] = from_left[i].max(from_left[i - 1] + gaps[i - 1]);
        }
        let mut from_right = desired;
        for i in (0..from_right.len().saturating_sub(1)).rev() {
            from_right[i] = from_right[i].min(from_right[i + 1] - gaps[i]);
        }
        for (i, &node) in layer.iter().enumerate() {
            centres[node] = f32::midpoint(from_left[i], from_right[i]);
        }
    }
}
