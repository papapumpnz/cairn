//! Environment-neutral induction of theory components from binding evidence.

use cairn_invent::{DefinitionInventor, InventionError, InventionOutcome, InventionProblem};
use cairn_rkl::{BinaryOp, Expr, Schedule, Value};
use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq)]
pub struct PredicateColumn {
    pub expression: Expr,
    pub true_rows: BTreeSet<u32>,
    pub structural_cost: u16,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BindingTable {
    pub positive_rows: BTreeSet<u32>,
    pub negative_rows: BTreeSet<u32>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GuardHypothesis {
    pub expression: Expr,
    pub predicates: Vec<usize>,
    pub structural_cost: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GuardLimits {
    pub maximum_conjuncts: usize,
    pub maximum_candidates: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GuardFailure {
    EmptyCapacity,
    InconsistentRows,
    Unrealizable,
    CandidateCapacity { maximum: u64 },
}

#[derive(Clone, Copy, Debug, Default)]
pub struct GuardInducer;

impl GuardInducer {
    /// Find a minimum-cost conjunction separating all positive and negative
    /// bindings. Predicate meanings remain supplied as generic RKL.
    ///
    /// # Errors
    ///
    /// Distinguishes malformed, capped, and unrealizable separation.
    pub fn induce(
        &self,
        table: &BindingTable,
        predicates: &[PredicateColumn],
        limits: GuardLimits,
    ) -> Result<GuardHypothesis, GuardFailure> {
        if limits.maximum_conjuncts == 0 || limits.maximum_candidates == 0 {
            return Err(GuardFailure::EmptyCapacity);
        }
        if !table.positive_rows.is_disjoint(&table.negative_rows) {
            return Err(GuardFailure::InconsistentRows);
        }
        let mut candidates = predicates
            .iter()
            .enumerate()
            .filter(|(_, predicate)| table.positive_rows.is_subset(&predicate.true_rows))
            .collect::<Vec<_>>();
        candidates.sort_by_key(|(index, predicate)| (predicate.structural_cost, *index));
        let mut evaluated = 0_u64;
        for width in 1..=limits.maximum_conjuncts.min(candidates.len()) {
            let mut selection = Vec::new();
            if let Some(indices) = search_conjunction(
                &candidates,
                &table.negative_rows,
                width,
                0,
                &mut selection,
                &mut evaluated,
                limits.maximum_candidates,
            )? {
                let expression = indices.iter().skip(1).fold(
                    predicates[indices[0]].expression.clone(),
                    |left, index| Expr::Binary {
                        op: BinaryOp::And,
                        left: Box::new(left),
                        right: Box::new(predicates[*index].expression.clone()),
                    },
                );
                let structural_cost = indices.iter().fold(0_u16, |cost, index| {
                    cost.saturating_add(predicates[*index].structural_cost)
                });
                return Ok(GuardHypothesis {
                    expression,
                    predicates: indices,
                    structural_cost,
                });
            }
        }
        Err(GuardFailure::Unrealizable)
    }
}

#[allow(clippy::too_many_arguments)]
fn search_conjunction(
    candidates: &[(usize, &PredicateColumn)],
    negatives: &BTreeSet<u32>,
    width: usize,
    start: usize,
    selection: &mut Vec<usize>,
    evaluated: &mut u64,
    maximum: u64,
) -> Result<Option<Vec<usize>>, GuardFailure> {
    if selection.len() == width {
        *evaluated = evaluated.saturating_add(1);
        if *evaluated > maximum {
            return Err(GuardFailure::CandidateCapacity { maximum });
        }
        let survivors = selection.iter().fold(negatives.clone(), |rows, candidate| {
            rows.intersection(&candidates[*candidate].1.true_rows)
                .copied()
                .collect()
        });
        return Ok(survivors.is_empty().then(|| {
            selection
                .iter()
                .map(|candidate| candidates[*candidate].0)
                .collect()
        }));
    }
    for candidate in start..candidates.len() {
        selection.push(candidate);
        if let Some(result) = search_conjunction(
            candidates,
            negatives,
            width,
            candidate + 1,
            selection,
            evaluated,
            maximum,
        )? {
            return Ok(Some(result));
        }
        selection.pop();
    }
    Ok(None)
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ConstructorSynthesizer;

impl ConstructorSynthesizer {
    /// Synthesize a checked variable-valued constructor from demonstrations.
    ///
    /// # Errors
    ///
    /// Returns the exact bounded invention failure.
    pub fn synthesize(
        &self,
        problem: &InventionProblem,
    ) -> Result<InventionOutcome, InventionError> {
        DefinitionInventor.invent(problem)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuleFootprint {
    pub name: String,
    pub reads: BTreeSet<Value>,
    pub writes: BTreeSet<Value>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ScheduleFailure {
    DuplicateRule(String),
    ConflictingWrites { left: String, right: String },
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ScheduleSynthesizer;

impl ScheduleSynthesizer {
    /// Infer parallel execution or a dependency DAG from generic read/write
    /// footprints; ambiguous write conflicts fail instead of choosing order.
    ///
    /// # Errors
    ///
    /// Rejects duplicate rules and unexplained write conflicts.
    pub fn synthesize(&self, rules: &[RuleFootprint]) -> Result<Schedule, ScheduleFailure> {
        let mut names = BTreeSet::new();
        for rule in rules {
            if !names.insert(rule.name.clone()) {
                return Err(ScheduleFailure::DuplicateRule(rule.name.clone()));
            }
        }
        let mut edges = BTreeSet::new();
        for (left_index, left) in rules.iter().enumerate() {
            for right in rules.iter().skip(left_index + 1) {
                if !left.writes.is_disjoint(&right.writes) {
                    return Err(ScheduleFailure::ConflictingWrites {
                        left: left.name.clone(),
                        right: right.name.clone(),
                    });
                }
                if !left.writes.is_disjoint(&right.reads) {
                    edges.insert((left.name.clone(), right.name.clone()));
                }
                if !right.writes.is_disjoint(&left.reads) {
                    edges.insert((right.name.clone(), left.name.clone()));
                }
            }
        }
        if edges.is_empty() {
            Ok(Schedule::Parallel)
        } else {
            Ok(Schedule::DependencyDag(edges.into_iter().collect()))
        }
    }
}
