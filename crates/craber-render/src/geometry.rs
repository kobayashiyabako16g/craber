//! Where things are: the box a table occupies, and the extent of the diagram.
//!
//! Both the node drawing and the edge routing need a table's rectangle, so it
//! lives on its own rather than inside one of them. Nothing here emits SVG.
//!
//! The coordinates are read out of the schema, never computed: layout runs in
//! `craber-layout` and this crate deliberately does not depend on it.

use craber_core::{Detail, Schema, Table, node_size};

/// A table with no position has not been through `craber-layout`. Drawing it at
/// the origin keeps the renderer total rather than making callers prove they
/// ran the layout first.
///
/// Each detail level has its own coordinates, because the node heights the
/// layout packed differ between them.
pub(crate) fn origin(table: &Table, detail: Detail) -> (f32, f32) {
    table.positions.map_or((0.0, 0.0), |positions| {
        let position = positions.at(detail);
        (position.x, position.y)
    })
}

/// The box a table occupies, for edge routing.
pub(crate) fn table_rect(table: &Table, detail: Detail) -> Rect {
    let (x, y) = origin(table, detail);
    let (width, height) = node_size(table, detail);
    Rect {
        x,
        y,
        width,
        height,
    }
}

/// An axis-aligned box: a table's extent, or the whole diagram's.
///
/// One type rather than the `Rect` and `Bounds` that used to sit fourteen lines
/// apart with the same four fields — they differ in what they are a box *of*,
/// which is the caller's business, not the box's.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    pub fn center(self) -> (f32, f32) {
        (self.x + self.width / 2.0, self.y + self.height / 2.0)
    }

    /// Top-left corner, for the emitter's `(x, y)`.
    pub fn origin(self) -> (f32, f32) {
        (self.x, self.y)
    }

    /// Extent, for the emitter's `(width, height)`.
    pub fn size(self) -> (f32, f32) {
        (self.width, self.height)
    }
}

/// Everything the diagram covers, plus `padding` on each side.
pub(crate) fn bounds(schema: &Schema, detail: Detail, padding: f32) -> Rect {
    let (mut left, mut top) = (f32::MAX, f32::MAX);
    let (mut right, mut bottom) = (f32::MIN, f32::MIN);
    for table in &schema.tables {
        let (x, y) = origin(table, detail);
        let (width, height) = node_size(table, detail);
        left = left.min(x);
        top = top.min(y);
        right = right.max(x + width);
        bottom = bottom.max(y + height);
    }
    if left > right {
        // No tables: still emit a valid, if empty, document.
        return Rect {
            x: 0.0,
            y: 0.0,
            width: padding * 2.0,
            height: padding * 2.0,
        };
    }
    Rect {
        x: left - padding,
        y: top - padding,
        width: right - left + padding * 2.0,
        height: bottom - top + padding * 2.0,
    }
}
