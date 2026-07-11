//! Deterministic developmental curriculum selection from support uncertainty.

use cairn_invent::{InventionError, InventionProblem};
use cairn_rkl::{Type, Value};
use serde::Serialize;
use std::collections::{btree_map::Entry, BTreeMap, BTreeSet};

pub const FAILURE_CORE_SEMANTICS_VERSION: &str = "1.0.0";

/// Redacted structural evidence for generating a new learning curriculum.
/// It deliberately contains no task identifier, observation payload, literal
/// value, expected answer, variable name, or domain label.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct LearningFailureCore {
    pub semantics_version: String,
    pub failure: SearchFailure,
    pub parameter_types: Vec<ValueClass>,
    pub return_type: ValueClass,
    pub demonstrations: usize,
    pub varying_parameters: usize,
    pub distinct_output_classes: usize,
    pub constant_explanation_possible: bool,
    pub direct_projection_candidates: usize,
    pub single_parameter_explanations: usize,
    pub exact_two_parameter_families: BTreeSet<TwoParameterFamily>,
    pub exact_three_parameter_families: BTreeSet<ThreeParameterFamily>,
    pub determinism: DeterminismStatus,
    pub minimum_dependency_arity_lower_bound: usize,
    pub redaction: RedactionStatus,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchFailure {
    CandidateCapacity { maximum: u64 },
    NoDefinition { maximum_cost: u16 },
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DeterminismStatus {
    Deterministic,
    ConflictingCompleteInputs,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RedactionStatus {
    ContentFree,
    ContainsRestrictedContent,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ValueClass {
    Boolean,
    Integer,
    Categorical,
    Coordinate,
    Vector,
    Axis,
    Transform,
    Collection,
    Shape,
    Entity,
    Symbol,
    Action,
    State,
    Field,
    Tuple,
    Optional,
    Relation,
    Operator,
    Unit,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TwoParameterFamily {
    Add,
    Subtract,
    Minimum,
    Maximum,
    ZeroGuardedSelection,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ThreeParameterFamily {
    ZeroGuardedBranch,
    EqualityGuardedBranch,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FailureCoreError {
    EmptyDemonstrations,
    NonSearchFailure,
    MissingBinding,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct FailureCoreExtractor;

impl FailureCoreExtractor {
    /// Summarize a failed executable-definition search without retaining any
    /// task content. The summary is suitable for an external curriculum
    /// proposal but cannot reconstruct an observation or answer.
    ///
    /// # Errors
    ///
    /// Rejects empty/incomplete problems and failures unrelated to bounded
    /// search exhaustion.
    pub fn extract(
        &self,
        problem: &InventionProblem,
        error: &InventionError,
    ) -> Result<LearningFailureCore, FailureCoreError> {
        if problem.demonstrations.is_empty() {
            return Err(FailureCoreError::EmptyDemonstrations);
        }
        let failure = match error {
            InventionError::CandidateCapacity { maximum } => {
                SearchFailure::CandidateCapacity { maximum: *maximum }
            }
            InventionError::NoDefinition { maximum_cost } => SearchFailure::NoDefinition {
                maximum_cost: *maximum_cost,
            },
            _ => return Err(FailureCoreError::NonSearchFailure),
        };
        let outputs = problem
            .demonstrations
            .iter()
            .map(|example| example.output.clone())
            .collect::<BTreeSet<_>>();
        let parameter_evidence = parameter_evidence(problem)?;
        let mut complete = BTreeMap::<Vec<Value>, Value>::new();
        let deterministic = problem.demonstrations.iter().all(|example| {
            let inputs = problem
                .parameters
                .iter()
                .filter_map(|parameter| example.inputs.get(&parameter.name).cloned())
                .collect::<Vec<_>>();
            match complete.entry(inputs) {
                Entry::Vacant(entry) => {
                    entry.insert(example.output.clone());
                    true
                }
                Entry::Occupied(entry) => entry.get() == &example.output,
            }
        });
        let constant = outputs.len() == 1;
        let exact_two_parameter_families = two_parameter_families(problem);
        let exact_three_parameter_families = three_parameter_families(problem);
        let minimum_dependency_arity_lower_bound = if constant {
            0
        } else if parameter_evidence.single_explanations > 0 {
            1
        } else {
            2
        };
        Ok(LearningFailureCore {
            semantics_version: FAILURE_CORE_SEMANTICS_VERSION.to_owned(),
            failure,
            parameter_types: problem
                .parameters
                .iter()
                .map(|parameter| value_class(&parameter.ty))
                .collect(),
            return_type: value_class(&problem.return_type),
            demonstrations: problem.demonstrations.len(),
            varying_parameters: parameter_evidence.varying,
            distinct_output_classes: outputs.len(),
            constant_explanation_possible: constant,
            direct_projection_candidates: parameter_evidence.direct_projections,
            single_parameter_explanations: parameter_evidence.single_explanations,
            exact_two_parameter_families,
            exact_three_parameter_families,
            determinism: if deterministic {
                DeterminismStatus::Deterministic
            } else {
                DeterminismStatus::ConflictingCompleteInputs
            },
            minimum_dependency_arity_lower_bound,
            redaction: RedactionStatus::ContentFree,
        })
    }
}

struct ParameterEvidence {
    varying: usize,
    direct_projections: usize,
    single_explanations: usize,
}

fn parameter_evidence(problem: &InventionProblem) -> Result<ParameterEvidence, FailureCoreError> {
    let mut evidence = ParameterEvidence {
        varying: 0,
        direct_projections: 0,
        single_explanations: 0,
    };
    for parameter in &problem.parameters {
        let bindings = problem
            .demonstrations
            .iter()
            .map(|example| {
                example
                    .inputs
                    .get(&parameter.name)
                    .cloned()
                    .map(|value| (value, example.output.clone()))
                    .ok_or(FailureCoreError::MissingBinding)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let values = bindings
            .iter()
            .map(|(value, _)| value.clone())
            .collect::<BTreeSet<_>>();
        evidence.varying += usize::from(values.len() > 1);
        evidence.direct_projections +=
            usize::from(bindings.iter().all(|(value, output)| value == output));
        let mut mapping = BTreeMap::<Value, Value>::new();
        let explains = bindings
            .iter()
            .all(|(value, output)| match mapping.entry(value.clone()) {
                Entry::Vacant(entry) => {
                    entry.insert(output.clone());
                    true
                }
                Entry::Occupied(entry) => entry.get() == output,
            });
        evidence.single_explanations += usize::from(explains);
    }
    Ok(evidence)
}

fn two_parameter_families(problem: &InventionProblem) -> BTreeSet<TwoParameterFamily> {
    let mut families = BTreeSet::new();
    for (left_index, left) in problem.parameters.iter().enumerate() {
        for (right_index, right) in problem.parameters.iter().enumerate() {
            if left_index == right_index {
                continue;
            }
            for family in [
                TwoParameterFamily::Add,
                TwoParameterFamily::Subtract,
                TwoParameterFamily::Minimum,
                TwoParameterFamily::Maximum,
                TwoParameterFamily::ZeroGuardedSelection,
            ] {
                let exact = problem.demonstrations.iter().all(|example| {
                    let (Some(Value::Int(left_value)), Some(Value::Int(right_value))) = (
                        example.inputs.get(&left.name),
                        example.inputs.get(&right.name),
                    ) else {
                        return false;
                    };
                    let Some(predicted) = apply_family(family, *left_value, *right_value) else {
                        return false;
                    };
                    example.output == Value::Int(predicted)
                });
                if exact {
                    families.insert(family);
                }
            }
        }
    }
    families
}

fn apply_family(family: TwoParameterFamily, left: i64, right: i64) -> Option<i64> {
    match family {
        TwoParameterFamily::Add => left.checked_add(right),
        TwoParameterFamily::Subtract => left.checked_sub(right),
        TwoParameterFamily::Minimum => Some(left.min(right)),
        TwoParameterFamily::Maximum => Some(left.max(right)),
        TwoParameterFamily::ZeroGuardedSelection => Some(if left == 0 { right } else { left }),
    }
}

fn three_parameter_families(problem: &InventionProblem) -> BTreeSet<ThreeParameterFamily> {
    let mut families = BTreeSet::new();
    for (guard_index, guard) in problem.parameters.iter().enumerate() {
        for (left_index, left) in problem.parameters.iter().enumerate() {
            if left_index == guard_index {
                continue;
            }
            for (right_index, right) in problem.parameters.iter().enumerate() {
                if right_index == guard_index || right_index == left_index {
                    continue;
                }
                let zero_guarded = problem.demonstrations.iter().all(|example| {
                    integer_triplet(example, guard, left, right).is_some_and(
                        |(guard, left, right)| {
                            example.output == Value::Int(if guard == 0 { left } else { right })
                        },
                    )
                });
                if zero_guarded {
                    families.insert(ThreeParameterFamily::ZeroGuardedBranch);
                }
                let equality_guarded = problem.demonstrations.iter().all(|example| {
                    integer_triplet(example, guard, left, right).is_some_and(
                        |(guard, left, right)| {
                            example.output == Value::Int(if guard == left { right } else { left })
                        },
                    )
                });
                if equality_guarded {
                    families.insert(ThreeParameterFamily::EqualityGuardedBranch);
                }
            }
        }
    }
    families
}

fn integer_triplet(
    example: &cairn_invent::BindingExample,
    first: &cairn_rkl::Binder,
    second: &cairn_rkl::Binder,
    third: &cairn_rkl::Binder,
) -> Option<(i64, i64, i64)> {
    let (Value::Int(first), Value::Int(second), Value::Int(third)) = (
        example.inputs.get(&first.name)?,
        example.inputs.get(&second.name)?,
        example.inputs.get(&third.name)?,
    ) else {
        return None;
    };
    Some((*first, *second, *third))
}

fn value_class(ty: &Type) -> ValueClass {
    match ty {
        Type::Bool => ValueClass::Boolean,
        Type::Int(_) => ValueClass::Integer,
        Type::Atom => ValueClass::Categorical,
        Type::Coord2 => ValueClass::Coordinate,
        Type::Vector2 => ValueClass::Vector,
        Type::Axis => ValueClass::Axis,
        Type::Transform2 => ValueClass::Transform,
        Type::CellSet { .. } | Type::EntitySet { .. } | Type::List { .. } | Type::Map { .. } => {
            ValueClass::Collection
        }
        Type::Shape => ValueClass::Shape,
        Type::Entity => ValueClass::Entity,
        Type::Symbol => ValueClass::Symbol,
        Type::Action => ValueClass::Action,
        Type::State => ValueClass::State,
        Type::FiniteField { .. } => ValueClass::Field,
        Type::Tuple(_) => ValueClass::Tuple,
        Type::Option(_) => ValueClass::Optional,
        Type::Relation { .. } => ValueClass::Relation,
        Type::Operator { .. } => ValueClass::Operator,
        Type::Unit => ValueClass::Unit,
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ExperienceId(pub String);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExperienceCandidate {
    pub id: ExperienceId,
    pub expected_information_microunits: u64,
    pub expected_transfer_microunits: u64,
    pub underused_mechanism_bonus: u64,
    pub action_cost: u64,
    pub irreversible_risk: u64,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum LearningAxis {
    CausalLaw,
    Cardinality,
    Value,
    Layout,
    Composition,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FrontierGap {
    pub id: ExperienceId,
    pub axis: LearningAxis,
    pub competing_hypotheses: u64,
    pub expected_transfer_microunits: u64,
    pub prior_exercises: u64,
    pub action_cost: u64,
    pub irreversible_risk: u64,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct FrontierCurriculumGenerator;

impl FrontierCurriculumGenerator {
    /// Convert unresolved support and version-space gaps into executable
    /// curriculum candidates without task payloads or expected answers.
    #[must_use]
    pub fn generate(&self, gaps: &[FrontierGap]) -> Vec<ExperienceCandidate> {
        gaps.iter()
            .filter(|gap| gap.competing_hypotheses > 1)
            .map(|gap| ExperienceCandidate {
                id: gap.id.clone(),
                expected_information_microunits: gap
                    .competing_hypotheses
                    .saturating_sub(1)
                    .saturating_mul(1_000_000),
                expected_transfer_microunits: gap.expected_transfer_microunits,
                underused_mechanism_bonus: 1_000_000_u64
                    .checked_div(gap.prior_exercises.saturating_add(1))
                    .unwrap_or(0),
                action_cost: gap.action_cost,
                irreversible_risk: gap.irreversible_risk,
            })
            .collect()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CurriculumError {
    EmptyCandidates,
    DuplicateCandidate,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Curriculum;

impl Curriculum {
    /// Choose maximum expected learning value with stable ID tie-breaking.
    ///
    /// # Errors
    ///
    /// Rejects an empty or duplicate candidate set.
    pub fn select<'a>(
        &self,
        candidates: &'a [ExperienceCandidate],
    ) -> Result<&'a ExperienceCandidate, CurriculumError> {
        if candidates.is_empty() {
            return Err(CurriculumError::EmptyCandidates);
        }
        let mut ids = candidates
            .iter()
            .map(|candidate| &candidate.id)
            .collect::<Vec<_>>();
        ids.sort();
        ids.dedup();
        if ids.len() != candidates.len() {
            return Err(CurriculumError::DuplicateCandidate);
        }
        candidates
            .iter()
            .max_by_key(|candidate| (score(candidate), std::cmp::Reverse(&candidate.id)))
            .ok_or(CurriculumError::EmptyCandidates)
    }
}

/// One frozen measurement taken after a consolidation boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DevelopmentalPoint {
    pub available_mechanisms: u32,
    pub solved_evaluation_tasks: u32,
    pub evaluation_tasks: u32,
    pub candidate_executions_to_acquire_next: u64,
    pub cpu_nanos_to_acquire_next: u64,
    pub examples_to_acquire_next: u32,
    pub interventions_to_acquire_next: u32,
    pub maximum_composition_depth: u32,
    pub retained_prior_tasks: u32,
    pub prior_tasks: u32,
    pub negative_transfers: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ScalingError {
    InsufficientStages,
    EmptyEvaluation,
    MechanismCountDidNotGrow,
    CapabilityRegressed,
    CapabilityDidNotGrow,
    MarginalSearchDidNotFall,
    MarginalCpuDidNotFall,
    MarginalExamplesDidNotFall,
    MarginalInterventionsDidNotFall,
    CompositionDepthDidNotGrow,
    CatastrophicForgetting,
    NegativeTransfer,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct DevelopmentalGate;

impl DevelopmentalGate {
    /// Check the signature of developmental learning on frozen measurements.
    ///
    /// Search, examples, and interventions compare the first independently
    /// acquired mechanisms with the final mechanism acquired by reuse or
    /// composition. Intermediate capability and retention may never regress.
    ///
    /// # Errors
    ///
    /// Returns the first violated scaling invariant.
    pub fn verify(&self, points: &[DevelopmentalPoint]) -> Result<(), ScalingError> {
        if points.len() < 3 {
            return Err(ScalingError::InsufficientStages);
        }
        if points.iter().any(|point| point.evaluation_tasks == 0) {
            return Err(ScalingError::EmptyEvaluation);
        }
        for pair in points.windows(2) {
            if pair[1].available_mechanisms <= pair[0].available_mechanisms {
                return Err(ScalingError::MechanismCountDidNotGrow);
            }
            if u64::from(pair[1].solved_evaluation_tasks) * u64::from(pair[0].evaluation_tasks)
                < u64::from(pair[0].solved_evaluation_tasks) * u64::from(pair[1].evaluation_tasks)
            {
                return Err(ScalingError::CapabilityRegressed);
            }
            if pair[1].retained_prior_tasks < pair[1].prior_tasks {
                return Err(ScalingError::CatastrophicForgetting);
            }
            if pair[1].negative_transfers != 0 {
                return Err(ScalingError::NegativeTransfer);
            }
        }
        let first = &points[0];
        let Some(last) = points.last() else {
            return Err(ScalingError::InsufficientStages);
        };
        if u64::from(last.solved_evaluation_tasks) * u64::from(first.evaluation_tasks)
            <= u64::from(first.solved_evaluation_tasks) * u64::from(last.evaluation_tasks)
        {
            return Err(ScalingError::CapabilityDidNotGrow);
        }
        if last.candidate_executions_to_acquire_next >= first.candidate_executions_to_acquire_next {
            return Err(ScalingError::MarginalSearchDidNotFall);
        }
        if last.cpu_nanos_to_acquire_next >= first.cpu_nanos_to_acquire_next {
            return Err(ScalingError::MarginalCpuDidNotFall);
        }
        if last.examples_to_acquire_next >= first.examples_to_acquire_next {
            return Err(ScalingError::MarginalExamplesDidNotFall);
        }
        if last.interventions_to_acquire_next >= first.interventions_to_acquire_next {
            return Err(ScalingError::MarginalInterventionsDidNotFall);
        }
        if last.maximum_composition_depth <= first.maximum_composition_depth {
            return Err(ScalingError::CompositionDepthDidNotGrow);
        }
        Ok(())
    }
}

fn score(candidate: &ExperienceCandidate) -> i128 {
    i128::from(candidate.expected_information_microunits)
        + i128::from(candidate.expected_transfer_microunits)
        + i128::from(candidate.underused_mechanism_bonus)
        - i128::from(candidate.action_cost)
        - i128::from(candidate.irreversible_risk)
}

#[cfg(test)]
mod tests {
    use super::{DevelopmentalGate, DevelopmentalPoint, ScalingError};

    fn point(mechanisms: u32, solved: u32, search: u64, examples: u32) -> DevelopmentalPoint {
        DevelopmentalPoint {
            available_mechanisms: mechanisms,
            solved_evaluation_tasks: solved,
            evaluation_tasks: 3,
            candidate_executions_to_acquire_next: search,
            cpu_nanos_to_acquire_next: search,
            examples_to_acquire_next: examples,
            interventions_to_acquire_next: examples,
            maximum_composition_depth: mechanisms,
            retained_prior_tasks: mechanisms,
            prior_tasks: mechanisms,
            negative_transfers: 0,
        }
    }

    #[test]
    fn developmental_gate_accepts_growth_with_falling_cost() {
        assert_eq!(
            DevelopmentalGate
                .verify(&[point(1, 1, 100, 4), point(2, 2, 30, 2), point(3, 3, 1, 0),]),
            Ok(())
        );
    }

    #[test]
    fn developmental_gate_rejects_a_solve_rate_only_claim() {
        let result = DevelopmentalGate.verify(&[
            point(1, 1, 100, 4),
            point(2, 2, 80, 3),
            point(3, 3, 120, 2),
        ]);
        assert_eq!(result, Err(ScalingError::MarginalSearchDidNotFall));
    }
}
