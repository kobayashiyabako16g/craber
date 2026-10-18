//! sqlparser AST → CrabER IR.
//!
//! Translation runs in two passes. `CREATE TABLE` and `CREATE TYPE` define
//! things and go first; `ALTER TABLE`, `CREATE INDEX` and `COMMENT ON` amend
//! something that must already exist and go second. Splitting them is what
//! makes a `pg_dump` readable: dumps emit every table first and then a long
//! tail of `ALTER TABLE ... ADD CONSTRAINT FOREIGN KEY`, and a single-pass
//! translator would resolve those against a schema that is still half-empty.

use craber_core::{
    Action, Column, Diagnostic, EnumType, ForeignKey, Outcome, Schema, Table, TableIndex, TableRef,
};
use sqlparser::ast::{
    AlterTableOperation, ColumnDef, ColumnOption, CommentObject, CreateIndex, CreateTable,
    CreateTableOptions, Expr, ForeignKeyConstraint, IndexColumn, ObjectName, ReferentialAction,
    SqlOption, Statement, TableConstraint, UserDefinedTypeRepresentation,
};

use crate::dialect::Dialect;

/// First pass: statements that bring a table or a type into existence.
pub(crate) fn define(statement: &Statement, dialect: Dialect, outcome: &mut Outcome<Schema>) {
    match statement {
        Statement::CreateTable(create) => {
            let table = create_table(create, outcome);
            if !is_internal(&table.name, dialect) {
                outcome.value.tables.push(table);
            }
        }
        Statement::CreateType {
            name,
            representation: Some(UserDefinedTypeRepresentation::Enum { labels }),
        } => {
            let (schema, name) = split_object_name(name);
            outcome.value.enums.push(EnumType {
                schema,
                name,
                values: labels.iter().map(|l| l.value.clone()).collect(),
            });
        }
        _ => {}
    }
}

/// Second pass: statements that modify a table defined by the first pass.
pub(crate) fn amend(statement: &Statement, outcome: &mut Outcome<Schema>) {
    match statement {
        Statement::AlterTable(alter) => alter_table(&alter.name, &alter.operations, outcome),
        Statement::CreateIndex(create) => create_index(create, outcome),
        Statement::Comment {
            object_type,
            object_name,
            comment: Some(text),
            ..
        } => comment(*object_type, object_name, text, outcome),
        // Everything else (grants, functions, extensions, transaction control)
        // has no representation on an ER diagram. Skipping it is not news.
        _ => {}
    }
}

fn create_table(create: &CreateTable, outcome: &mut Outcome<Schema>) -> Table {
    let (schema, name) = split_object_name(&create.name);
    let mut table = Table::new(name);
    table.schema = schema;
    table.comment = create
        .comment
        .as_ref()
        .map(ToString::to_string)
        .or_else(|| option_comment(&create.table_options));
    let context = table.qualified_name();

    for def in &create.columns {
        add_column(&mut table, def);
    }

    for constraint in &create.constraints {
        if let Some(diagnostic) = add_constraint(&mut table, constraint) {
            outcome.push(diagnostic.with_context(format!("{context}: {constraint}")));
        }
    }

    table
}

fn alter_table(
    name: &ObjectName,
    operations: &[AlterTableOperation],
    outcome: &mut Outcome<Schema>,
) {
    let Some(index) = find_table(&outcome.value, &split_object_name(name)) else {
        // A dump can legitimately alter something CrabER never turned into a
        // node — a view, a partition, a table filtered out upstream.
        return;
    };

    for operation in operations {
        let table = &mut outcome.value.tables[index];
        let context = table.qualified_name();
        let diagnostic = match operation {
            AlterTableOperation::AddConstraint { constraint, .. } => {
                add_constraint(table, constraint)
            }
            AlterTableOperation::AddColumn { column_def, .. } => {
                add_column(table, column_def);
                None
            }
            AlterTableOperation::DropColumn { column_names, .. } => {
                for name in column_names {
                    drop_column(table, &name.value);
                }
                None
            }
            // These change what the diagram should show, but applying them
            // needs bookkeeping (name history, constraint identity) that has no
            // payoff on a dump, where the tail is almost entirely ADD
            // CONSTRAINT. Say so rather than showing a stale node.
            AlterTableOperation::AlterColumn { .. }
            | AlterTableOperation::RenameColumn { .. }
            | AlterTableOperation::RenameTable { .. }
            | AlterTableOperation::ChangeColumn { .. }
            | AlterTableOperation::ModifyColumn { .. }
            | AlterTableOperation::DropConstraint { .. }
            | AlterTableOperation::DropPrimaryKey { .. }
            | AlterTableOperation::DropForeignKey { .. } => {
                Some(Diagnostic::warning("ALTER TABLE operation not applied"))
            }
            // Ownership, storage, partitioning, row-level security: invisible on
            // an ER diagram, and pg_dump emits them by the dozen.
            _ => None,
        };
        if let Some(diagnostic) = diagnostic {
            outcome.push(diagnostic.with_context(format!("{context}: {operation}")));
        }
    }
}

