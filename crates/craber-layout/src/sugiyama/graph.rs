//! Stages 1 and 2: the schema becomes a graph, the graph becomes layers.
//!
//! [`arcs`] is the only function in the whole algorithm that reads a
//! [`Schema`]. Everything after it works in `usize` node ids, which is what
//! lets the layering be tested — and reasoned about — as plain graph theory.

use std::collections::{BTreeSet, VecDeque};

use craber_core::{Schema, SchemaIndex};

use super::{Arcs, Layered};

/// Foreign keys as layering arcs, pointing from the referenced table down to
/// the table that references it — so a parent table is drawn above its
/// children, which is how an ER diagram is read.
pub(super) fn arcs(schema: &Schema) -> Arcs {
    let index = SchemaIndex::build(schema);
    index
        .edges(schema)
        .into_iter()
        .filter(|edge| edge.from != edge.to)
        .map(|edge| (edge.to.0, edge.from.0))
        .collect()
}

/// Connected components over the undirected FK graph, each sorted, and in the
/// order their first table appears in the schema.
pub(super) fn components(nodes: usize, arcs: &Arcs) -> Vec<Vec<usize>> {
    let mut neighbours = vec![Vec::new(); nodes];
    for &(u, v) in arcs {
        neighbours[u].push(v);
        neighbours[v].push(u);
    }

    let mut seen = vec![false; nodes];
    let mut components = Vec::new();
    for root in 0..nodes {
        if seen[root] {
            continue;
        }
        seen[root] = true;
        let mut members = vec![root];
        let mut queue = VecDeque::from([root]);
        while let Some(node) = queue.pop_front() {
            for &next in &neighbours[node] {
                if !seen[next] {
                    seen[next] = true;
                    members.push(next);
                    queue.push_back(next);
                }
            }
        }
        members.sort_unstable();
        components.push(members);
    }
    components
}

/// Stages 1 and 2: break cycles, assign layers, and split long edges.
pub(super) fn layered(schema: &Schema, members: &[usize], arcs: &Arcs) -> Layered {
    let mut local = vec![usize::MAX; schema.tables.len()];
    for (i, &table) in members.iter().enumerate() {
        local[table] = i;
    }
    let internal: Vec<(usize, usize)> = arcs
        .iter()
        .filter(|(u, _)| local[*u] != usize::MAX)
        .map(|&(u, v)| (local[u], local[v]))
        .collect();

    let acyclic = break_cycles(members.len(), &internal);
    let layer = assign_layers(members.len(), &acyclic);

    let mut graph = Layered {
        table: members.iter().copied().map(Some).collect(),
        layers: Vec::new(),
        succ: vec![Vec::new(); members.len()],
        pred: vec![Vec::new(); members.len()],
    };

    // Split every edge that spans more than one layer into a chain through
    // dummies. Stages 3 and 4 then only ever look one layer up or down,
    // and a long edge takes part in ordering instead of being ignored by it.
    let mut node_layer = layer.clone();
    for &(u, v) in &acyclic {
        let mut from = u;
        for depth in (layer[u] + 1)..layer[v] {
            let dummy = graph.table.len();
            graph.table.push(None);
            graph.succ.push(Vec::new());
            graph.pred.push(Vec::new());
            node_layer.push(depth);
            graph.succ[from].push(dummy);
            graph.pred[dummy].push(from);
            from = dummy;
        }
        graph.succ[from].push(v);
        graph.pred[v].push(from);
    }

    let depth = node_layer.iter().copied().max().unwrap_or(0) + 1;
    let mut layers = vec![Vec::new(); depth];
    for (node, &at) in node_layer.iter().enumerate() {
        layers[at].push(node);
    }
    // Tightening can vacate a layer, and an empty one would still take up a
    // layer's worth of height. Nothing reads the absolute layer number, so
    // dropping them is free — and no arc can cross one, since any arc that
    // did would have left a dummy behind.
    layers.retain(|layer: &Vec<usize>| !layer.is_empty());
    graph.layers = layers;
    graph
}

/// Where a node is in the cycle-breaking search: not reached, on the current
/// path (so an arc back to it closes a cycle), or finished.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Visit {
    Unvisited,
    OnStack,
    Done,
}

/// Reverse the back edges a depth-first search finds, which is enough to make
/// any digraph acyclic. Circular foreign keys are rare but real, and a layering
/// pass that met one would otherwise never terminate.
fn break_cycles(nodes: usize, arcs: &[(usize, usize)]) -> Vec<(usize, usize)> {
    let mut succ = vec![Vec::new(); nodes];
    for &(u, v) in arcs {
        succ[u].push(v);
    }

    let mut state = vec![Visit::Unvisited; nodes];
    let mut out = BTreeSet::new();

    for root in 0..nodes {
        if state[root] != Visit::Unvisited {
            continue;
        }
        state[root] = Visit::OnStack;
        let mut stack = vec![(root, 0_usize)];
        // `last_mut` rather than the `last` the loop would read: the cursor has
        // to be advanced before the arc is followed, or a node with two
        // successors would follow the first one twice.
        while let Some(&mut (node, ref mut cursor)) = stack.last_mut() {
            let next = *cursor;
            if next == succ[node].len() {
                state[node] = Visit::Done;
                stack.pop();
                continue;
            }
            *cursor += 1;
            let target = succ[node][next];
            match state[target] {
                Visit::OnStack => {
                    // The back edge that closes a cycle. Reversing it keeps the
                    // relationship in the layering instead of dropping it.
                    out.insert((target, node));
                }
                Visit::Done => {
                    out.insert((node, target));
                }
                Visit::Unvisited => {
                    out.insert((node, target));
                    state[target] = Visit::OnStack;
                    stack.push((target, 0));
                }
            }
        }
    }
    // Reversal can turn one arc into a duplicate of another, and a self-arc if
    // two tables reference each other. Neither says anything about layering.
    out.into_iter().filter(|(u, v)| u != v).collect()
}

/// Longest-path layering, then tightened downwards: every node sits directly
/// above its highest successor. Without the second half every table that
/// nothing references is stranded on the top row, which for a real schema is
/// most of them.
fn assign_layers(nodes: usize, arcs: &[(usize, usize)]) -> Vec<usize> {
    let mut succ = vec![Vec::new(); nodes];
    let mut incoming = vec![0_usize; nodes];
    for &(u, v) in arcs {
        succ[u].push(v);
        incoming[v] += 1;
    }

    let mut queue: VecDeque<usize> = (0..nodes).filter(|&n| incoming[n] == 0).collect();
    let mut order = Vec::with_capacity(nodes);
    let mut layer = vec![0_usize; nodes];
    while let Some(node) = queue.pop_front() {
        order.push(node);
        for &next in &succ[node] {
            layer[next] = layer[next].max(layer[node] + 1);
            incoming[next] -= 1;
            if incoming[next] == 0 {
                queue.push_back(next);
            }
        }
    }

    for &node in order.iter().rev() {
        if let Some(highest) = succ[node].iter().map(|&n| layer[n]).min() {
            layer[node] = highest - 1;
        }
    }
    layer
}
