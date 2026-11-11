//! The step being recorded.

use craber_core::{Access, ColumnRef, ColumnValue, Join, TableRef, Touch, ValueKind};

#[derive(Default)]
pub(super) struct StepBuilder {
    touches: Vec<Touch>,
    pub(super) joins: Vec<Join>,
}

impl StepBuilder {
    pub(super) fn finish(self) -> (Vec<Touch>, Vec<Join>) {
        (self.touches, self.joins)
    }

    /// Record that `table` is touched, and return its record. A table touched
    /// twice keeps the stronger access — an `UPDATE` that also filters on its
    /// own table writes it.
    pub(super) fn touch(&mut self, table: &TableRef, access: Access) -> &mut Touch {
        let at = self
            .touches
            .iter()
            .position(|t| &t.table == table)
            .unwrap_or_else(|| {
                self.touches.push(Touch {
                    table: table.clone(),
                    access,
                    columns: Vec::new(),
                    values: Vec::new(),
                });
                self.touches.len() - 1
            });
        let touch = &mut self.touches[at];
        touch.access = touch.access.max(access);
        touch
    }

    pub(super) fn touch_column(&mut self, column: &ColumnRef, access: Access) -> &mut Touch {
        let touch = self.touch(&column.table, access);
        if !touch.columns.contains(&column.column) {
            touch.columns.push(column.column.clone());
        }
        touch
    }

    /// Record a value the statement names for `column`, touching it with
    /// `access` if nothing has yet.
    pub(super) fn value(
        &mut self,
        column: &ColumnRef,
        access: Access,
        kind: ValueKind,
        text: String,
    ) {
        let touch = self.touch_column(column, access);
        let value = ColumnValue {
            column: column.column.clone(),
            kind,
            text,
        };
        if !touch.values.contains(&value) {
            touch.values.push(value);
        }
    }
}
