//! Node geometry: the numbers the layout reserves and the browser draws.
//!
//! These are properties of `craber_core::geometry::metrics` rather than of any
//! generated file, and the reason they are pinned is this: the layout reserves
//! a box and the browser draws into it, so a disagreement between the two
//! formulas is not an error but a diagram whose tables overlap.
//!
//! The generator that writes `web/src/generated` is a unit test in
//! `src/codegen.rs`, beside the functions it runs.

/// What the browser actually draws, measured against `web/src/theme.css`.
///
/// These numbers were read off the rendered DOM rather than derived: with
/// `box-sizing: border-box` the 1px rule between two column rows sits inside
/// `--craber-node-row`, so a row costs 22px and not 23. The node adds its own
/// border top and bottom, plus the rule under the header.
#[test]
fn css_node_height_matches_what_the_browser_lays_out() {
    use craber_core::geometry::metrics;

    #[allow(
        clippy::float_cmp,
        reason = "exact pixel constants, not computed measurements"
    )]
    for (rows, expected) in [(0, 42.0), (2, 87.0), (3, 109.0), (5, 153.0)] {
        assert_eq!(
            metrics::css_node_height(rows),
            expected,
            "a node with {rows} rows measures {expected}px in the browser"
        );
    }
}

/// The layout may reserve more room than a node needs, but never less.
///
/// This is the property the diagram actually depends on: extra space only
/// pushes tables apart, while a short reservation lets two of them touch — and
/// the frontend cannot fix that, because it draws the coordinates it is given.
#[test]
fn reservation_is_never_short() {
    use craber_core::geometry::metrics;

    for rows in 0..200 {
        let reserved = metrics::node_height(rows);
        let drawn = metrics::css_node_height(rows);
        assert!(
            reserved >= drawn,
            "a {rows}-row node draws {drawn}px but the layout reserved {reserved}px"
        );
    }
}

/// The gap between the two is `rows - 1` pixels, for a table that has columns. Pinned so that changing either
/// formula is a deliberate act rather than a surprise — closing it would move
/// every table in every `schema.json` ever written.
#[test]
fn the_layout_over_reserves_by_one_pixel_per_extra_row() {
    use craber_core::geometry::metrics;

    #[allow(
        clippy::float_cmp,
        clippy::cast_precision_loss,
        reason = "exact pixel constants at row counts far below f32's mantissa"
    )]
    for rows in 1..200 {
        let slack = metrics::node_height(rows) - metrics::css_node_height(rows);
        assert_eq!(slack, (rows - 1) as f32, "{rows} rows");
    }
}

/// The tokens the stylesheet sizes a node from.
#[test]
fn the_node_metrics_are_the_ones_the_stylesheet_uses() {
    use craber_core::geometry::metrics;

    #[allow(
        clippy::float_cmp,
        reason = "exact pixel constants, not computed measurements"
    )]
    {
        assert_eq!(metrics::NODE_WIDTH, 260.0);
        assert_eq!(metrics::HEADER_CONTENT, 40.0);
        assert_eq!(metrics::ROW_CONTENT, 22.0);
        assert_eq!(metrics::BORDER, 1.0);
        assert_eq!(metrics::HEADER_RULE, 1.0);
    }
}
