//! DML → [`Workflow`]: what each statement reads, writes and deletes.
//!
//! Nothing is evaluated. A step records the tables and columns its statement
//! names, which is what the viewer needs to light the part of the diagram a
//! step is about. Column names are attributed to a table only when the SQL
//! says which one — through a qualifier or because one table is in scope — so
//! that an ambiguous `id` is dropped rather than drawn on the wrong node.
//!
//! Values are recorded the same way, as the SQL writes them: what `INSERT` and
//! `UPDATE` put in a column, and what `WHERE` and `ON` hold one to. Only the
//! conditions joined by `AND` are kept — one inside an `OR` is not something
//! the row is known to satisfy, so showing it beside the column would mislead.
//!
//! The same forgiving rule as DDL applies: a statement this cannot read is a
//! warning and is skipped, never the end of the workflow.

use std::collections::HashMap;

use craber_core::{
    Access, ColumnRef, ColumnValue, Diagnostic, Join, Outcome, Step, StepKind, TableRef, Touch,
    ValueKind, Workflow,
};
use sqlparser::ast::{
    AssignmentTarget, BinaryOperator, Delete, Expr, FromTable, Insert, JoinConstraint,
    JoinOperator, Query, Select, SelectItem, SelectItemQualifiedWildcardKind, SetExpr, Statement,
    TableFactor, TableObject, TableWithJoins, Update, UpdateTableFromKind,
};

use crate::translate::{name_parts, split_object_name};

/// Read each statement of `statements` as one step of a workflow called `name`.
pub(crate) fn workflow(statements: &[Statement], name: String) -> Outcome<Workflow> {
    let mut outcome = Outcome::new(Workflow::new(name));
    for statement in statements {
        let kind = match statement {
            Statement::Query(_) => StepKind::Select,
            Statement::Insert(_) => StepKind::Insert,
            Statement::Update(_) => StepKind::Update,
            Statement::Delete(_) => StepKind::Delete,
            // Transaction control frames the steps; it is not one of them.
            Statement::StartTransaction { .. }
            | Statement::Commit { .. }
            | Statement::Rollback { .. } => continue,
            other => {
                outcome.push(Diagnostic::warning(format!(
                    "skipped a statement that is not SELECT/INSERT/UPDATE/DELETE: {}",
                    first_words(&other.to_string())
                )));
                continue;
            }
        };

        let mut step = StepBuilder::default();
        match statement {
            Statement::Query(query) => step.query(query, &Scope::default()),
            Statement::Insert(insert) => step.insert(insert),
            Statement::Update(update) => step.update(update),
            Statement::Delete(delete) => step.delete(delete),
            _ => unreachable!("filtered above"),
        }
        outcome.value.steps.push(Step {
            kind,
            sql: statement.to_string(),
            touches: step.touches,
            joins: step.joins,
        });
    }
    outcome
}

/// The tables a statement can name columns of, by alias and by name.
#[derive(Default, Clone)]
struct Scope {
    names: HashMap<String, TableRef>,
    /// Every table in scope, so an unqualified column can be attributed when
    /// there is exactly one.
    tables: Vec<TableRef>,
    /// CTE names: referring to one reads the CTE, not a table on the diagram.
    ctes: Vec<String>,
}

impl Scope {
    fn bind(&mut self, table: &TableRef, alias: Option<&str>) {
        self.names
            .insert(alias.unwrap_or(&table.name).to_owned(), table.clone());
        self.names.insert(table.qualified_name(), table.clone());
        self.tables.push(table.clone());
    }

    /// The table a column reference means, or `None` when the SQL does not say.
    fn column(&self, parts: &[String]) -> Option<ColumnRef> {
        let (column, qualifier) = parts.split_last()?;
        let table = match qualifier {
            [] => match self.tables.as_slice() {
                [only] => only.clone(),
                _ => return None,
            },
            [table] => self.names.get(table)?.clone(),
            [.., schema, table] => self
                .names
                .get(&format!("{schema}.{table}"))
                .cloned()
                .unwrap_or_else(|| TableRef::qualified(schema, table)),
        };
        Some(ColumnRef {
            table,
            column: column.clone(),
        })
    }
}

#[derive(Default)]
struct StepBuilder {
    touches: Vec<Touch>,
    joins: Vec<Join>,
}

