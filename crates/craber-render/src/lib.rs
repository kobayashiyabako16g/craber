//! IR + coordinates → SVG, with no browser in the loop.
//!
//! This is what `craber export --format svg` writes and what the M1 layout
//! checkpoint is judged on. It draws from the same [`Schema`] and the same
//! [`node_size`](craber_core::node_size) the web view uses, so a table is the same size and in the same
//! place whether it is rendered here or by React Flow.

mod edge;
mod geometry;
mod node;
mod svg;

use craber_core::{Detail, Schema, SchemaIndex};

pub use craber_core::Theme;

use crate::geometry::bounds;
use crate::node::node;
use crate::svg::Xml;

/// The grid pattern's id, which is declared once and referenced once.
const GRID_ID: &str = "craber-grid";

#[derive(Debug, Clone)]
pub struct Options {
    pub theme: Theme,
    pub detail: Detail,
    /// Blank margin around the diagram, in diagram units.
    pub padding: f32,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            theme: Theme::default(),
            detail: Detail::FULL,
            padding: 40.0,
        }
    }
}

/// Render a laid-out schema as a standalone SVG document.
#[must_use]
pub fn to_svg(schema: &Schema) -> String {
    to_svg_with(schema, &Options::default())
}

#[must_use]
pub fn to_svg_with(schema: &Schema, options: &Options) -> String {
    let detail = options.detail;
    let bounds = bounds(schema, detail, options.padding);
    let theme = &options.theme;
    let mut out = Xml::new();

    out.open(
        "svg",
        &format!(
            r#"xmlns="http://www.w3.org/2000/svg" width="{:.0}" height="{:.0}" viewBox="{:.1} {:.1} {:.1} {:.1}" font-family="{}""#,
            bounds.width, bounds.height, bounds.x, bounds.y, bounds.width, bounds.height, theme.font,
        ),
    );

    // A dot grid as a tiled pattern rather than as elements: a 200-table
    // diagram would otherwise need tens of thousands of circles.
    out.open("defs", "");
    out.line(&format!(
        r#"<pattern id="{GRID_ID}" width="20" height="20" patternUnits="userSpaceOnUse"><circle cx="1" cy="1" r="1" fill="{}"/></pattern>"#,
        theme.grid
    ));
    out.close("defs");

    out.rect(
        bounds.origin(),
        bounds.size(),
        &format!(r#"fill="{}""#, theme.background),
    );
    out.rect(
        bounds.origin(),
        bounds.size(),
        &format!(r#"fill="url(#{GRID_ID})""#),
    );

    // Edges first so that a line never crosses over a table it passes near.
    out.open("g", r#"stroke-linecap="round" fill="none""#);
    edge::render_all(schema, &SchemaIndex::build(schema), detail, theme, &mut out);
    out.close("g");

    for table in &schema.tables {
        node(table, detail, theme, &mut out);
    }

    out.close("svg");
    out.finish()
}
