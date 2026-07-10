use crate::{Expr, TypedExpr};
use std::fmt;

/// Deterministic structural lower-bound cost hooks for search packages.
#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub struct Cost {
    pub description_units: u64,
    pub literal_constants: u64,
    pub maximum_execution_steps: u64,
}

pub trait CostModel {
    /// Measure structural cost without hiding arithmetic overflow.
    ///
    /// # Errors
    ///
    /// Returns [`CostError::Overflow`] for an adversarially large bound.
    fn cost(&self, expression: &TypedExpr) -> Result<Cost, CostError>;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct StructuralCostModel;

impl CostModel for StructuralCostModel {
    fn cost(&self, expression: &TypedExpr) -> Result<Cost, CostError> {
        cost_expr(expression.expression())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CostError {
    Overflow,
}

impl fmt::Display for CostError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("RKL structural cost overflow")
    }
}

impl std::error::Error for CostError {}

fn cost_expr(expression: &Expr) -> Result<Cost, CostError> {
    let mut cost = Cost {
        description_units: 1,
        literal_constants: u64::from(matches!(
            expression,
            Expr::Int(_) | Expr::Atom(_) | Expr::Coord2 { .. }
        )),
        maximum_execution_steps: 1,
    };
    match expression {
        Expr::Tuple(values)
        | Expr::Call {
            arguments: values, ..
        } => {
            for value in values {
                add_cost(&mut cost, cost_expr(value)?)?;
            }
        }
        Expr::Let { value, body, .. } => {
            add_cost(&mut cost, cost_expr(value)?)?;
            add_cost(&mut cost, cost_expr(body)?)?;
        }
        Expr::If {
            condition,
            then_branch,
            else_branch,
        } => {
            add_cost(&mut cost, cost_expr(condition)?)?;
            add_cost(&mut cost, cost_expr(then_branch)?)?;
            add_cost(&mut cost, cost_expr(else_branch)?)?;
        }
        Expr::Unary { argument, .. } | Expr::Count(argument) => {
            add_cost(&mut cost, cost_expr(argument)?)?;
        }
        Expr::Binary { left, right, .. } => {
            add_cost(&mut cost, cost_expr(left)?)?;
            add_cost(&mut cost, cost_expr(right)?)?;
        }
        Expr::SingletonSet { value, .. } => add_cost(&mut cost, cost_expr(value)?)?,
        Expr::BoundedFixpoint {
            initial,
            step,
            maximum_steps,
            progress_measure,
            ..
        } => {
            add_cost(&mut cost, cost_expr(initial)?)?;
            let step_cost = cost_expr(step)?;
            add_cost(&mut cost, cost_expr(progress_measure)?)?;
            cost.description_units = cost
                .description_units
                .checked_add(step_cost.description_units)
                .ok_or(CostError::Overflow)?;
            cost.literal_constants = cost
                .literal_constants
                .checked_add(step_cost.literal_constants)
                .ok_or(CostError::Overflow)?;
            let repeated = step_cost
                .maximum_execution_steps
                .checked_mul(u64::from(*maximum_steps))
                .ok_or(CostError::Overflow)?;
            cost.maximum_execution_steps = cost
                .maximum_execution_steps
                .checked_add(repeated)
                .ok_or(CostError::Overflow)?;
        }
        Expr::Bool(_)
        | Expr::Int(_)
        | Expr::Atom(_)
        | Expr::Coord2 { .. }
        | Expr::Var(_)
        | Expr::EmptySet { .. } => {}
    }
    Ok(cost)
}

fn add_cost(target: &mut Cost, value: Cost) -> Result<(), CostError> {
    target.description_units = target
        .description_units
        .checked_add(value.description_units)
        .ok_or(CostError::Overflow)?;
    target.literal_constants = target
        .literal_constants
        .checked_add(value.literal_constants)
        .ok_or(CostError::Overflow)?;
    target.maximum_execution_steps = target
        .maximum_execution_steps
        .checked_add(value.maximum_execution_steps)
        .ok_or(CostError::Overflow)?;
    Ok(())
}