impl StepBuilder {
    /// Record that `table` is touched. A table touched twice keeps the stronger
    /// access — an `UPDATE` that also filters on its own table writes it.
    fn touch(&mut self, table: &TableRef, access: Access) {
        match self.touches.iter_mut().find(|t| &t.table == table) {
            Some(existing) => existing.access = existing.access.max(access),
            None => self.touches.push(Touch {
                table: table.clone(),
                access,
                columns: Vec::new(),
                values: Vec::new(),
            }),
        }
    }

    fn touch_column(&mut self, column: &ColumnRef, access: Access) {
        self.touch(&column.table, access);
        let touch = self
            .touches
            .iter_mut()
            .find(|t| t.table == column.table)
            .expect("touched above");
        if !touch.columns.contains(&column.column) {
            touch.columns.push(column.column.clone());
        }
    }

    /// Record a value the statement names for `column`, touching it with
    /// `access` if nothing has yet.
    fn value(&mut self, column: &ColumnRef, access: Access, kind: ValueKind, text: String) {
        self.touch_column(column, access);
        let touch = self
            .touches
            .iter_mut()
            .find(|t| t.table == column.table)
            .expect("touched above");
        let value = ColumnValue {
            column: column.column.clone(),
            kind,
            text,
        };
        if !touch.values.contains(&value) {
            touch.values.push(value);
        }
    }

    /// The conditions in `expr` that hold a column to something that is not
    /// another column, following `AND` and nothing else.
    fn filters(&mut self, expr: &Expr, scope: &Scope) {
        let mut found = |column: &Expr, text: String| {
            if let Some(column) = column_of(column, scope) {
                self.value(&column, Access::Read, ValueKind::Filter, text);
            }
        };
        match expr {
            Expr::BinaryOp {
                left,
                op: BinaryOperator::And,
                right,
            } => {
                self.filters(left, scope);
                self.filters(right, scope);
            }
            Expr::Nested(inner) => self.filters(inner, scope),
            Expr::BinaryOp { left, op, right } => {
                match (is_column(left), is_column(right)) {
                    (true, false) => found(left, format!("{op} {right}")),
                    // `100 = id` reads as `id = 100`, with the comparison
                    // turned round to match.
                    (false, true) => {
                        if let Some(op) = mirrored(op) {
                            found(right, format!("{op} {left}"));
                        }
                    }
                    _ => {}
                }
            }
            Expr::InList {
                expr,
                list,
                negated,
            } => {
                let list: Vec<String> = list.iter().map(ToString::to_string).collect();
                let not = if *negated { "NOT " } else { "" };
                found(expr, format!("{not}IN ({})", list.join(", ")));
            }
            Expr::IsNull(expr) => found(expr, "IS NULL".to_owned()),
            Expr::IsNotNull(expr) => found(expr, "IS NOT NULL".to_owned()),
            Expr::Between {
                expr,
                negated,
                low,
                high,
            } => {
                let not = if *negated { "NOT " } else { "" };
                found(expr, format!("{not}BETWEEN {low} AND {high}"));
            }
            Expr::Like {
                expr,
                negated,
                pattern,
                ..
            } => {
                let not = if *negated { "NOT " } else { "" };
                found(expr, format!("{not}LIKE {pattern}"));
            }
            Expr::ILike {
                expr,
                negated,
                pattern,
                ..
            } => {
                let not = if *negated { "NOT " } else { "" };
                found(expr, format!("{not}ILIKE {pattern}"));
            }
            _ => {}
        }
    }

    fn query(&mut self, query: &Query, outer: &Scope) {
        let mut scope = outer.clone();
        if let Some(with) = &query.with {
            for cte in &with.cte_tables {
                self.query(&cte.query, &scope);
                scope.ctes.push(cte.alias.name.value.clone());
            }
        }
        self.set_expr(&query.body, &scope);
    }

    fn set_expr(&mut self, body: &SetExpr, scope: &Scope) {
        match body {
            SetExpr::Select(select) => self.select(select, scope),
            SetExpr::Query(query) => self.query(query, scope),
            SetExpr::SetOperation { left, right, .. } => {
                self.set_expr(left, scope);
                self.set_expr(right, scope);
            }
            // VALUES, TABLE and the rest name no columns of a known table.
            _ => {}
        }
    }

