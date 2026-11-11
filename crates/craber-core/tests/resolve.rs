//! `SchemaIndex::resolve` against `cases/resolve.json`.
//!
//! The web app resolves references itself (`resolveRef` in `web/src/lib/schema.ts`)
//! to decide which edges to draw, while `--depth` and the parser resolve them
//! here. Both test suites read this one table of cases, so the rule is written
//! once and a divergence fails a test on whichever side moved.

use craber_core::{Schema, SchemaIndex, Table, TableRef};
use serde::Deserialize;

#[derive(Deserialize)]
struct Example {
    case: String,
    tables: Vec<TableRef>,
    #[serde(rename = "ref")]
    reference: TableRef,
    expect: Option<String>,
}

#[test]
fn resolves_every_shared_case() {
    let cases: Vec<Example> =
        serde_json::from_str(include_str!("cases/resolve.json")).expect("cases/resolve.json");
    assert!(!cases.is_empty());

    for case in cases {
        let mut schema = Schema::new("cases");
        for table in &case.tables {
            let mut t = Table::new(table.name.clone());
            t.schema.clone_from(&table.schema);
            schema.tables.push(t);
        }

        let resolved = SchemaIndex::build(&schema)
            .resolve(&case.reference)
            .map(|id| schema.tables[id.0].qualified_name());
        assert_eq!(resolved, case.expect, "{}", case.case);
    }
}
