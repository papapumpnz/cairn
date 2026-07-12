//! Deterministic developmental curriculum selection from support uncertainty.

use cairn_invent::{InventionError, InventionProblem};
use cairn_rkl::{Type, Value};
use serde::Serialize;
use std::collections::{btree_map::Entry, BTreeMap, BTreeSet};

pub const FAILURE_CORE_SEMANTICS_VERSION: &str = "1.0.0";
pub const DEVELOPMENTAL_CURRICULUM_SEMANTICS_VERSION: &str = "1.0.0";

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

/// Stable curriculum-level role for a learned mechanism. It is not a task ID
/// or answer key; the evidence identity recorded on success remains separate.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct MechanismTarget(pub String);

/// One unresolved developmental objective. Prerequisites refer only to
/// previously verified mechanism targets, allowing composition exercises to
/// become eligible from experience rather than scripted stage order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DevelopmentalObjective {
    pub id: ExperienceId,
    pub axis: LearningAxis,
    pub produces: MechanismTarget,
    pub prerequisites: BTreeSet<MechanismTarget>,
    pub competing_hypotheses: u64,
    pub expected_transfer_microunits: u64,
    pub action_cost: u64,
    pub irreversible_risk: u64,
}

/// Immutable explanation of one autonomous curriculum choice.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CurriculumDecision {
    pub sequence: u64,
    pub objective: ExperienceId,
    pub axis: LearningAxis,
    pub produces: MechanismTarget,
    pub expected_information_microunits: u64,
    pub expected_transfer_microunits: u64,
    pub underuse_bonus: u64,
}

