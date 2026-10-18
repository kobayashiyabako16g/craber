//! Relationship lines, and the crow's-foot markers at their ends.

use craber_core::{Cardinality, Detail, ForeignKey, Schema, SchemaIndex, Table};

use crate::Theme;
use crate::geometry::{Rect, table_rect};
use crate::svg::Xml;

/// How far from a node the crow's foot and the "one" tick sit.
const FOOT_LENGTH: f32 = 12.0;
const FOOT_SPREAD: f32 = 6.0;
const TICK_OFFSET: f32 = 11.0;
const TICK_SPREAD: f32 = 5.0;
/// The second bar of `one and only one`, and where the `zero` ring sits: both
/// further out than the mark they qualify.
const SECOND_TICK_OFFSET: f32 = 17.0;
const RING_OFFSET: f32 = 19.0;
const RING_RADIUS: f32 = 3.0;

/// Every stroke a relationship is made of — the curve, the foot, the ticks, the
/// ring — is this wide. Written once so they cannot end up drawn at two
/// weights, which reads as two kinds of relationship.
const STROKE_WIDTH: f32 = 1.5;

/// How one end of a relationship line is drawn.
///
/// The same three CrabER can derive from DDL as the web app draws — see
/// `EndKind` and `endKinds` in `web/src/diagram/graph.ts`, and
/// `annotate_relationships` for where the two facts behind them come from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum End {
    /// A fork, and a ring for "or none": the many side of a many-to-one.
    ZeroOrMany,
    /// A bar and a ring: at most one row, and it may not be there.
    ZeroOrOne,
    /// Two bars: exactly one row, always.
    OneOnly,
}

impl End {
    /// The end at the table that holds the foreign key.
    fn referencing(fk: &ForeignKey) -> Self {
        match fk.cardinality {
            Cardinality::OneToOne => Self::ZeroOrOne,
            Cardinality::ManyToOne => Self::ZeroOrMany,
        }
    }

    /// The end at the table it points at. Never "many": a foreign key names one
    /// row there, and only its nullability decides whether that row must exist.
    fn referenced(fk: &ForeignKey) -> Self {
        if fk.optional {
            Self::ZeroOrOne
        } else {
            Self::OneOnly
        }
    }
}

pub(crate) fn render_all(
    schema: &Schema,
    index: &SchemaIndex,
    detail: Detail,
    theme: &Theme,
    out: &mut Xml,
) {
    for edge in index.edges(schema) {
        if edge.from == edge.to {
            // A self-reference has no second node to run to. Drawing it as a
            // loop would say nothing the FK badge on the column does not.
            continue;
        }
        let child = &schema.tables[edge.from.0];
        let parent = &schema.tables[edge.to.0];
        let fk = &child.fks[edge.fk];
        render(child, parent, detail, fk, theme, out);
    }
}

fn render(
    child: &Table,
    parent: &Table,
    detail: Detail,
    fk: &ForeignKey,
    theme: &Theme,
    out: &mut Xml,
) {
    let inferred = fk.inferred;
    let (from, to) = (table_rect(child, detail), table_rect(parent, detail));
    let (start, start_out) = anchor(from, to);
    let (end, end_out) = anchor(to, from);

    // Control points pushed out along each node's own facing direction, so the
    // line leaves a table perpendicular to its edge instead of at a shallow
    // angle that is hard to trace back.
    let reach = distance(start, end).mul_add(0.4, 20.0).min(120.0);
    let dashes = if inferred {
        // Relationships CrabER inferred from naming rather than read from a
        // constraint are dashed, so the two are never confused.
        r#" stroke-dasharray="5 4""#
    } else {
        ""
    };

    out.path(
        &format!(
            "M{:.1},{:.1} C{:.1},{:.1} {:.1},{:.1} {:.1},{:.1}",
            start.0,
            start.1,
            start_out.0.mul_add(reach, start.0),
            start_out.1.mul_add(reach, start.1),
            end_out.0.mul_add(reach, end.0),
            end_out.1.mul_add(reach, end.1),
            end.0,
            end.1,
        ),
        &format!(
            r#"stroke="{}" stroke-width="{STROKE_WIDTH}"{dashes}"#,
            theme.edge
        ),
    );

    end_marker(End::referencing(fk), start, start_out, theme, out);
    end_marker(End::referenced(fk), end, end_out, theme, out);
}

