//! Node geometry: the numbers the layout reserves and the browser draws.
//!
//! The layout reserves a box and the browser draws into it, so a node height
//! that disagrees with the stylesheet is not an error but a diagram whose
//! tables overlap.
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
fn node_height_matches_what_the_browser_lays_out() {
    use craber_core::metrics;

    #[allow(
        clippy::float_cmp,
        reason = "exact pixel constants, not computed measurements"
    )]
    for (rows, expected) in [(0, 42.0), (2, 87.0), (3, 109.0), (5, 153.0)] {
        assert_eq!(
            metrics::node_height(rows),
            expected,
            "a node with {rows} rows measures {expected}px in the browser"
        );
    }
}