fn create_index(create: &CreateIndex, outcome: &mut Outcome<Schema>) {
    let Some(index) = find_table(&outcome.value, &split_object_name(&create.table_name)) else {
        return;
    };
    let columns = index_columns(&create.columns);
    if columns.is_empty() {
        // A purely expression-based index has no column to hang off a node.
        return;
    }
    outcome.value.tables[index].indexes.push(TableIndex {
        name: create.name.as_ref().map(|n| split_object_name(n).1),
        columns,
        unique: create.unique,
    });
}

fn comment(
    object_type: CommentObject,
    object_name: &ObjectName,
    text: &str,
    outcome: &mut Outcome<Schema>,
) {
    let parts = name_parts(object_name);
    match object_type {
        CommentObject::Table => {
            if let Some(index) = find_table(&outcome.value, &split_parts(&parts)) {
                outcome.value.tables[index].comment = Some(text.to_string());
            }
        }
        CommentObject::Column => {
            // `schema.table.column`, or `table.column` when unqualified.
            let Some((column, rest)) = parts.split_last() else {
                return;
            };
            let Some(index) = find_table(&outcome.value, &split_parts(rest)) else {
                return;
            };
            if let Some(column) = outcome.value.tables[index]
                .columns
                .iter_mut()
                .find(|c| &c.name == column)
            {
                column.comment = Some(text.to_string());
            }
        }
        _ => {}
    }
}

fn add_column(table: &mut Table, def: &ColumnDef) {
    let mut column = Column::new(def.name.value.clone(), def.data_type.to_string());
    for option in &def.options {
        match &option.option {
            ColumnOption::NotNull => column.nullable = false,
            ColumnOption::Null => column.nullable = true,
            ColumnOption::Default(expr) => column.default = Some(expr.to_string()),
            ColumnOption::Comment(comment) => column.comment = Some(comment.clone()),
            ColumnOption::PrimaryKey(_) => {
                // A column-level PRIMARY KEY implies NOT NULL even when the
                // DDL does not spell it out.
                column.nullable = false;
                table.pk.push(column.name.clone());
            }
            ColumnOption::Unique(unique) => table.indexes.push(TableIndex {
                name: unique.name.as_ref().map(|i| i.value.clone()),
                columns: vec![column.name.clone()],
                unique: true,
            }),
            ColumnOption::ForeignKey(fk) => {
                let mut foreign_key = foreign_key(fk);
                // Column-level REFERENCES has no column list of its own.
                if foreign_key.columns.is_empty() {
                    foreign_key.columns = vec![column.name.clone()];
                }
                table.fks.push(foreign_key);
            }
            _ => {}
        }
    }
    table.columns.push(column);
}

fn drop_column(table: &mut Table, name: &str) {
    table.columns.retain(|c| c.name != name);
    table.pk.retain(|c| c != name);
    table
        .indexes
        .retain(|i| !i.columns.iter().any(|c| c == name));
    table.fks.retain(|fk| !fk.columns.iter().any(|c| c == name));
}

/// Apply a table constraint, returning a diagnostic when it carries something
/// the diagram cannot show.
///
/// Returns rather than pushes because the caller may be holding a `&mut Table`
/// borrowed out of the very `Outcome` a diagnostic would go into.
fn add_constraint(table: &mut Table, constraint: &TableConstraint) -> Option<Diagnostic> {
    match constraint {
        TableConstraint::PrimaryKey(pk) => {
            table.pk = index_columns(&pk.columns);
            None
        }
        TableConstraint::Unique(unique) => {
            table.indexes.push(TableIndex {
                name: unique
                    .name
                    .as_ref()
                    .or(unique.index_name.as_ref())
                    .map(|i| i.value.clone()),
                columns: index_columns(&unique.columns),
                unique: true,
            });
            None
        }
        TableConstraint::ForeignKey(fk) => {
            table.fks.push(foreign_key(fk));
            None
        }
        // MySQL spells a plain index inside CREATE TABLE as `KEY`/`INDEX`, and
        // a dump has one per index. It says nothing about uniqueness, so it
        // never changes a relationship's cardinality — but it is still what the
        // details panel lists under the table.
        TableConstraint::Index(index) => {
            table.indexes.push(TableIndex {
                name: index.name.as_ref().map(|i| i.value.clone()),
                columns: index_columns(&index.columns),
                unique: false,
            });
            None
        }
        // A full-text or spatial key is an access path, not a constraint: it
        // makes no column unique and draws nothing. Same as CHECK — not shown,
        // and not worth a warning either.
        TableConstraint::Check(_) | TableConstraint::FulltextOrSpatial(_) => None,
        // `... USING INDEX <name>` names an index instead of listing columns,
        // so there is nothing to hang off a node. That is a real loss, unlike
        // the arms above, so it keeps the warning.
        _ => Some(Diagnostic::warning(
            "table constraint not represented in the diagram",
        )),
    }
}

