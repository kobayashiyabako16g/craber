//! Walking `SELECT`, `INSERT`, `UPDATE` and `DELETE`.

use craber_core::{Access, ColumnRef, ValueKind};
use sqlparser::ast::{
    AssignmentTarget, Delete, Expr, FromTable, FunctionArg, FunctionArgExpr, FunctionArguments,
    GroupByExpr, Insert, JoinConstraint, JoinOperator, OrderByKind, Query, Select, SelectItem,
    SelectItemQualifiedWildcardKind, SetExpr, TableFactor, TableObject, TableWithJoins, Update,
    UpdateTableFromKind,
};

use super::builder::StepBuilder;
use super::scope::Scope;
use crate::names::{name_parts, object_ref};

impl StepBuilder {
    pub(super) fn query(&mut self, query: &Query, outer: &Scope) {
        let mut scope = outer.clone();
        if let Some(with) = &query.with {
            for cte in &with.cte_tables {
                self.query(&cte.query, &scope);
                scope.ctes.push(cte.alias.name.value.clone());
            }
        }
        // `ORDER BY` names the columns of the `SELECT` it ends, so it is read
        // in that `SELECT`'s scope. After a `UNION` it names output columns,
        // which belong to no one table.
        let order: Vec<&Expr> = match (&query.order_by, query.body.as_ref()) {
            (Some(order_by), SetExpr::Select(_)) => match &order_by.kind {
                OrderByKind::Expressions(items) => items.iter().map(|item| &item.expr).collect(),
                OrderByKind::All(_) => Vec::new(),
            },
            _ => Vec::new(),
        };
        match query.body.as_ref() {
            SetExpr::Select(select) => self.select(select, &scope, &order),
            body => self.set_expr(body, &scope),
        }
    }

    fn set_expr(&mut self, body: &SetExpr, scope: &Scope) {
        match body {
            SetExpr::Select(select) => self.select(select, scope, &[]),
            SetExpr::Query(query) => self.query(query, scope),
            SetExpr::SetOperation { left, right, .. } => {
                self.set_expr(left, scope);
                self.set_expr(right, scope);
            }
            // VALUES, TABLE and the rest name no columns of a known table.
            _ => {}
        }
    }

