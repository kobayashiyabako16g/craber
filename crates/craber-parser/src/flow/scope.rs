//! Which tables a column name can mean.

use std::collections::HashMap;

use craber_core::{ColumnRef, TableRef};
use sqlparser::ast::{Expr, ObjectName};

use crate::names::object_ref;

/// The tables a statement can name columns of, by alias and by name.
#[derive(Default, Clone)]
pub(super) struct Scope {
    names: HashMap<String, TableRef>,
    /// Every table in scope, so an unqualified column can be attributed when
    /// there is exactly one.
    pub(super) tables: Vec<TableRef>,
    /// CTE names: referring to one reads the CTE, not a table on the diagram.
    pub(super) ctes: Vec<String>,
}

impl Scope {
    /// The same names, for a nested `SELECT`: the outer query's tables stay
    /// visible to a correlated subquery, but an unqualified column means the
    /// innermost table.
    pub(super) fn nested(&self) -> Self {
        Self {
            tables: Vec::new(),
            ..self.clone()
        }
    }

    pub(super) fn bind(&mut self, table: &TableRef, alias: Option<&str>) {
        self.names
            .insert(alias.unwrap_or(&table.name).to_owned(), table.clone());
        self.names.insert(table.qualified_name(), table.clone());
        self.tables.push(table.clone());
    }

    /// The table `name` means here — an alias, a bare name or a qualified one.
    pub(super) fn table(&self, name: &ObjectName) -> Option<TableRef> {
        self.names.get(&object_ref(name).qualified_name()).cloned()
    }

    /// The column `expr` names, or `None` when it is not a column or the SQL
    /// does not say whose it is.
    pub(super) fn column_of(&self, expr: &Expr) -> Option<ColumnRef> {
        match expr {
            Expr::Identifier(ident) => self.column(std::slice::from_ref(&ident.value)),
            Expr::CompoundIdentifier(parts) => {
                let parts: Vec<String> = parts.iter().map(|p| p.value.clone()).collect();
                self.column(&parts)
            }
            Expr::Nested(inner) => self.column_of(inner),
            _ => None,
        }
    }

    /// The table a dotted column reference means, or `None` when the SQL does
    /// not say — including a qualifier that names no table in scope, which is
    /// a typo or an outer reference this statement cannot see.
    pub(super) fn column(&self, parts: &[String]) -> Option<ColumnRef> {
        let (column, qualifier) = parts.split_last()?;
        let table = match qualifier {
            [] => match self.tables.as_slice() {
                [only] => only.clone(),
                _ => return None,
            },
            [table] => self.names.get(table)?.clone(),
            [.., schema, table] => self.names.get(&format!("{schema}.{table}"))?.clone(),
        };
        Some(ColumnRef {
            table,
            column: column.clone(),
        })
    }
}

/// True for a column reference, whether or not the scope can say whose. A
/// condition between two columns is a join, not a value.
pub(super) fn is_column(expr: &Expr) -> bool {
    match expr {
        Expr::Identifier(_) | Expr::CompoundIdentifier(_) => true,
        Expr::Nested(inner) => is_column(inner),
        _ => false,
    }
}