/// Draws one end's notation, outwards from the node it touches.
fn end_marker(kind: End, at: (f32, f32), out_dir: (f32, f32), theme: &Theme, out: &mut Xml) {
    match kind {
        End::ZeroOrMany => {
            crow_foot(at, out_dir, theme, out);
            ring(at, out_dir, theme, out);
        }
        End::ZeroOrOne => {
            tick(at, out_dir, TICK_OFFSET, theme, out);
            ring(at, out_dir, theme, out);
        }
        End::OneOnly => {
            tick(at, out_dir, TICK_OFFSET, theme, out);
            tick(at, out_dir, SECOND_TICK_OFFSET, theme, out);
        }
    }
}

/// The "or none" ring, out beyond whatever mark it qualifies.
fn ring(at: (f32, f32), out_dir: (f32, f32), theme: &Theme, out: &mut Xml) {
    out.circle(
        (
            out_dir.0.mul_add(RING_OFFSET, at.0),
            out_dir.1.mul_add(RING_OFFSET, at.1),
        ),
        RING_RADIUS,
        &format!(
            r#"fill="none" stroke="{}" stroke-width="{STROKE_WIDTH}""#,
            theme.edge
        ),
    );
}

/// Where a line leaves `rect` on its way to `towards`, and the outward unit
/// direction there.
///
/// Layout puts a parent to the left of its children, so most edges leave
/// through a side. Top and bottom are for the rest: two tables on the same
/// layer, which happens when a cycle was broken between them.
fn anchor(rect: Rect, towards: Rect) -> ((f32, f32), (f32, f32)) {
    let (cx, cy) = rect.center();
    let (tx, ty) = towards.center();

    let horizontal_gap = if tx < rect.x {
        rect.x - tx
    } else if tx > rect.x + rect.width {
        tx - (rect.x + rect.width)
    } else {
        0.0
    };

    if horizontal_gap > 0.0 {
        // Slide the anchor towards the other table rather than pinning it to
        // the centre, so several edges on one table fan out instead of piling
        // into a single point.
        let y = ty.clamp(rect.y + 16.0, rect.y + rect.height - 16.0);
        if tx < cx {
            ((rect.x, y), (-1.0, 0.0))
        } else {
            ((rect.x + rect.width, y), (1.0, 0.0))
        }
    } else {
        let x = tx.clamp(rect.x + 24.0, rect.x + rect.width - 24.0);
        if ty < cy {
            ((x, rect.y), (0.0, -1.0))
        } else {
            ((x, rect.y + rect.height), (0.0, 1.0))
        }
    }
}

/// The "many" marker: three lines splaying from a point out on the line back to
/// the node's edge.
fn crow_foot(at: (f32, f32), out_dir: (f32, f32), theme: &Theme, out: &mut Xml) {
    let apex = (
        out_dir.0.mul_add(FOOT_LENGTH, at.0),
        out_dir.1.mul_add(FOOT_LENGTH, at.1),
    );
    let across = perpendicular(out_dir);
    for offset in [-FOOT_SPREAD, 0.0, FOOT_SPREAD] {
        out.draw_line(
            apex,
            (
                across.0.mul_add(offset, at.0),
                across.1.mul_add(offset, at.1),
            ),
            &stroke(theme),
        );
    }
}

/// A single bar across the line, `offset` out from the node.
fn tick(at: (f32, f32), out_dir: (f32, f32), offset: f32, theme: &Theme, out: &mut Xml) {
    let base = (
        out_dir.0.mul_add(offset, at.0),
        out_dir.1.mul_add(offset, at.1),
    );
    let across = perpendicular(out_dir);
    out.draw_line(
        (
            across.0.mul_add(-TICK_SPREAD, base.0),
            across.1.mul_add(-TICK_SPREAD, base.1),
        ),
        (
            across.0.mul_add(TICK_SPREAD, base.0),
            across.1.mul_add(TICK_SPREAD, base.1),
        ),
        &stroke(theme),
    );
}

/// The stroke attributes the crow's-foot notation is drawn with.
fn stroke(theme: &Theme) -> String {
    format!(r#"stroke="{}" stroke-width="{STROKE_WIDTH}""#, theme.edge)
}

fn perpendicular((x, y): (f32, f32)) -> (f32, f32) {
    (-y, x)
}

fn distance(a: (f32, f32), b: (f32, f32)) -> f32 {
    (b.0 - a.0).hypot(b.1 - a.1)
}