    fn select(&mut self, select: &Select, outer: &Scope) {
        // Columns of the outer query stay visible, for correlated subqueries,
        // but an unqualified column means the innermost table.
        let mut scope = Scope {
            tables: Vec::new(),
            ..outer.clone()
        };
        self.from(&select.from, &mut scope, Access::Read);

        for item in &select.projection {
            match item {
                SelectItem::UnnamedExpr(expr)
                | SelectItem::ExprWithAlias { expr, .. }
                | SelectItem::ExprWithAliases { expr, .. } => self.expr(expr, &scope, Access::Read),
                SelectItem::QualifiedWildcard(
                    SelectItemQualifiedWildcardKind::ObjectName(name),
                    _,
                ) => {
                    if let Some(table) = scope.names.get(&split_object_name(name).1).cloned() {
                        self.touch(&table, Access::Read);
                    }
                }
                _ => {}
            }
        }
        if let Some(selection) = &select.selection {
            self.expr(selection, &scope, Access::Read);
            self.filters(selection, &scope);
        }
        if let Some(having) = &select.having {
            self.expr(having, &scope, Access::Read);
            self.filters(having, &scope);
        }
    }

    /// Bring the tables of a `FROM` (or `USING`, or `UPDATE … FROM`) into
    /// scope, touching them with `access`, and read their join conditions.
    fn from(&mut self, from: &[TableWithJoins], scope: &mut Scope, access: Access) {
        for item in from {
            self.relation(&item.relation, scope, access);
            for join in &item.joins {
                self.relation(&join.relation, scope, Access::Read);
                if let Some(JoinConstraint::On(on)) = join_constraint(&join.join_operator) {
                    self.expr(on, scope, Access::Read);
                    self.equalities(on, scope);
                    self.filters(on, scope);
                }
            }
        }
    }

    fn relation(&mut self, relation: &TableFactor, scope: &mut Scope, access: Access) {
        match relation {
            TableFactor::Table { name, alias, .. } => {
                let (schema, table) = split_object_name(name);
                let alias = alias.as_ref().map(|a| a.name.value.as_str());
                if schema.is_none() && scope.ctes.contains(&table) {
                    // A CTE's own reads were recorded where it was defined.
                    return;
                }
                let table = TableRef {
                    schema,
                    name: table,
                };
                scope.bind(&table, alias);
                self.touch(&table, access);
            }
            TableFactor::Derived { subquery, .. } => self.query(subquery, scope),
            TableFactor::NestedJoin {
                table_with_joins, ..
            } => self.from(std::slice::from_ref(table_with_joins), scope, access),
            _ => {}
        }
    }

    /// Every column `expr` names is read (or `access`); subqueries in it are
    /// followed with the current scope as their outer one.
    fn expr(&mut self, expr: &Expr, scope: &Scope, access: Access) {
        match expr {
            Expr::Identifier(ident) => {
                if let Some(column) = scope.column(std::slice::from_ref(&ident.value)) {
                    self.touch_column(&column, access);
                }
            }
            Expr::CompoundIdentifier(parts) => {
                let parts: Vec<String> = parts.iter().map(|p| p.value.clone()).collect();
                if let Some(column) = scope.column(&parts) {
                    self.touch_column(&column, access);
                }
            }
            Expr::BinaryOp { left, right, .. } => {
                self.expr(left, scope, access);
                self.expr(right, scope, access);
            }
            Expr::Nested(inner)
            | Expr::UnaryOp { expr: inner, .. }
            | Expr::IsNull(inner)
            | Expr::IsNotNull(inner) => self.expr(inner, scope, access),
            Expr::InList { expr, list, .. } => {
                self.expr(expr, scope, access);
                for item in list {
                    self.expr(item, scope, access);
                }
            }
            Expr::Between {
                expr, low, high, ..
            } => {
                self.expr(expr, scope, access);
                self.expr(low, scope, access);
                self.expr(high, scope, access);
            }
            Expr::Like { expr, .. } | Expr::ILike { expr, .. } => self.expr(expr, scope, access),
            Expr::InSubquery { expr, subquery, .. } => {
                self.expr(expr, scope, access);
                self.query(subquery, scope);
            }
            Expr::Exists { subquery, .. } | Expr::Subquery(subquery) => {
                self.query(subquery, scope);
            }
            // Literals, functions, casts: nothing a diagram can point at, or
            // not worth following for it.
            _ => {}
        }
    }

