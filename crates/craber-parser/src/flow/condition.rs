//! What a `WHERE`, `HAVING` or `ON` says about a column.

use craber_core::{Access, Join, ValueKind};
use sqlparser::ast::{BinaryOperator, Expr};

use super::builder::StepBuilder;
use super::scope::{Scope, is_column};

impl StepBuilder {
    /// A `WHERE` or `HAVING`: every column it names is read, and what it holds
    /// one to is a filter.
    pub(super) fn condition(&mut self, expr: &Expr, scope: &Scope) {
        self.expr(expr, scope, Access::Read);
        self.filters(expr, scope);
    }

    /// An `ON`: a condition, plus the `a.x = b.y` pairs that join two tables.
    pub(super) fn join_condition(&mut self, expr: &Expr, scope: &Scope) {
        self.condition(expr, scope);
        for conjunct in conjuncts(expr) {
            if let Expr::BinaryOp {
                left,
                op: BinaryOperator::Eq,
                right,
            } = conjunct
                && let (Some(left), Some(right)) = (scope.column_of(left), scope.column_of(right))
                && left.table != right.table
            {
                self.joins.push(Join { left, right });
            }
        }
    }

    /// The conditions in `expr` that hold a column to something that is not
    /// another column, following `AND` and nothing else.
    fn filters(&mut self, expr: &Expr, scope: &Scope) {
        for conjunct in conjuncts(expr) {
            let Some((column, text)) = filter(conjunct) else {
                continue;
            };
            if let Some(column) = scope.column_of(column) {
                self.value(&column, Access::Read, ValueKind::Filter, text);
            }
        }
    }
}

/// The parts of `expr` a row is known to satisfy: the operands of every `AND`,
/// through any parentheses. Anything else — an `OR` included — is one part.
fn conjuncts(expr: &Expr) -> Vec<&Expr> {
    match expr {
        Expr::BinaryOp {
            left,
            op: BinaryOperator::And,
            right,
        } => {
            let mut parts = conjuncts(left);
            parts.extend(conjuncts(right));
            parts
        }
        Expr::Nested(inner) => conjuncts(inner),
        other => vec![other],
    }
}

/// The column one condition holds to something, and what, as the SQL writes
/// it.
fn filter(expr: &Expr) -> Option<(&Expr, String)> {
    let not = |negated: bool| if negated { "NOT " } else { "" };
    Some(match expr {
        Expr::BinaryOp { left, op, right } => match (is_column(left), is_column(right)) {
            (true, false) => (&**left, format!("{op} {right}")),
            // `100 = id` reads as `id = 100`, with the comparison turned round
            // to match.
            (false, true) => (&**right, format!("{} {left}", mirrored(op)?)),
            _ => return None,
        },
        Expr::InList {
            expr,
            list,
            negated,
        } => {
            let list: Vec<String> = list.iter().map(ToString::to_string).collect();
            (
                &**expr,
                format!("{}IN ({})", not(*negated), list.join(", ")),
            )
        }
        Expr::IsNull(expr) => (&**expr, "IS NULL".to_owned()),
        Expr::IsNotNull(expr) => (&**expr, "IS NOT NULL".to_owned()),
        Expr::Between {
            expr,
            negated,
            low,
            high,
        } => (
            &**expr,
            format!("{}BETWEEN {low} AND {high}", not(*negated)),
        ),
        Expr::Like {
            expr,
            negated,
            pattern,
            ..
        } => (&**expr, format!("{}LIKE {pattern}", not(*negated))),
        Expr::ILike {
            expr,
            negated,
            pattern,
            ..
        } => (&**expr, format!("{}ILIKE {pattern}", not(*negated))),
        _ => return None,
    })
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