fn foreign_key(fk: &ForeignKeyConstraint) -> ForeignKey {
    let (schema, name) = split_object_name(&fk.foreign_table);
    let mut foreign_key = ForeignKey::declared(
        fk.columns.iter().map(|i| i.value.clone()).collect(),
        TableRef { schema, name },
        fk.referred_columns
            .iter()
            .map(|i| i.value.clone())
            .collect(),
    );
    foreign_key.name = fk.name.as_ref().map(|i| i.value.clone());
    foreign_key.on_delete = action(fk.on_delete.as_ref());
    foreign_key.on_update = action(fk.on_update.as_ref());
    foreign_key
}

fn action(action: Option<&ReferentialAction>) -> Action {
    match action {
        Some(ReferentialAction::Restrict) => Action::Restrict,
        Some(ReferentialAction::Cascade) => Action::Cascade,
        Some(ReferentialAction::SetNull) => Action::SetNull,
        Some(ReferentialAction::SetDefault) => Action::SetDefault,
        Some(ReferentialAction::NoAction) | None => Action::NoAction,
    }
}

/// Locate a table for a second-pass statement to amend.
///
/// A qualified name matches exactly. An unqualified one matches by bare name,
/// but only when that is unambiguous — the same rule
/// [`SchemaIndex::resolve`](craber_core::SchemaIndex::resolve) uses for foreign
/// keys, so a reference and an `ALTER TABLE` never disagree about which table
/// they mean.
fn find_table(schema: &Schema, (qualifier, name): &(Option<String>, String)) -> Option<usize> {
    if qualifier.is_some()
        && let Some(index) = schema
            .tables
            .iter()
            .position(|t| &t.schema == qualifier && &t.name == name)
    {
        return Some(index);
    }
    let mut matches = schema
        .tables
        .iter()
        .enumerate()
        .filter(|(_, t)| &t.name == name)
        .map(|(i, _)| i);
    let first = matches.next()?;
    matches.next().is_none().then_some(first)
}

/// MySQL puts `COMMENT='...'` among the trailing table options rather than in
/// the `COMMENT` clause sqlparser exposes as `CreateTable::comment`, so a dump's
/// table comments only arrive through here.
fn option_comment(options: &CreateTableOptions) -> Option<String> {
    let options = match options {
        CreateTableOptions::With(options)
        | CreateTableOptions::Options(options)
        | CreateTableOptions::Plain(options)
        | CreateTableOptions::TableProperties(options) => options.as_slice(),
        CreateTableOptions::None => &[],
    };
    options.iter().find_map(|option| match option {
        SqlOption::Comment(comment) => Some(comment.to_string()),
        _ => None,
    })
}

/// True for a table the engine maintains rather than the schema author.
///
/// `sqlite3 .schema` prints `sqlite_sequence` next to the real tables whenever
/// anything uses AUTOINCREMENT, and `sqlite_stat1` once ANALYZE has run. They
/// are part of the file, not part of the model, and drawing them puts a
/// typeless two-column node in the middle of the diagram.
fn is_internal(name: &str, dialect: Dialect) -> bool {
    dialect == Dialect::Sqlite && name.to_ascii_lowercase().starts_with("sqlite_")
}

fn name_parts(name: &ObjectName) -> Vec<String> {
    name.0
        .iter()
        .filter_map(|p| p.as_ident())
        .map(|i| i.value.clone())
        .collect()
}

/// `schema.table` → `(Some("schema"), "table")`. Longer paths (`db.schema.table`)
/// keep only the last two parts, which is what the diagram namespaces by.
fn split_object_name(name: &ObjectName) -> (Option<String>, String) {
    split_parts(&name_parts(name))
}

fn split_parts(parts: &[String]) -> (Option<String>, String) {
    match parts {
        [] => (None, String::new()),
        [only] => (None, only.clone()),
        [.., schema, table] => (Some(schema.clone()), table.clone()),
    }
}

/// Index/key columns, dropping anything that is not a plain column reference
/// (expression indexes have no column to attach to a diagram node).
fn index_columns(columns: &[IndexColumn]) -> Vec<String> {
    columns
        .iter()
        .filter_map(|c| match &c.column.expr {
            Expr::Identifier(ident) => Some(ident.value.clone()),
            Expr::CompoundIdentifier(parts) => parts.last().map(|i| i.value.clone()),
            _ => None,
        })
        .collect()
}