    /// `a.x = b.y` conditions in an `ON` clause, joined by `AND`.
    fn equalities(&mut self, expr: &Expr, scope: &Scope) {
        match expr {
            Expr::BinaryOp {
                left,
                op: BinaryOperator::And,
                right,
            } => {
                self.equalities(left, scope);
                self.equalities(right, scope);
            }
            Expr::BinaryOp {
                left,
                op: BinaryOperator::Eq,
                right,
            } => {
                if let (Some(left), Some(right)) = (column_of(left, scope), column_of(right, scope))
                    && left.table != right.table
                {
                    self.joins.push(Join { left, right });
                }
            }
            Expr::Nested(inner) => self.equalities(inner, scope),
            _ => {}
        }
    }

    fn insert(&mut self, insert: &Insert) {
        let TableObject::TableName(name) = &insert.table else {
            return;
        };
        let (schema, table) = split_object_name(name);
        let table = TableRef {
            schema,
            name: table,
        };
        self.touch(&table, Access::Write);
        let columns: Vec<ColumnRef> = insert
            .columns
            .iter()
            .filter_map(|column| name_parts(column).pop())
            .map(|column| ColumnRef {
                table: table.clone(),
                column,
            })
            .collect();
        for column in &columns {
            self.touch_column(column, Access::Write);
        }
        let Some(source) = &insert.source else {
            return;
        };
        // What goes into each column, by position: one row of `VALUES` per
        // value, or the expression a `SELECT` projects into it.
        let inserted: Vec<Vec<String>> = match source.body.as_ref() {
            SetExpr::Values(values) => values
                .rows
                .iter()
                .map(|row| row.iter().map(ToString::to_string).collect())
                .collect(),
            SetExpr::Select(select) => {
                let projected = select.projection.iter().map(|item| match item {
                    SelectItem::UnnamedExpr(expr)
                    | SelectItem::ExprWithAlias { expr, .. }
                    | SelectItem::ExprWithAliases { expr, .. } => Some(expr.to_string()),
                    _ => None,
                });
                // A `*` leaves the positions unknown, so nothing is paired.
                projected.collect::<Option<Vec<_>>>().into_iter().collect()
            }
            _ => Vec::new(),
        };
        for (i, column) in columns.iter().enumerate() {
            let row_values: Vec<&str> = inserted
                .iter()
                .filter_map(|row| row.get(i).map(String::as_str))
                .collect();
            if let Some(text) = rows(&row_values) {
                self.value(column, Access::Write, ValueKind::Set, text);
            }
        }
        // `INSERT … SELECT` reads its source; `VALUES` reads nothing.
        self.query(source, &Scope::default());
    }

    fn update(&mut self, update: &Update) {
        let mut scope = Scope::default();
        self.from(
            std::slice::from_ref(&update.table),
            &mut scope,
            Access::Write,
        );
        if let Some(UpdateTableFromKind::BeforeSet(from) | UpdateTableFromKind::AfterSet(from)) =
            &update.from
        {
            self.from(from, &mut scope, Access::Read);
        }

        // SET targets belong to the updated table even when the statement
        // names others, so they are attributed to it directly.
        let target = scope.tables.first().cloned();
        for assignment in &update.assignments {
            let names = match &assignment.target {
                AssignmentTarget::ColumnName(name) => vec![name],
                AssignmentTarget::Tuple(names) => names.iter().collect(),
            };
            for name in names {
                let parts = name_parts(name);
                let column = match (parts.as_slice(), &target) {
                    ([column], Some(table)) => Some(ColumnRef {
                        table: table.clone(),
                        column: column.clone(),
                    }),
                    _ => scope.column(&parts),
                };
                if let Some(column) = column {
                    self.touch_column(&column, Access::Write);
                    // A tuple target pairs with a tuple or a subquery, which
                    // has no one value per column to show.
                    if matches!(assignment.target, AssignmentTarget::ColumnName(_)) {
                        self.value(
                            &column,
                            Access::Write,
                            ValueKind::Set,
                            assignment.value.to_string(),
                        );
                    }
                }
            }
            self.expr(&assignment.value, &scope, Access::Read);
        }
        if let Some(selection) = &update.selection {
            self.expr(selection, &scope, Access::Read);
            self.filters(selection, &scope);
        }
    }

