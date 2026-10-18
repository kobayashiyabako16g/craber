//! What the SVG has to get right regardless of how the diagram looks.

use craber_core::{
    Cardinality, Column, Detail, ForeignKey, Position, Positions, Schema, Table, TableRef,
};
use craber_layout::{Layout, SugiyamaLayout};
use craber_render::{Options, Theme, to_svg, to_svg_with};

fn shop() -> Schema {
    let mut schema = Schema::new("shop");

    let mut users = Table::new("users");
    users.schema = Some("public".into());
    users.columns = vec![Column::new("id", "bigint"), Column::new("email", "text")];
    users.pk = vec!["id".into()];

    let mut orders = Table::new("orders");
    orders.schema = Some("public".into());
    orders.columns = vec![
        Column::new("id", "bigint"),
        Column::new("user_id", "bigint"),
    ];
    orders.pk = vec!["id".into()];
    orders.fks = vec![ForeignKey::new(
        "user_id",
        TableRef::qualified("public", "users"),
        "id",
    )];

    schema.tables = vec![users, orders];
    SugiyamaLayout::default().apply(&mut schema);
    schema
}

#[test]
fn renders_a_standalone_document() {
    let svg = to_svg(&shop());
    assert!(svg.starts_with("<svg xmlns=\"http://www.w3.org/2000/svg\""));
    assert!(svg.trim_end().ends_with("</svg>"));
    // Self-contained: no stylesheet, script or image to fetch.
    assert!(!svg.contains("<script"));
    assert!(!svg.contains("href="));
}

#[test]
fn every_table_is_drawn_and_labelled() {
    let svg = to_svg(&shop());
    assert!(svg.contains(r#"id="public.users""#));
    assert!(svg.contains(r#"id="public.orders""#));
    assert!(svg.contains("users<"), "the table name is written out");
    assert!(svg.contains("user_id<"), "columns are written out");
}

/// The whole point of the crate: a diagram exported to a file has to be the
/// same picture the browser draws, at the same coordinates.
#[test]
fn the_viewbox_contains_every_node() {
    let schema = shop();
    let svg = to_svg(&schema);
    let view_box = svg
        .split(r#"viewBox=""#)
        .nth(1)
        .and_then(|rest| rest.split('"').next())
        .expect("a viewBox");
    let numbers: Vec<f32> = view_box
        .split_whitespace()
        .map(|n| n.parse().expect("a number"))
        .collect();
    let [x, y, width, height] = numbers[..] else {
        panic!("expected four numbers, got {view_box}");
    };

    for table in &schema.tables {
        let position = table.positions.unwrap().at(Detail::FULL);
        let (node_width, node_height) = craber_core::node_size(table, Detail::FULL);
        assert!(position.x >= x && position.x + node_width <= x + width);
        assert!(position.y >= y && position.y + node_height <= y + height);
    }
}

/// A relationship read from a constraint and one CrabER guessed from a name
/// have to be distinguishable at a glance, or the guesses quietly become facts.
#[test]
fn an_inferred_relationship_is_dashed_and_a_declared_one_is_not() {
    let mut schema = shop();
    assert!(!to_svg(&schema).contains("stroke-dasharray"));

    schema.tables[1].fks[0].inferred = true;
    assert!(to_svg(&schema).contains("stroke-dasharray"));
}

#[test]
fn cardinality_picks_the_marker_at_the_referencing_end() {
    let mut schema = shop();
    // Three lines converge on the node for the crow's foot; a one-to-one end
    // gets a single bar instead.
    let many = to_svg(&schema).matches("<line").count();
    schema.tables[1].fks[0].cardinality = Cardinality::OneToOne;
    let one = to_svg(&schema).matches("<line").count();
    assert!(
        many > one,
        "a crow's foot should be more strokes than a single bar ({many} vs {one})"
    );
}

/// A self-reference has no second node to run to, and a loop back to the same
/// box would say nothing the FK badge on the column does not.
#[test]
fn a_self_reference_draws_no_edge() {
    let mut schema = Schema::new("org");
    let mut employees = Table::new("employees");
    employees.columns = vec![
        Column::new("id", "bigint"),
        Column::new("manager_id", "bigint"),
    ];
    employees.fks = vec![ForeignKey::new(
        "manager_id",
        TableRef::new("employees"),
        "id",
    )];
    employees.positions = Some(Positions::uniform(Position::new(0.0, 0.0)));
    schema.tables = vec![employees];

    let svg = to_svg(&schema);
    // The node header is a path too, so look inside the edge group rather than
    // counting paths across the whole document.
    let edges = svg
        .split(r#"<g stroke-linecap="round" fill="none">"#)
        .nth(1)
        .and_then(|rest| rest.split("</g>").next())
        .expect("an edge group");
    assert!(edges.trim().is_empty(), "no edge should be drawn: {edges}");
    assert!(svg.contains("manager_id<"), "the column is still shown");
}

/// Identifiers come from a user's database and can contain anything. An
/// unescaped one would produce a document no viewer can open.
#[test]
fn identifiers_are_escaped() {
    let mut schema = Schema::new("odd");
    let mut table = Table::new(r#"a<b>&"c""#);
    table.columns = vec![Column::new("x & y", "text")];
    table.positions = Some(Positions::uniform(Position::new(0.0, 0.0)));
    schema.tables = vec![table];

    let svg = to_svg(&schema);
    assert!(svg.contains("a&lt;b&gt;&amp;"));
    assert!(!svg.contains("a<b>"));
}

#[test]
fn the_theme_is_the_only_thing_a_light_render_changes() {
    let schema = shop();
    let dark = to_svg(&schema);
    let light = to_svg_with(
        &schema,
        &Options {
            theme: Theme::light(),
            ..Options::default()
        },
    );
    assert!(dark.contains("#16181d") && !dark.contains("#ffffff"));
    assert!(light.contains("#ffffff") && !light.contains("#16181d"));
    // Same geometry, different paint.
    assert_eq!(
        dark.matches("<rect").count(),
        light.matches("<rect").count()
    );
    assert_eq!(
        dark.matches("<text").count(),
        light.matches("<text").count()
    );
}

#[test]
fn an_empty_schema_still_renders_a_valid_document() {
    let svg = to_svg(&Schema::new("empty"));
    assert!(svg.starts_with("<svg"));
    assert!(svg.trim_end().ends_with("</svg>"));
}

#[test]
fn the_rendered_diagram_is_stable() {
    insta::assert_snapshot!(to_svg(&shop()));
}
