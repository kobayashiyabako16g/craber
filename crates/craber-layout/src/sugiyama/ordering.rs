//! Stage 3: order the nodes within each layer so that fewer edges cross.
//!
//! Nothing here reads a [`Schema`](craber_core::Schema) or a node size. The
//! whole stage is a permutation of [`Layered::layers`], judged by counting
//! crossings — which is why it can be reasoned about without knowing what a
//! table looks like.

use super::{Direction, Layered, SugiyamaLayout, median, sweep};

impl SugiyamaLayout {
    /// Stage 3: reduce crossings by repeated median sorting and transposition,
    /// keeping the best ordering seen rather than the last one.
    pub(super) fn order(&self, graph: &mut Layered) {
        let mut best = graph.layers.clone();
        let mut fewest = crossings(graph);

        for iteration in 0..self.iterations {
            let layers = graph.layers.len();
            sweep(iteration, layers, |i, direction| {
                median_sort(graph, i, direction);
            });
            transpose(graph);

            let count = crossings(graph);
            if count < fewest {
                fewest = count;
                best.clone_from(&graph.layers);
            }
        }
        graph.layers = best;
    }
}

/// A node's place in its layer, as a sort key.
///
/// The one cast in this module, named so the reason it is lossless is stated
/// once rather than at each of the two places a position becomes a key.
#[allow(
    clippy::cast_precision_loss,
    reason = "layer widths are far below f32 precision limits"
)]
fn place(position: usize) -> f32 {
    position as f32
}

/// Reorder one layer by the median position of each node's neighbours, the
/// heuristic that does most of the crossing reduction.
fn median_sort(graph: &mut Layered, index: usize, direction: Direction) {
    let adjacent = match direction {
        Direction::Back => index.checked_sub(1),
        Direction::Forward => Some(index + 1),
    };
    let Some(adjacent) = adjacent.filter(|&i| i < graph.layers.len()) else {
        return;
    };

    let mut rank = vec![0.0_f32; graph.table.len()];
    for (position, &node) in graph.layers[adjacent].iter().enumerate() {
        rank[node] = place(position);
    }

    let keys: Vec<f32> = graph.layers[index]
        .iter()
        .enumerate()
        .map(|(position, &node)| {
            let neighbours = direction.neighbours(graph, node);
            if neighbours.is_empty() {
                // A node with no neighbours in that direction has no opinion,
                // so it holds its place instead of collapsing to the left.
                place(position)
            } else {
                let mut ranks: Vec<f32> = neighbours.iter().map(|&n| rank[n]).collect();
                ranks.sort_by(f32::total_cmp);
                median(&ranks)
            }
        })
        .collect();

    let mut order: Vec<usize> = (0..graph.layers[index].len()).collect();
    order.sort_by(|&a, &b| keys[a].total_cmp(&keys[b]).then(a.cmp(&b)));
    graph.layers[index] = order.iter().map(|&i| graph.layers[index][i]).collect();
}

/// Swap adjacent pairs while doing so removes crossings. Cheap, and it clears
/// up the local tangles the median heuristic leaves behind.
fn transpose(graph: &mut Layered) {
    let mut improved = true;
    let mut rounds = 0;
    while improved && rounds < 4 {
        improved = false;
        rounds += 1;
        for index in 0..graph.layers.len() {
            for i in 0..graph.layers[index].len().saturating_sub(1) {
                let before = local_crossings(graph, index);
                graph.layers[index].swap(i, i + 1);
                if local_crossings(graph, index) < before {
                    improved = true;
                } else {
                    graph.layers[index].swap(i, i + 1);
                }
            }
        }
    }
}

/// Crossings on both sides of one layer — all a single swap can affect.
fn local_crossings(graph: &Layered, index: usize) -> usize {
    let above = index
        .checked_sub(1)
        .map_or(0, |i| crossings_between(graph, i));
    let below = if index + 1 < graph.layers.len() {
        crossings_between(graph, index)
    } else {
        0
    };
    above + below
}

fn crossings(graph: &Layered) -> usize {
    (0..graph.layers.len().saturating_sub(1))
        .map(|i| crossings_between(graph, i))
        .sum()
}

/// Crossings between layer `index` and the one below it: the number of edge
/// pairs whose endpoints appear in opposite orders on the two layers.
fn crossings_between(graph: &Layered, index: usize) -> usize {
    let mut rank = vec![0_usize; graph.table.len()];
    for (position, &node) in graph.layers[index + 1].iter().enumerate() {
        rank[node] = position;
    }

    let mut lower = Vec::new();
    for &node in &graph.layers[index] {
        let mut targets: Vec<usize> = graph.succ[node].iter().map(|&n| rank[n]).collect();
        targets.sort_unstable();
        lower.extend(targets);
    }

    let mut count = 0;
    for (i, &a) in lower.iter().enumerate() {
        count += lower[i + 1..].iter().filter(|&&b| b < a).count();
    }
    count
}
