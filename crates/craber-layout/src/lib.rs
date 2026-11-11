//! Diagram layout: turning a [`Schema`] into coordinates.
//!
//! Layout runs in Rust rather than in the browser on purpose: positions are
//! part of `schema.json`, so every consumer of it — the static site, `serve`,
//! the WASM demo — draws the same diagram, and the frontend never re-lays-out
//! on load.
//!
//! [`SugiyamaLayout`] is the one CrabER draws with. [`GridLayout`] is the
//! placeholder it replaced, kept because "is the clever layout actually better
//! than putting the tables in rows?" is a question worth being able to answer.

mod grid;
mod sugiyama;

use craber_core::Schema;

pub use grid::GridLayout;
pub use sugiyama::SugiyamaLayout;

// Node extents used to live here, but the SVG renderer needs them too and
// should not have to depend on a layout algorithm to get them. They are now
// `craber_core::geometry`, re-exported so existing callers keep working.
pub use craber_core::{geometry::metrics::NODE_WIDTH, node_size, visible_columns};

/// Assigns a [`Position`](craber_core::Position) to every table in a schema.
pub trait Layout {
    fn apply(&self, schema: &mut Schema);
}
