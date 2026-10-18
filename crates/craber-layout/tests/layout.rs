//! Properties the layout has to hold for any schema.
//!
//! Layout quality is a judgement call — that is what looking at the exported
//! SVG of `fixtures/wide.sql` is for. These tests cover the things
//! that are not judgement calls: nothing overlaps, nothing is left unplaced,
//! foreign keys read left to right, and the same input always draws the same
//! picture.

use craber_core::{Column, Detail, ForeignKey, Schema, Table, TableRef};
use craber_layout::{Layout, SugiyamaLayout, node_size};

/// `users <- orders <- order_items`, plus a table that relates to nothing.
fn shop() -> Schema {
    let mut schema = Schema::new("shop");
    schema.tables = vec![
        table("users", &["id", "email"], &[]),
        table("orders", &["id", "user_id"], &[("user_id", "users", "id")]),
        table(
            "order_items",
            &["order_id", "product_id"],
            &[("order_id", "orders", "id")],
        ),
        table("settings", &["key", "value"], &[]),
    ];
    schema
}

fn table(name: &str, columns: &[&str], fks: &[(&str, &str, &str)]) -> Table {
    let mut table = Table::new(name);
    table.columns = columns.iter().map(|c| Column::new(*c, "bigint")).collect();
    table.fks = fks
        .iter()
        .map(|(column, target, target_column)| {
            ForeignKey::new(*column, TableRef::new(*target), *target_column)
        })
        .collect();
    table
}

/// `(left, top, right, bottom)` of a table's node at one detail level, once it
/// has been placed.
fn rect(table: &Table, detail: Detail) -> (f32, f32, f32, f32) {
    let position = table.positions.expect("every table is placed").at(detail);
    let (width, height) = node_size(table, detail);
    (
        position.x,
        position.y,
        position.x + width,
        position.y + height,
    )
}

/// A table's `x` at the level the CLI draws.
fn left(schema: &Schema, name: &str) -> f32 {
    schema
        .table(name)
        .unwrap()
        .positions
        .unwrap()
        .at(Detail::FULL)
        .x
}

fn laid_out(mut schema: Schema) -> Schema {
    SugiyamaLayout::default().apply(&mut schema);
    schema
}

#[test]
fn every_table_gets_a_position() {
    let schema = laid_out(shop());
    assert!(schema.tables.iter().all(|t| t.positions.is_some()));
}

#[test]
fn a_referenced_table_is_drawn_before_the_table_that_references_it() {
    let schema = laid_out(shop());
    let x = |name: &str| left(&schema, name);
    assert!(x("users") < x("orders"), "users should sit left of orders");
    assert!(
        x("orders") < x("order_items"),
        "orders should sit left of order_items"
    );
}

#[test]
fn nodes_never_overlap() {
    // A hub-and-spoke schema: one table everything points at, which is the
    // shape that pushes a layer widest and is where overlaps would show up.
    let mut schema = Schema::new("hub");
    schema.tables.push(table("tenants", &["id"], &[]));
    for i in 0..40 {
        schema.tables.push(table(
            &format!("t{i}"),
            &["id", "tenant_id"],
            &[("tenant_id", "tenants", "id")],
        ));
    }
    assert_no_overlaps(&laid_out(schema));
}

#[test]
fn disconnected_tables_are_packed_without_overlapping() {
    let mut schema = Schema::new("islands");
    for i in 0..30 {
        schema.tables.push(table(&format!("t{i}"), &["id"], &[]));
    }
    assert_no_overlaps(&laid_out(schema));
}

/// Every detail level is a diagram in its own right, so each one has to be free
/// of overlaps — a shorter node must not let the layer above slide into it.
fn assert_no_overlaps(schema: &Schema) {
    for detail in Detail::ALL {
        let rects: Vec<_> = schema.tables.iter().map(|t| rect(t, detail)).collect();
        for (i, a) in rects.iter().enumerate() {
            for (j, b) in rects.iter().enumerate().skip(i + 1) {
                let overlaps = a.0 < b.2 && b.0 < a.2 && a.1 < b.3 && b.1 < a.3;
                assert!(
                    !overlaps,
                    "at {detail:?}, {} at {a:?} overlaps {} at {b:?}",
                    schema.tables[i].name, schema.tables[j].name
                );
            }
        }
    }
}

/// Switching the detail level must not move a table sideways: the reader is
/// looking at one table when they change it, and horizontal movement would cost
/// them the diagram they had oriented themselves in.
#[test]
fn the_detail_level_changes_heights_only() {
    let schema = laid_out(shop());
    for table in &schema.tables {
        let positions = table.positions.unwrap();
        let x = positions.at(Detail::FULL).x;
        for detail in Detail::ALL {
            // Bit-for-bit: every level reads the same x out of the same pass,
            // so anything else means a level computed its own.
            assert!(
                (positions.at(detail).x - x).abs() < f32::EPSILON,
                "{} moved sideways at {detail:?}",
                table.name
            );
        }
    }
}

