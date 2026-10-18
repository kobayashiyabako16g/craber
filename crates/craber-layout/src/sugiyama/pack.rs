//! Arranging the laid-out components on the page.
//!
//! Not one of the classic four stages. A real schema is usually one large web
//! plus a scattering of tables that relate to nothing, and each of those is a
//! component of its own — so the algorithm runs once per component and this
//! puts the results side by side. It reads nothing but [`Placement`].

use craber_core::Detail;

use super::{LEVELS, Placement};

/// Shelf-pack the components into rows, widest first, aiming for a roughly
/// landscape overall shape. Returns each component with its origin.
pub(super) fn pack(boxes: &[Placement], gap: f32) -> Vec<(&Placement, (f32, [f32; LEVELS]))> {
    // Which component goes on which shelf is decided once, from the tallest
    // detail level, and reused for all of them. Deciding it per level would
    // move components sideways when the reader changed the level.
    let full = Detail::FULL.level();
    let total: f32 = boxes.iter().map(|b| b.width * b.heights[full]).sum();
    // 16:9-ish. A diagram that fills a screen beats one that scrolls.
    let target = (total * 1.8)
        .sqrt()
        .max(boxes.iter().fold(0.0, |w, b| f32::max(w, b.width)));

    let mut order: Vec<usize> = (0..boxes.len()).collect();
    order.sort_by(|&a, &b| {
        boxes[b].heights[full]
            .total_cmp(&boxes[a].heights[full])
            .then(boxes[b].width.total_cmp(&boxes[a].width))
            .then(a.cmp(&b))
    });

    let mut placed = Vec::with_capacity(boxes.len());
    let mut x = 0.0_f32;
    let mut tops = [0.0_f32; LEVELS];
    let mut shelf = [0.0_f32; LEVELS];
    for i in order {
        let placement = &boxes[i];
        if x > 0.0 && x + placement.width > target {
            x = 0.0;
            for (level, top) in tops.iter_mut().enumerate() {
                *top += shelf[level] + gap;
                shelf[level] = 0.0;
            }
        }
        placed.push((placement, (x, tops)));
        x += placement.width + gap;
        for (level, tallest) in shelf.iter_mut().enumerate() {
            *tallest = tallest.max(placement.heights[level]);
        }
    }
    placed
}