    fn delete(&mut self, delete: &Delete) {
        let mut scope = Scope::default();
        let (FromTable::WithFromKeyword(from) | FromTable::WithoutKeyword(from)) = &delete.from;
        // MySQL's multi-table `DELETE t1 FROM t1 JOIN t2` names what it deletes
        // separately from what it reads.
        if delete.tables.is_empty() {
            self.from(from, &mut scope, Access::Delete);
        } else {
            self.from(from, &mut scope, Access::Read);
            for name in &delete.tables {
                let (_, table) = split_object_name(name);
                if let Some(table) = scope.names.get(&table).cloned() {
                    self.touch(&table, Access::Delete);
                }
            }
        }
        if let Some(using) = &delete.using {
            self.from(using, &mut scope, Access::Read);
        }
        if let Some(selection) = &delete.selection {
            self.expr(selection, &scope, Access::Read);
            self.filters(selection, &scope);
        }
    }
}

fn column_of(expr: &Expr, scope: &Scope) -> Option<ColumnRef> {
    match expr {
        Expr::Identifier(ident) => scope.column(std::slice::from_ref(&ident.value)),
        Expr::CompoundIdentifier(parts) => {
            let parts: Vec<String> = parts.iter().map(|p| p.value.clone()).collect();
            scope.column(&parts)
        }
        Expr::Nested(inner) => column_of(inner, scope),
        _ => None,
    }
}

/// True for a column reference, whether or not the scope can say whose. A
/// condition between two columns is a join, not a value.
fn is_column(expr: &Expr) -> bool {
    match expr {
        Expr::Identifier(_) | Expr::CompoundIdentifier(_) => true,
        Expr::Nested(inner) => is_column(inner),
        _ => false,
    }
}

/// The comparison that means the same with its two sides swapped, for the
/// operators where there is one.
fn mirrored(op: &BinaryOperator) -> Option<BinaryOperator> {
    Some(match op {
        BinaryOperator::Eq => BinaryOperator::Eq,
        BinaryOperator::NotEq => BinaryOperator::NotEq,
        BinaryOperator::Lt => BinaryOperator::Gt,
        BinaryOperator::Gt => BinaryOperator::Lt,
        BinaryOperator::LtEq => BinaryOperator::GtEq,
        BinaryOperator::GtEq => BinaryOperator::LtEq,
        _ => return None,
    })
}

/// The values a multi-row insert puts in one column, as one string: each row
/// separated by ` | `, and past three rows only the first three and `…`.
fn rows(values: &[&str]) -> Option<String> {
    const SHOWN: usize = 3;
    match values {
        [] => None,
        values if values.len() > SHOWN => Some(format!("{} | …", values[..SHOWN].join(" | "))),
        values => Some(values.join(" | ")),
    }
}

fn join_constraint(operator: &JoinOperator) -> Option<&JoinConstraint> {
    match operator {
        JoinOperator::Join(c)
        | JoinOperator::Inner(c)
        | JoinOperator::Left(c)
        | JoinOperator::LeftOuter(c)
        | JoinOperator::Right(c)
        | JoinOperator::RightOuter(c)
        | JoinOperator::FullOuter(c)
        | JoinOperator::CrossJoin(c)
        | JoinOperator::Semi(c)
        | JoinOperator::LeftSemi(c)
        | JoinOperator::RightSemi(c)
        | JoinOperator::Anti(c)
        | JoinOperator::LeftAnti(c)
        | JoinOperator::RightAnti(c)
        | JoinOperator::StraightJoin(c)
        | JoinOperator::AsOf { constraint: c, .. } => Some(c),
        _ => None,
    }
}

/// Enough of a statement to recognise it in a warning.
fn first_words(sql: &str) -> String {
    let words: Vec<&str> = sql.split_whitespace().take(4).collect();
    let mut out = words.join(" ");
    if sql.split_whitespace().count() > 4 {
        out.push_str(" …");
    }
    out
}
