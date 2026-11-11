//! Object names from the AST, as the IR spells them.

use craber_core::TableRef;
use sqlparser::ast::ObjectName;

/// Every identifier in a dotted name, unquoted.
pub(crate) fn name_parts(name: &ObjectName) -> Vec<String> {
    name.0
        .iter()
        .filter_map(|p| p.as_ident())
        .map(|i| i.value.clone())
        .collect()
}

/// `schema.table` → a qualified [`TableRef`]. Longer paths (`db.schema.table`)
/// keep only the last two parts, which is what the diagram namespaces by.
pub(crate) fn object_ref(name: &ObjectName) -> TableRef {
    parts_ref(&name_parts(name))
}

/// [`object_ref`] for a name already split into its parts.
pub(crate) fn parts_ref(parts: &[String]) -> TableRef {
    match parts {
        [] => TableRef::new(String::new()),
        [only] => TableRef::new(only.clone()),
        [.., schema, table] => TableRef::qualified(schema.clone(), table.clone()),
    }
}
