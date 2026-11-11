//! Diagram layout: turning a [`Schema`] into coordinates.
//!
//! Layout runs in Rust rather than in the browser on purpose: positions are
//! part of `schema.json`, so every consumer of it — the static site, `serve`,
//! the WASM demo — draws the same diagram, and the frontend never re-lays-out
//! on load.

mod sugiyama;

use craber_core::Schema;

pub use sugiyama::SugiyamaLayout;

/// Assigns a [`Position`](craber_core::Position) to every table in a schema.
pub trait Layout {
    fn apply(&self, schema: &mut Schema);
}