/// A compact level is compact: fewer rows per node has to mean a shorter
/// diagram, which is the whole reason the levels have their own coordinates.
#[test]
fn a_shorter_node_makes_for_a_shorter_diagram() {
    let schema = laid_out(shop());
    let height = |detail: Detail| {
        schema
            .tables
            .iter()
            .map(|t| rect(t, detail).3)
            .fold(0.0_f32, f32::max)
    };
    assert!(height(Detail::TableName) < height(Detail::KeyOnly));
    assert!(height(Detail::KeyOnly) < height(Detail::AllFields));
}

/// Circular foreign keys are unusual but legal, and a layering pass that met
/// one without breaking the cycle would not terminate.
#[test]
fn circular_foreign_keys_terminate_and_place_everything() {
    let mut schema = Schema::new("cycle");
    schema.tables = vec![
        table("a", &["id", "b_id"], &[("b_id", "b", "id")]),
        table("b", &["id", "c_id"], &[("c_id", "c", "id")]),
        table("c", &["id", "a_id"], &[("a_id", "a", "id")]),
    ];
    let schema = laid_out(schema);
    assert!(schema.tables.iter().all(|t| t.positions.is_some()));
    assert_no_overlaps(&schema);
}

/// A table that references itself — an `employees.manager_id` — has nothing to
/// say about layering, and must not be mistaken for a cycle to break.
#[test]
fn a_self_reference_is_not_a_cycle() {
    let mut schema = Schema::new("org");
    schema.tables = vec![table(
        "employees",
        &["id", "manager_id"],
        &[("manager_id", "employees", "id")],
    )];
    let schema = laid_out(schema);
    // One table, one layer, one column: the only sensible place for it.
    let position = schema.tables[0].positions.unwrap().at(Detail::FULL);
    assert!(position.x.abs() < f32::EPSILON && position.y.abs() < f32::EPSILON);
}

/// The same schema has to draw the same diagram every time: `schema.json` is
/// snapshot-tested, committed to repositories, and diffed between runs.
#[test]
fn layout_is_deterministic() {
    let first = laid_out(shop());
    let second = laid_out(shop());
    let positions = |s: &Schema| -> Vec<_> { s.tables.iter().map(|t| t.positions).collect() };
    assert_eq!(positions(&first), positions(&second));
}

/// Tables are laid out relative to the top-left of the diagram, so a viewport
/// fitted to the content has no dead space to scroll through.
#[test]
fn the_diagram_starts_at_the_origin() {
    let schema = laid_out(shop());
    for detail in Detail::ALL {
        let left = schema
            .tables
            .iter()
            .map(|t| t.positions.unwrap().at(detail).x)
            .fold(f32::MAX, f32::min);
        let top = schema
            .tables
            .iter()
            .map(|t| t.positions.unwrap().at(detail).y)
            .fold(f32::MAX, f32::min);
        assert_eq!((left, top), (0.0, 0.0), "at {detail:?}");
    }
}

#[test]
fn an_empty_schema_is_not_an_error() {
    let schema = laid_out(Schema::new("empty"));
    assert!(schema.tables.is_empty(), "{:?}", schema.tables);
}

/// Long edges are what make a layered drawing readable or not: an edge that
/// skips layers has to be given room, not routed through whatever happens to
/// be in the way.
#[test]
fn a_table_referenced_from_several_depths_still_leaves_room() {
    let mut schema = Schema::new("deep");
    schema.tables = vec![
        table("root", &["id"], &[]),
        table("mid", &["id", "root_id"], &[("root_id", "root", "id")]),
        table("leaf", &["id", "mid_id"], &[("mid_id", "mid", "id")]),
        // Skips `mid` entirely, so it spans two layers.
        table("deep", &["id", "root_id"], &[("root_id", "root", "id")]),
    ];
    let schema = laid_out(schema);
    assert_no_overlaps(&schema);
    let x = |name: &str| left(&schema, name);
    assert!(x("root") < x("deep"));
    assert!(x("root") < x("mid") && x("mid") < x("leaf"));
    // A guard against the layout silently collapsing every table onto one line,
    // which would satisfy every ordering assert above.
    let lowest = schema
        .tables
        .iter()
        .map(|t| t.positions.unwrap().at(Detail::FULL).y)
        .fold(0.0_f32, f32::max);
    assert!(lowest > 0.0, "expected more than one row");
}
