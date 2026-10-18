//! How big a node is: the numbers, and the two rules that use them.
//!
//! Separate from [`theme`](crate::theme) because size and colour are different
//! questions with different audiences. A wrong colour is a diagram that looks
//! off; a wrong size is a diagram whose tables overlap, because the layout
//! reserved a box the browser then drew something bigger into.
//!
//! This lives in `craber-core` rather than in `craber-layout` because both the
//! layout and the SVG renderer need it, and they must agree to the pixel. It is
//! also what lets `craber-render` stay independent of `craber-layout` — the
//! renderer needs node extents, not an algorithm.

use crate::ir::{Column, Detail, Table};

/// Space the diagram occupies, in the units [`crate::Position`] uses.
///
/// `web/src/theme.css` sizes a node from the same numbers, so the box the
/// layout reserves is the box the browser draws. Content heights are stated and
/// the borders are added, because that is how CSS composes them — see
/// [`metrics::node_height`].
pub mod metrics {
    /// `--craber-node-width`.
    pub const NODE_WIDTH: f32 = 260.0;
    /// `--craber-node-header`: the header row's height.
    pub const HEADER_CONTENT: f32 = 40.0;
    /// `--craber-node-row`: one column row's height.
    pub const ROW_CONTENT: f32 = 22.0;
    /// `--craber-sidebar-width`. Web-only; here so the stylesheet has one source.
    pub const SIDEBAR_WIDTH: f32 = 230.0;
    /// `--craber-radius`, the node corner.
    pub const RADIUS: f32 = 8.0;

    /// `.craber-node`'s own border, top and bottom.
    pub const BORDER: f32 = 1.0;
    /// The rule `.craber-node-columns` draws between the header and the columns.
    pub const HEADER_RULE: f32 = 1.0;

    /// What the browser actually lays out for a node of `rows` rows.
    ///
    /// `web/src/theme.css` sets `box-sizing: border-box` globally, so the 1px
    /// rule between two column rows sits *inside* the row's
    /// `--craber-node-row` and a row costs exactly `ROW_CONTENT`. The parts
    /// that do add up are the node's own border (top and bottom) and the rule
    /// under the header — and that rule belongs to `.craber-node-columns`,
    /// which `TableNode.tsx` does not render at all when a table has no
    /// columns, so a bare node is exactly its header plus its border.
    #[must_use]
    #[allow(
        clippy::cast_precision_loss,
        reason = "column counts are far below f32 precision limits"
    )]
    pub fn css_node_height(rows: usize) -> f32 {
        let columns = if rows == 0 {
            0.0
        } else {
            HEADER_RULE + rows as f32 * ROW_CONTENT
        };
        BORDER * 2.0 + HEADER_CONTENT + columns
    }

    /// The height `craber-layout` reserves for a node of `rows` rows.
    ///
    /// This is deliberately *not* [`css_node_height`]: it counts a row as
    /// `ROW_CONTENT + 1` and the header as `HEADER_CONTENT + 2`, so it comes out
    /// `rows - 1` pixels taller than the node the browser draws.
    ///
    /// Reserving more than the node needs is safe — it only ever pushes tables
    /// further apart, and `SugiyamaLayout::node_gap` puts 60px between them
    /// anyway — whereas reserving less would let two tables touch. It is left
    /// as it is because every `schema.json` ever written carries coordinates
    /// computed from it, so tightening it moves every table in every published
    /// diagram. [`reservation_is_never_short`] pins the property that matters.
    ///
    /// [`reservation_is_never_short`]: ../../../tests/generated_tokens.rs
    #[must_use]
    #[allow(
        clippy::cast_precision_loss,
        reason = "column counts are far below f32 precision limits"
    )]
    pub fn node_height(rows: usize) -> f32 {
        HEADER_CONTENT + BORDER * 2.0 + rows as f32 * (ROW_CONTENT + BORDER)
    }
}

/// On-canvas size of a table node, in the same units as [`crate::Position`].
///
/// The width is the same at every detail level; only the number of rows changes.
#[must_use]
pub fn node_size(table: &Table, detail: Detail) -> (f32, f32) {
    let rows = visible_columns(table, detail).len();
    (metrics::NODE_WIDTH, metrics::node_height(rows))
}

/// The column rows a node draws at this detail level.
///
/// The layout, the SVG renderer and `web/src/TableNode.tsx` all have to agree on
/// this, so the rule lives in one place and both Rust callers share it.
#[must_use]
pub fn visible_columns(table: &Table, detail: Detail) -> Vec<&Column> {
    match detail {
        Detail::TableName => Vec::new(),
        Detail::KeyOnly => {
            let keys: std::collections::BTreeSet<&str> = table
                .pk
                .iter()
                .map(String::as_str)
                .chain(
                    table
                        .fks
                        .iter()
                        .flat_map(|fk| fk.columns.iter().map(String::as_str)),
                )
                .collect();
            table
                .columns
                .iter()
                .filter(|column| keys.contains(column.name.as_str()))
                .collect()
        }
        Detail::AllFields => table.columns.iter().collect(),
    }
}
