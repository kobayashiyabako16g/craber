//! The 100-table layout check, as a test.
//!
//! 100 tables is the size at which the layout has to prove itself, on the
//! grounds that an ER tool is judged almost entirely on whether its diagrams
//! are readable. Readability is a judgement call and a test cannot make it —
//! what a test can do is hold the measurable parts still, so that a layout
//! change which quietly breaks one of them is caught rather than discovered in
//! a screenshot months later.

mod common;

use craber_core::geometry::metrics;

use common::{assert_success, craber, fixture};

fn wide() -> serde_json::Value {
    let wide = fixture("wide.sql");
    let output = craber(&["export", "--file", wide.to_str().unwrap()]);
    assert_success(&output, "exporting fixtures/wide.sql");
    // The fixture is ordinary PostgreSQL. Anything CrabER warns about here is
    // DDL coverage it is missing, not something the fixture should work around.
    assert_eq!(
        String::from_utf8_lossy(&output.stderr).trim(),
        "",
        "the fixture should parse cleanly"
    );
    serde_json::from_slice(&output.stdout).expect("valid JSON")
}

/// `(left, top, right, bottom)` for a table at the level the CLI draws,
/// matching `craber_layout::node_size`.
///
/// The box comes from `craber_core::geometry::metrics` rather than from numbers
/// written out here. Restating 260 and `42 + rows * 23` would make this test
/// agree with a *copy* of the geometry: the layout could then move every table
/// in the diagram and these assertions would still pass, which is the one
/// failure a size test exists to catch.
fn rect(table: &serde_json::Value) -> (f64, f64, f64, f64) {
    let position = table
        .get("positions")
        .and_then(|positions| positions.get("allFields"))
        .unwrap_or_else(|| panic!("{} has no position", table["name"]));
    let x = position["x"].as_f64().unwrap();
    let y = position["y"].as_f64().unwrap();
    let rows = table["columns"].as_array().map_or(0, Vec::len);
    let height = f64::from(metrics::node_height(rows));
    (x, y, x + f64::from(metrics::NODE_WIDTH), y + height)
}

#[test]
fn a_hundred_tables_are_read_and_placed() {
    let schema = wide();
    let tables = schema["tables"].as_array().unwrap();
    assert_eq!(tables.len(), 100);

    let relationships: usize = tables
        .iter()
        .map(|t| t["fks"].as_array().map_or(0, Vec::len))
        .sum();
    assert_eq!(
        relationships, 136,
        "every ALTER TABLE ... ADD CONSTRAINT should have become an edge"
    );
}

#[test]
fn no_two_tables_overlap_at_a_hundred_tables() {
    let schema = wide();
    let tables = schema["tables"].as_array().unwrap();
    let rects: Vec<_> = tables.iter().map(rect).collect();
    for (i, a) in rects.iter().enumerate() {
        for (j, b) in rects.iter().enumerate().skip(i + 1) {
            assert!(
                !(a.0 < b.2 && b.0 < a.2 && a.1 < b.3 && b.1 < a.3),
                "{} overlaps {}",
                tables[i]["name"],
                tables[j]["name"]
            );
        }
    }
}

/// Layering is what makes a diagram readable: a referenced table above the
/// tables that reference it, so relationships are followed downwards. A layout
/// that scattered tables would still pass the overlap test.
#[test]
fn foreign_keys_read_left_to_right() {
    let schema = wide();
    let tables = schema["tables"].as_array().unwrap();

    let mut by_name = std::collections::HashMap::new();
    for table in tables {
        by_name.insert(table["name"].as_str().unwrap(), rect(table));
    }

    let (mut forwards, mut total) = (0, 0);
    for table in tables {
        let from = by_name[table["name"].as_str().unwrap()];
        for fk in table["fks"].as_array().into_iter().flatten() {
            let Some(&to) = by_name.get(fk["refTable"]["name"].as_str().unwrap()) else {
                continue;
            };
            total += 1;
            if to.0 < from.0 {
                forwards += 1;
            }
        }
    }
    // Not all of them, necessarily: breaking a cycle has to point some edge the
    // other way, and a foreign key onto a table in the same layer has nowhere
    // to go. A line that does read backwards is the one the reader has to trace
    // around a node, so the bar is high.
    assert!(
        forwards * 10 >= total * 9,
        "only {forwards} of {total} relationships read left to right"
    );
}

/// A diagram twenty screens long and one screen across is technically a
/// diagram. Keeping the aspect ratio in check is what stops the layout from
/// solving overlaps by spreading everything along one axis.
#[test]
fn the_diagram_stays_roughly_square() {
    let schema = wide();
    let rects: Vec<_> = schema["tables"]
        .as_array()
        .unwrap()
        .iter()
        .map(rect)
        .collect();
    let width = rects.iter().fold(0.0_f64, |w, r| w.max(r.2));
    let height = rects.iter().fold(0.0_f64, |h, r| h.max(r.3));
    let aspect = (width / height).max(height / width);
    assert!(
        aspect < 5.0,
        "diagram is {width:.0} x {height:.0}, an aspect ratio of {aspect:.1}:1"
    );
}

#[test]
fn a_hundred_tables_render_to_svg() {
    let wide = fixture("wide.sql");
    let output = craber(&[
        "export",
        "--file",
        wide.to_str().unwrap(),
        "--format",
        "svg",
    ]);
    assert_success(&output, "rendering fixtures/wide.sql as SVG");

    let svg = String::from_utf8(output.stdout).expect("UTF-8");
    assert_eq!(svg.matches("<g id=").count(), 100, "one group per table");
    assert!(svg.trim_end().ends_with("</svg>"));
}
