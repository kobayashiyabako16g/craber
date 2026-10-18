//! Drawing one table: the box, its header, and a row per column.
//!
//! The counterpart to [`edge`](crate::edge). Between them they are everything
//! that ends up on the canvas — a node and a relationship — and they are
//! separate for the same reason the diagram has two kinds of thing in it.

use craber_core::{Detail, Table, Theme, node_size, visible_columns};

use crate::geometry::origin;
use crate::svg::{self, Xml};

/// The content heights [`node_size`] reserves room for, so the rows drawn here
/// fit inside the box the layout made space for.
const HEADER_HEIGHT: f32 = craber_core::geometry::metrics::HEADER_CONTENT;
const ROW_HEIGHT: f32 = craber_core::geometry::metrics::ROW_CONTENT;
/// Where the first column row starts below the header rule.
const ROW_INSET: f32 = 6.0;
const CORNER: f32 = craber_core::geometry::metrics::RADIUS;
const PADDING_X: f32 = 10.0;

/// The accent dot in the header, and how far past it the title starts.
const DOT_RADIUS: f32 = 3.0;
const TITLE_INSET: f32 = 16.0;
/// How far into a row the column name starts — clear of the `PK`/`FK` badge
/// drawn at `PADDING_X`.
const COLUMN_INSET: f32 = 24.0;
/// Clear air between a column name and the type right-aligned after it.
const NAME_TYPE_GAP: f32 = 8.0;

/// Type sizes. `TYPE_SIZE` and `NAME_SIZE` are also what the text is *measured*
/// at when deciding where to truncate, so they are one value each rather than a
/// number in the attribute and a matching one in the call to
/// [`svg::text_width`].
const TITLE_SIZE: f32 = 12.0;
const NAME_SIZE: f32 = 12.0;
const TYPE_SIZE: f32 = 11.0;
const BADGE_SIZE: f32 = 9.0;

pub(crate) fn node(table: &Table, detail: Detail, theme: &Theme, out: &mut Xml) {
    let at = origin(table, detail);
    let size = node_size(table, detail);

    out.open(
        "g",
        &format!(r#"id="{}""#, svg::escape(&table.qualified_name())),
    );
    header(table, at, size, theme, out);
    rows(table, detail, at, size, theme, out);
    out.close("g");
}

/// The box, its rounded top, and the title line.
fn header(
    table: &Table,
    (x, y): (f32, f32),
    (width, height): (f32, f32),
    theme: &Theme,
    out: &mut Xml,
) {
    out.rect(
        (x, y),
        (width, height),
        &format!(
            r#"rx="{CORNER}" fill="{}" stroke="{}""#,
            theme.surface, theme.border
        ),
    );
    // The header is the node's rounded top only, so it has to be a path: an
    // `rx` rect here would round the corners where it meets the columns.
    out.path(
        &format!(
            "M{x:.1},{:.1} a{CORNER},{CORNER} 0 0 1 {CORNER},-{CORNER} h{:.1} a{CORNER},{CORNER} 0 0 1 {CORNER},{CORNER} v{:.1} h-{width:.1} Z",
            y + CORNER,
            width - CORNER * 2.0,
            HEADER_HEIGHT - CORNER,
        ),
        &format!(r#"fill="{}""#, theme.surface_raised),
    );
    out.draw_line(
        (x, y + HEADER_HEIGHT),
        (x + width, y + HEADER_HEIGHT),
        &format!(r#"stroke="{}""#, theme.border),
    );
    out.circle(
        (x + PADDING_X + DOT_RADIUS, y + HEADER_HEIGHT / 2.0),
        DOT_RADIUS,
        &format!(r#"fill="{}""#, theme.accent),
    );

    let title = match &table.schema {
        Some(schema) => format!(
            r#"<tspan fill="{}">{}.</tspan>{}"#,
            theme.text_muted,
            svg::escape(schema),
            svg::escape(&table.name)
        ),
        None => svg::escape(&table.name),
    };
    out.draw_text(
        (x + PADDING_X + TITLE_INSET, y + HEADER_HEIGHT / 2.0),
        &format!(
            r#"font-size="{TITLE_SIZE}" font-weight="600" fill="{}""#,
            theme.text
        ),
        &title,
    );
}

/// One row per visible column: its key badge, its name, and its type.
fn rows(
    table: &Table,
    detail: Detail,
    (x, y): (f32, f32),
    (width, _): (f32, f32),
    theme: &Theme,
    out: &mut Xml,
) {
    let pk: Vec<&str> = table.pk.iter().map(String::as_str).collect();
    let fk: Vec<&str> = table
        .fks
        .iter()
        .flat_map(|f| f.columns.iter().map(String::as_str))
        .collect();

    for (i, column) in visible_columns(table, detail).into_iter().enumerate() {
        #[allow(
            clippy::cast_precision_loss,
            reason = "column counts are far below f32 precision limits"
        )]
        let row_y = y + HEADER_HEIGHT + ROW_INSET + i as f32 * ROW_HEIGHT + ROW_HEIGHT / 2.0;

        // The same two colours `web/src/TableNode.tsx` gives its key glyphs: the
        // accent for a primary key, the FK tint for a foreign one. They used to
        // differ — the browser drew FK in a tan and an exported SVG drew it in
        // the muted grey — which made the same schema two different pictures.
        let (badge, badge_fill) = if pk.contains(&column.name.as_str()) {
            ("PK", theme.accent)
        } else if fk.contains(&column.name.as_str()) {
            ("FK", theme.glyph_fk)
        } else {
            ("", theme.text_muted)
        };
        if !badge.is_empty() {
            out.draw_text(
                (x + PADDING_X, row_y),
                &format!(r#"font-size="{BADGE_SIZE}" font-weight="700" fill="{badge_fill}""#),
                badge,
            );
        }

        // The type is right-aligned and gets what it needs; the name takes the
        // rest, so a long type never pushes the column name out of the node.
        let type_text = svg::truncate(&column.data_type, 15);
        let name_width = width
            - (PADDING_X + COLUMN_INSET)
            - PADDING_X
            - svg::text_width(&type_text, TYPE_SIZE)
            - NAME_TYPE_GAP;
        let name_text = svg::truncate(&column.name, svg::fits(name_width, NAME_SIZE));

        out.draw_text(
            (x + PADDING_X + COLUMN_INSET, row_y),
            &format!(
                r#"font-size="{NAME_SIZE}" font-family="{}" fill="{}""#,
                theme.font_mono, theme.text
            ),
            &svg::escape(&name_text),
        );
        out.draw_text(
            (x + width - PADDING_X, row_y),
            &format!(
                r#"text-anchor="end" font-size="{TYPE_SIZE}" font-family="{}" fill="{}""#,
                theme.font_mono, theme.text_muted
            ),
            &svg::escape(&type_text),
        );
    }
}