/// Independently measured result returned by the learning and verification
/// runtimes. Curriculum never receives task payloads or expected answers.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DevelopmentalOutcome {
    pub decision_sequence: u64,
    pub objective: ExperienceId,
    pub mechanism_evidence: String,
    pub independently_verified: bool,
    pub held_out_transfer_cases: u32,
    pub candidate_work_without: u64,
    pub candidate_work_with: u64,
    pub examples_without: u32,
    pub examples_with: u32,
    pub retained_prior_tasks: u32,
    pub prior_tasks: u32,
    pub negative_transfers: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LearnedMechanismStatus {
    Active,
    Quarantined,
    Deprecated,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LearnedMechanismRecord {
    pub target: MechanismTarget,
    pub evidence: String,
    pub learned_axis: LearningAxis,
    pub held_out_transfer_cases: u32,
    pub verified_reuses: u32,
    pub status: LearnedMechanismStatus,
    counterexamples: BTreeSet<String>,
}

impl LearnedMechanismRecord {
    #[must_use]
    pub fn counterexamples(&self) -> &BTreeSet<String> {
        &self.counterexamples
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AutonomousCurriculumError {
    EmptyObjectives,
    EmptyBudget,
    DuplicateObjective,
    DuplicateMechanismTarget,
    Complete,
    Capacity,
    BlockedByPrerequisites,
    OutstandingDecision,
    NoOutstandingDecision,
    DecisionMismatch,
    UnverifiedOutcome,
    NoMeasuredLearningReduction,
    RetentionRegression,
    NegativeTransfer,
    UnknownMechanism,
    DuplicateCounterexample,
}

/// Stateful deterministic developmental controller. It selects only eligible
/// unresolved objectives, derives priority from uncertainty/transfer/underuse,
/// and updates support solely from independently verified outcomes.
#[derive(Clone, Debug)]
pub struct AutonomousCurriculum {
    objectives: BTreeMap<ExperienceId, DevelopmentalObjective>,
    attempts: BTreeMap<ExperienceId, u64>,
    completed: BTreeSet<ExperienceId>,
    mechanisms: BTreeMap<MechanismTarget, LearnedMechanismRecord>,
    history: Vec<DevelopmentalOutcome>,
    outstanding: Option<CurriculumDecision>,
    issued: u64,
    maximum_decisions: u64,
}

impl AutonomousCurriculum {
    /// Construct a bounded curriculum and validate objective identity and
    /// produced-mechanism uniqueness.
    ///
    /// # Errors
    ///
    /// Rejects empty curricula/budgets and duplicate objective or mechanism
    /// targets before state is created.
    pub fn new(
        objectives: Vec<DevelopmentalObjective>,
        maximum_decisions: u64,
    ) -> Result<Self, AutonomousCurriculumError> {
        if objectives.is_empty() {
            return Err(AutonomousCurriculumError::EmptyObjectives);
        }
        if maximum_decisions == 0 {
            return Err(AutonomousCurriculumError::EmptyBudget);
        }
        let mut indexed = BTreeMap::new();
        let mut targets = BTreeSet::new();
        for objective in objectives {
            if !targets.insert(objective.produces.clone()) {
                return Err(AutonomousCurriculumError::DuplicateMechanismTarget);
            }
            match indexed.entry(objective.id.clone()) {
                Entry::Vacant(entry) => {
                    entry.insert(objective);
                }
                Entry::Occupied(_) => {
                    return Err(AutonomousCurriculumError::DuplicateObjective);
                }
            }
        }
        Ok(Self {
            objectives: indexed,
            attempts: BTreeMap::new(),
            completed: BTreeSet::new(),
            mechanisms: BTreeMap::new(),
            history: Vec::new(),
            outstanding: None,
            issued: 0,
            maximum_decisions,
        })
    }

    /// Select the highest-value currently eligible objective. Completion,
    /// capacity, and prerequisite deadlock are distinct terminal states.
    ///
    /// # Errors
    ///
    /// Rejects an unrecorded previous decision and returns exact terminal state
    /// when no further bounded decision can be issued.
    pub fn select_next(&mut self) -> Result<CurriculumDecision, AutonomousCurriculumError> {
        if self.outstanding.is_some() {
            return Err(AutonomousCurriculumError::OutstandingDecision);
        }
        if self.completed.len() == self.objectives.len() {
            return Err(AutonomousCurriculumError::Complete);
        }
        if self.issued >= self.maximum_decisions {
            return Err(AutonomousCurriculumError::Capacity);
        }
        let active = self
            .mechanisms
            .iter()
            .filter(|(_, record)| record.status == LearnedMechanismStatus::Active)
            .map(|(target, _)| target)
            .collect::<BTreeSet<_>>();
        let candidates = self
            .objectives
            .values()
            .filter(|objective| !self.completed.contains(&objective.id))
            .filter(|objective| {
                objective
                    .prerequisites
                    .iter()
                    .all(|required| active.contains(required))
            })
            .map(|objective| ExperienceCandidate {
                id: objective.id.clone(),
                expected_information_microunits: objective
                    .competing_hypotheses
                    .saturating_sub(1)
                    .saturating_mul(1_000_000),
                expected_transfer_microunits: objective.expected_transfer_microunits,
                underused_mechanism_bonus: objective
                    .prerequisites
                    .iter()
                    .filter_map(|required| self.mechanisms.get(required))
                    .map(|record| {
                        1_000_000_u64
                            .checked_div(u64::from(record.verified_reuses).saturating_add(1))
                            .unwrap_or(0)
                    })
                    .fold(0_u64, u64::saturating_add),
                action_cost: objective.action_cost,
                irreversible_risk: objective.irreversible_risk,
            })
            .collect::<Vec<_>>();
        let selected = match Curriculum.select(&candidates) {
            Ok(selected) => selected,
            Err(CurriculumError::EmptyCandidates) => {
                return Err(AutonomousCurriculumError::BlockedByPrerequisites);
            }
            Err(CurriculumError::DuplicateCandidate) => {
                return Err(AutonomousCurriculumError::DuplicateObjective);
            }
        };
        let objective = self
            .objectives
            .get(&selected.id)
            .ok_or(AutonomousCurriculumError::BlockedByPrerequisites)?;
        self.issued = self
            .issued
            .checked_add(1)
            .ok_or(AutonomousCurriculumError::Capacity)?;
        let attempts = self.attempts.entry(objective.id.clone()).or_default();
        *attempts = attempts
            .checked_add(1)
            .ok_or(AutonomousCurriculumError::Capacity)?;
        let decision = CurriculumDecision {
            sequence: self.issued,
            objective: objective.id.clone(),
            axis: objective.axis,
            produces: objective.produces.clone(),
            expected_information_microunits: selected.expected_information_microunits,
            expected_transfer_microunits: selected.expected_transfer_microunits,
            underuse_bonus: selected.underused_mechanism_bonus,
        };
        self.outstanding = Some(decision.clone());
        Ok(decision)
    }

    /// Admit one independently verified learning result and expose its new
    /// mechanism to prerequisite resolution.
    ///
    /// # Errors
    ///
    /// Rejects mismatched/unverified results, absent work/sample reduction,
    /// retention regression, and any negative transfer.
    pub fn record(
        &mut self,
        outcome: DevelopmentalOutcome,
    ) -> Result<(), AutonomousCurriculumError> {
        let decision = self
            .outstanding
            .as_ref()
            .ok_or(AutonomousCurriculumError::NoOutstandingDecision)?;
        if decision.sequence != outcome.decision_sequence || decision.objective != outcome.objective
        {
            return Err(AutonomousCurriculumError::DecisionMismatch);
        }
        if !outcome.independently_verified
            || outcome.held_out_transfer_cases == 0
            || outcome.mechanism_evidence.trim().is_empty()
        {
            return Err(AutonomousCurriculumError::UnverifiedOutcome);
        }
        if outcome.candidate_work_with >= outcome.candidate_work_without
            || outcome.examples_with >= outcome.examples_without
        {
            return Err(AutonomousCurriculumError::NoMeasuredLearningReduction);
        }
        if outcome.retained_prior_tasks < outcome.prior_tasks {
            return Err(AutonomousCurriculumError::RetentionRegression);
        }
        if outcome.negative_transfers != 0 {
            return Err(AutonomousCurriculumError::NegativeTransfer);
        }
        self.mechanisms.insert(
            decision.produces.clone(),
            LearnedMechanismRecord {
                target: decision.produces.clone(),
                evidence: outcome.mechanism_evidence.clone(),
                learned_axis: decision.axis,
                held_out_transfer_cases: outcome.held_out_transfer_cases,
                verified_reuses: 1,
                status: LearnedMechanismStatus::Active,
                counterexamples: BTreeSet::new(),
            },
        );
        for prerequisite in self
            .objectives
            .get(&decision.objective)
            .into_iter()
            .flat_map(|objective| &objective.prerequisites)
        {
            if let Some(record) = self.mechanisms.get_mut(prerequisite) {
                record.verified_reuses = record.verified_reuses.saturating_add(1);
            }
        }
        self.completed.insert(decision.objective.clone());
        self.history.push(outcome);
        self.outstanding = None;
        Ok(())
    }

    /// Record a distinct independently verified counterexample. The first
    /// quarantines the mechanism; the second deprecates it.
    ///
    /// # Errors
    ///
    /// Rejects unknown mechanisms, unverified evidence, and duplicates.
    pub fn record_counterexample(
        &mut self,
        target: &MechanismTarget,
        evidence: String,
        independently_verified: bool,
    ) -> Result<LearnedMechanismStatus, AutonomousCurriculumError> {
        if !independently_verified || evidence.trim().is_empty() {
            return Err(AutonomousCurriculumError::UnverifiedOutcome);
        }
        let record = self
            .mechanisms
            .get_mut(target)
            .ok_or(AutonomousCurriculumError::UnknownMechanism)?;
        if !record.counterexamples.insert(evidence) {
            return Err(AutonomousCurriculumError::DuplicateCounterexample);
        }
        record.status = if record.counterexamples.len() >= 2 {
            LearnedMechanismStatus::Deprecated
        } else {
            LearnedMechanismStatus::Quarantined
        };
        Ok(record.status)
    }

    #[must_use]
    pub fn mechanisms(&self) -> &BTreeMap<MechanismTarget, LearnedMechanismRecord> {
        &self.mechanisms
    }

    #[must_use]
    pub fn history(&self) -> &[DevelopmentalOutcome] {
        &self.history
    }

    #[must_use]
    pub fn completed_objectives(&self) -> usize {
        self.completed.len()
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

/// One independently verified phase in a sustained autonomous curriculum.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SustainedLearningPoint {
    pub learned_mechanisms: u32,
    pub solved_capabilities: u32,
    pub evaluated_capabilities: u32,
    pub candidate_work_without: u64,
    pub candidate_work_with: u64,
    pub examples_without: u32,
    pub examples_with: u32,
    pub proof_units_without: u64,
    pub proof_units_with: u64,
    pub retained_prior_tasks: u32,
    pub prior_tasks: u32,
    pub negative_transfers: u32,
    pub transfer_attempts: u32,
    pub completed_restarts: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SustainedLearningError {
    InsufficientStages,
    EmptyEvaluation,
    MechanismCountDidNotGrow,
    CapabilityRegressed,
    CapabilityDidNotGrow,
    NoCandidateReduction,
    NoSampleReduction,
    NoProofReduction,
    CatastrophicForgetting,
    NegativeTransferRate,
    MissingRestart,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct SustainedLearningGate;

impl SustainedLearningGate {
    /// Verify repeated learning rather than a single favorable acquisition.
    /// Every phase must reduce candidate, sample, and proof work against its
    /// own frozen memory-off baseline; retention, negative transfer, and
    /// restart are gated independently of task-size variation.
    ///
    /// # Errors
    ///
    /// Returns the first violated sustained-learning invariant.
    pub fn verify(&self, points: &[SustainedLearningPoint]) -> Result<(), SustainedLearningError> {
        if points.len() < 3 {
            return Err(SustainedLearningError::InsufficientStages);
        }
        for point in points {
            if point.evaluated_capabilities == 0 {
                return Err(SustainedLearningError::EmptyEvaluation);
            }
            if point.candidate_work_with >= point.candidate_work_without {
                return Err(SustainedLearningError::NoCandidateReduction);
            }
            if point.examples_with >= point.examples_without {
                return Err(SustainedLearningError::NoSampleReduction);
            }
            if point.proof_units_with >= point.proof_units_without {
                return Err(SustainedLearningError::NoProofReduction);
            }
            if point.retained_prior_tasks < point.prior_tasks {
                return Err(SustainedLearningError::CatastrophicForgetting);
            }
            if point.transfer_attempts == 0
                || u64::from(point.negative_transfers).saturating_mul(100)
                    >= u64::from(point.transfer_attempts).saturating_mul(2)
            {
                return Err(SustainedLearningError::NegativeTransferRate);
            }
        }
        for pair in points.windows(2) {
            if pair[1].learned_mechanisms <= pair[0].learned_mechanisms {
                return Err(SustainedLearningError::MechanismCountDidNotGrow);
            }
            if u64::from(pair[1].solved_capabilities)
                .saturating_mul(u64::from(pair[0].evaluated_capabilities))
                < u64::from(pair[0].solved_capabilities)
                    .saturating_mul(u64::from(pair[1].evaluated_capabilities))
            {
                return Err(SustainedLearningError::CapabilityRegressed);
            }
        }
        let last = points
            .last()
            .ok_or(SustainedLearningError::InsufficientStages)?;
        let first = &points[0];
        if u64::from(last.solved_capabilities)
            .saturating_mul(u64::from(first.evaluated_capabilities))
            <= u64::from(first.solved_capabilities)
                .saturating_mul(u64::from(last.evaluated_capabilities))
        {
            return Err(SustainedLearningError::CapabilityDidNotGrow);
        }
        if last.completed_restarts == 0 {
            return Err(SustainedLearningError::MissingRestart);
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