    /// `order` is the `ORDER BY` of the query this `SELECT` is the body of.
    fn select(&mut self, select: &Select, outer: &Scope, order: &[&Expr]) {
        let mut scope = outer.nested();
        self.from(&select.from, &mut scope, Access::Read);

        for item in &select.projection {
            if let Some(expr) = projected(item) {
                self.expr(expr, &scope, Access::Read);
            } else if let SelectItem::QualifiedWildcard(
                SelectItemQualifiedWildcardKind::ObjectName(name),
                _,
            ) = item
                && let Some(table) = scope.table(name)
            {
                self.touch(&table, Access::Read);
            }
        }
        if let Some(selection) = &select.selection {
            self.condition(selection, &scope);
        }
        if let GroupByExpr::Expressions(exprs, _) = &select.group_by {
            for expr in exprs {
                self.expr(expr, &scope, Access::Read);
            }
        }
        if let Some(having) = &select.having {
            self.condition(having, &scope);
        }
        for expr in order {
            self.expr(expr, &scope, Access::Read);
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
                    self.join_condition(on, scope);
                }
            }
        }
    }

    fn relation(&mut self, relation: &TableFactor, scope: &mut Scope, access: Access) {
        match relation {
            TableFactor::Table { name, alias, .. } => {
                let table = object_ref(name);
                let alias = alias.as_ref().map(|a| a.name.value.as_str());
                if table.schema.is_none() && scope.ctes.contains(&table.name) {
                    // A CTE's own reads were recorded where it was defined.
                    return;
                }
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
    pub(super) fn expr(&mut self, expr: &Expr, scope: &Scope, access: Access) {
        let mut each = |exprs: &mut dyn Iterator<Item = &Expr>| {
            for expr in exprs {
                self.expr(expr, scope, access);
            }
        };
        match expr {
            Expr::Identifier(_) | Expr::CompoundIdentifier(_) => {
                if let Some(column) = scope.column_of(expr) {
                    self.touch_column(&column, access);
                }
            }
            Expr::BinaryOp { left, right, .. } => each(&mut [&**left, &**right].into_iter()),
            Expr::Nested(inner)
            | Expr::UnaryOp { expr: inner, .. }
            | Expr::IsNull(inner)
            | Expr::IsNotNull(inner)
            | Expr::Cast { expr: inner, .. }
            | Expr::Like { expr: inner, .. }
            | Expr::ILike { expr: inner, .. } => self.expr(inner, scope, access),
            Expr::InList { expr, list, .. } => {
                each(&mut std::iter::once(&**expr).chain(list));
            }
            Expr::Between {
                expr, low, high, ..
            } => each(&mut [&**expr, &**low, &**high].into_iter()),
            Expr::Case {
                operand,
                conditions,
                else_result,
                ..
            } => each(
                &mut operand
                    .as_deref()
                    .into_iter()
                    .chain(conditions.iter().flat_map(|w| [&w.condition, &w.result]))
                    .chain(else_result.as_deref()),
            ),
            Expr::Function(function) => match &function.args {
                FunctionArguments::List(list) => {
                    each(&mut list.args.iter().filter_map(argument));
                }
                FunctionArguments::Subquery(subquery) => self.query(subquery, scope),
                FunctionArguments::None => {}
            },
            Expr::InSubquery { expr, subquery, .. } => {
                self.expr(expr, scope, access);
                self.query(subquery, scope);
            }
            Expr::Exists { subquery, .. } | Expr::Subquery(subquery) => {
                self.query(subquery, scope);
            }
            // Literals and the rest: nothing a diagram can point at.
            _ => {}
        }
    }

    pub(super) fn insert(&mut self, insert: &Insert) {
        let TableObject::TableName(name) = &insert.table else {
            return;
        };
        let table = object_ref(name);
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
                let projected = select
                    .projection
                    .iter()
                    .map(|item| projected(item).map(ToString::to_string));
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

    pub(super) fn update(&mut self, update: &Update) {
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
                let Some(column) = column else {
                    continue;
                };
                // A tuple target pairs with a tuple or a subquery, which has no
                // one value per column to show.
                if matches!(assignment.target, AssignmentTarget::ColumnName(_)) {
                    let text = assignment.value.to_string();
                    self.value(&column, Access::Write, ValueKind::Set, text);
                } else {
                    self.touch_column(&column, Access::Write);
                }
            }
            self.expr(&assignment.value, &scope, Access::Read);
        }
        if let Some(selection) = &update.selection {
            self.condition(selection, &scope);
        }
    }

    pub(super) fn delete(&mut self, delete: &Delete) {
        let mut scope = Scope::default();
        let (FromTable::WithFromKeyword(from) | FromTable::WithoutKeyword(from)) = &delete.from;
        // MySQL's multi-table `DELETE t1 FROM t1 JOIN t2` names what it deletes
        // separately from what it reads.
        if delete.tables.is_empty() {
            self.from(from, &mut scope, Access::Delete);
        } else {
            self.from(from, &mut scope, Access::Read);
            for name in &delete.tables {
                if let Some(table) = scope.table(name) {
                    self.touch(&table, Access::Delete);
                }
            }
        }
        if let Some(using) = &delete.using {
            self.from(using, &mut scope, Access::Read);
        }
        if let Some(selection) = &delete.selection {
            self.condition(selection, &scope);
        }
    }
}

/// The expression a projection item reads, or `None` for a wildcard.
fn projected(item: &SelectItem) -> Option<&Expr> {
    match item {
        SelectItem::UnnamedExpr(expr)
        | SelectItem::ExprWithAlias { expr, .. }
        | SelectItem::ExprWithAliases { expr, .. } => Some(expr),
        _ => None,
    }
}

/// The expression a function argument passes, named or not.
fn argument(arg: &FunctionArg) -> Option<&Expr> {
    let (FunctionArg::Named { arg, .. }
    | FunctionArg::ExprNamed { arg, .. }
    | FunctionArg::Unnamed(arg)) = arg;
    match arg {
        FunctionArgExpr::Expr(expr) => Some(expr),
        _ => None,
    }
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
