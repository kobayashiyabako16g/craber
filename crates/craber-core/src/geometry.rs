//! How big a node is: the numbers, and the two rules that use them.
//!
//! Separate from [`theme`](crate::theme) because size and colour are different
//! questions with different audiences. A wrong colour is a diagram that looks
//! off; a wrong size is a diagram whose tables overlap, because the layout
//! reserved a box the browser then drew something bigger into.
//!
//! This lives in `craber-core` rather than in `craber-layout` because the
//! stylesheet is generated from the same numbers (`codegen`), and the layout
//! and the browser must agree to the pixel.

use crate::ir::{Column, Detail, Table};

/// Declare the node metrics: each one paired with the custom property that
/// carries it into the browser, the way `theme`'s `palette!` pairs a colour.
///
/// The pairing used to be a doc comment on each constant plus a hand-written
/// list in `codegen`, two places that had to be kept in step by hand.
macro_rules! metrics {
    ($( $(#[$meta:meta])* $name:ident = $value:literal => $token:literal ),* $(,)?) => {
        $( $(#[$meta])* pub const $name: f32 = $value; )*

        /// Every metric the stylesheet carries, with its custom property.
        #[cfg_attr(not(test), allow(dead_code, reason = "read by codegen, which is test-only"))]
        pub(crate) const TOKENS: &[(&str, f32)] = &[ $( ($token, $name), )* ];
    };
}

/// Space the diagram occupies, in the units [`crate::Position`] uses.
///
/// `web/src/generated/tokens.css` is written from these numbers, so the box the
/// layout reserves is the box the browser draws. Content heights are stated and
/// the borders are added, because that is how CSS composes them — see
/// [`metrics::node_height`].
pub mod metrics {
    metrics! {
        NODE_WIDTH = 260.0 => "--craber-node-width",
        /// The header row's height.
        HEADER_CONTENT = 40.0 => "--craber-node-header",
        /// One column row's height.
        ROW_CONTENT = 22.0 => "--craber-node-row",
        /// Web-only; here so the stylesheet has one source.
        SIDEBAR_WIDTH = 230.0 => "--craber-sidebar-width",
        /// The node corner.
        RADIUS = 8.0 => "--craber-radius",
    }

    /// `.craber-node`'s own border, top and bottom.
    pub const BORDER: f32 = 1.0;
    /// The rule `.craber-node-columns` draws between the header and the columns.
    pub const HEADER_RULE: f32 = 1.0;

    /// The height of a node of `rows` rows: what the layout reserves, and what
    /// the browser draws.
    ///
    /// `web/src/theme.css` sets `box-sizing: border-box` globally, so the 1px
    /// rule between two column rows sits *inside* the row's
    /// `--craber-node-row` and a row costs exactly `ROW_CONTENT`. The parts
    /// that do add up are the node's own border (top and bottom) and the rule
    /// under the header — and that rule belongs to `.craber-node-columns`,
    /// which `TableNode.tsx` does not render at all when a table has no
    /// columns, so a bare node is exactly its header plus its border.
    ///
    /// The layout used to reserve `rows - 1` pixels more than this, a second
    /// formula kept only so that published coordinates would not move. Spacing
    /// between tables is `SugiyamaLayout::node_gap`'s job, not this one's.
    #[must_use]
    #[allow(
        clippy::cast_precision_loss,
        reason = "column counts are far below f32 precision limits"
    )]
    pub fn node_height(rows: usize) -> f32 {
        let columns = if rows == 0 {
            0.0
        } else {
            HEADER_RULE + rows as f32 * ROW_CONTENT
        };
        BORDER * 2.0 + HEADER_CONTENT + columns
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
/// The layout and `web/src/components/TableNode/TableNode.tsx` have to agree
/// on this, so the rule lives in one place.
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
