//! Deterministic developmental curriculum selection from support uncertainty.

use cairn_cas::{Cas, CasError, StoredArtifact};
use cairn_core::{
    ArtifactEnvelope, ArtifactError, ArtifactId, ArtifactKind, BlobId, BudgetRecord,
    CanonicalEncode, CanonicalLimits, CanonicalValue, Hash32, ProducerRecord, SchemaDescriptor,
    SchemaRef, SchemaRegistry, SemanticsVersion,
};
use cairn_invent::{InventionError, InventionProblem};
use cairn_rkl::{Type, Value};
use serde::Serialize;
use std::collections::{btree_map::Entry, BTreeMap, BTreeSet};
use std::fmt;

pub const FAILURE_CORE_SEMANTICS_VERSION: &str = "1.0.0";
pub const DEVELOPMENTAL_CURRICULUM_SEMANTICS_VERSION: &str = "1.2.0";
pub const CURRICULUM_CHECKPOINT_SCHEMA: &str = "cairn.curriculum.checkpoint";
pub const CURRICULUM_CHECKPOINT_SCHEMA_VERSION: u32 = 1;

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

/// Structural reason a new developmental objective was generated. This is a
/// curriculum category, never a domain or benchmark family label.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum DevelopmentalNeed {
    LatentContext,
    VariableCardinality,
    RelationalStructure,
    MechanismComposition,
    ValueLaw,
}

/// Deterministic content-free proposal generated from one redacted failure
/// core and the currently reusable mechanism roles.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeneratedDevelopmentalObjective {
    pub objective: DevelopmentalObjective,
    pub need: DevelopmentalNeed,
    pub failure_core: Hash32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ObjectiveGenerationError {
    UnsupportedFailureCoreSemantics,
    RestrictedContent,
    InvalidEvidence,
    Canonicalization,
}

/// Reference generator for turning structural failure evidence into a new
/// experiment objective without access to observations or expected answers.
#[derive(Clone, Copy, Debug, Default)]
pub struct FailureDrivenObjectiveGenerator;

impl FailureDrivenObjectiveGenerator {
    /// Generate one deterministic developmental objective. At most three
    /// active support roles become prerequisites for a composition objective.
    ///
    /// # Errors
    ///
    /// Rejects unsupported semantics, non-redacted evidence, empty evidence,
    /// and canonicalization failure.
    pub fn generate(
        &self,
        core: &LearningFailureCore,
        active_support: &BTreeSet<MechanismTarget>,
    ) -> Result<GeneratedDevelopmentalObjective, ObjectiveGenerationError> {
        if core.semantics_version != FAILURE_CORE_SEMANTICS_VERSION {
            return Err(ObjectiveGenerationError::UnsupportedFailureCoreSemantics);
        }
        if core.redaction != RedactionStatus::ContentFree {
            return Err(ObjectiveGenerationError::RestrictedContent);
        }
        if core.demonstrations == 0 || core.parameter_types.is_empty() {
            return Err(ObjectiveGenerationError::InvalidEvidence);
        }
        let need = developmental_need(core, active_support.len());
        let axis = match need {
            DevelopmentalNeed::LatentContext => LearningAxis::CausalLaw,
            DevelopmentalNeed::VariableCardinality => LearningAxis::Cardinality,
            DevelopmentalNeed::RelationalStructure => LearningAxis::Layout,
            DevelopmentalNeed::MechanismComposition => LearningAxis::Composition,
            DevelopmentalNeed::ValueLaw => LearningAxis::Value,
        };
        let prerequisites = if need == DevelopmentalNeed::MechanismComposition {
            active_support.iter().take(3).cloned().collect()
        } else {
            BTreeSet::new()
        };
        let failure_core = failure_core_hash(core)?;
        let objective_identity = generated_objective_hash(failure_core, &prerequisites)?;
        let suffix = objective_identity.to_hex();
        let competing_hypotheses = u64::try_from(core.distinct_output_classes)
            .unwrap_or(u64::MAX)
            .saturating_add(u64::try_from(core.minimum_dependency_arity_lower_bound).unwrap_or(0))
            .max(2);
        let expected_transfer_microunits = u64::try_from(core.varying_parameters)
            .unwrap_or(u64::MAX)
            .saturating_add(1)
            .saturating_mul(1_000_000);
        Ok(GeneratedDevelopmentalObjective {
            objective: DevelopmentalObjective {
                id: ExperienceId(format!("generated-{suffix}")),
                axis,
                produces: MechanismTarget(format!("mechanism-{suffix}")),
                prerequisites,
                competing_hypotheses,
                expected_transfer_microunits,
                action_cost: u64::try_from(core.demonstrations).unwrap_or(u64::MAX),
                irreversible_risk: u64::from(
                    core.determinism == DeterminismStatus::ConflictingCompleteInputs,
                ),
            },
            need,
            failure_core,
        })
    }
}

fn developmental_need(core: &LearningFailureCore, available_support: usize) -> DevelopmentalNeed {
    if core.determinism == DeterminismStatus::ConflictingCompleteInputs {
        DevelopmentalNeed::LatentContext
    } else if core.parameter_types.contains(&ValueClass::Collection)
        || core.return_type == ValueClass::Collection
    {
        DevelopmentalNeed::VariableCardinality
    } else if core.minimum_dependency_arity_lower_bound >= 2 && available_support >= 2 {
        DevelopmentalNeed::MechanismComposition
    } else if core.parameter_types.iter().any(|class| {
        matches!(
            class,
            ValueClass::Entity | ValueClass::Relation | ValueClass::Shape | ValueClass::Coordinate
        )
    }) {
        DevelopmentalNeed::RelationalStructure
    } else {
        DevelopmentalNeed::ValueLaw
    }
}

fn failure_core_hash(core: &LearningFailureCore) -> Result<Hash32, ObjectiveGenerationError> {
    let value = CanonicalValue::Array(vec![
        CanonicalValue::Text("cairn.failure-core.objective.v1".to_owned()),
        CanonicalValue::Text(core.semantics_version.clone()),
        search_failure_value(core.failure),
        CanonicalValue::Array(
            core.parameter_types
                .iter()
                .map(|class| CanonicalValue::Unsigned(value_class_code(*class)))
                .collect(),
        ),
        CanonicalValue::Unsigned(value_class_code(core.return_type)),
        CanonicalValue::Unsigned(u64::try_from(core.demonstrations).unwrap_or(u64::MAX)),
        CanonicalValue::Unsigned(u64::try_from(core.varying_parameters).unwrap_or(u64::MAX)),
        CanonicalValue::Unsigned(u64::try_from(core.distinct_output_classes).unwrap_or(u64::MAX)),
        CanonicalValue::Bool(core.constant_explanation_possible),
        CanonicalValue::Unsigned(
            u64::try_from(core.direct_projection_candidates).unwrap_or(u64::MAX),
        ),
        CanonicalValue::Unsigned(
            u64::try_from(core.single_parameter_explanations).unwrap_or(u64::MAX),
        ),
        CanonicalValue::Array(
            core.exact_two_parameter_families
                .iter()
                .map(|family| CanonicalValue::Unsigned(two_parameter_family_code(*family)))
                .collect(),
        ),
        CanonicalValue::Array(
            core.exact_three_parameter_families
                .iter()
                .map(|family| CanonicalValue::Unsigned(three_parameter_family_code(*family)))
                .collect(),
        ),
        CanonicalValue::Unsigned(match core.determinism {
            DeterminismStatus::Deterministic => 0,
            DeterminismStatus::ConflictingCompleteInputs => 1,
        }),
        CanonicalValue::Unsigned(
            u64::try_from(core.minimum_dependency_arity_lower_bound).unwrap_or(u64::MAX),
        ),
    ]);
    value
        .encode_canonical()
        .map(|bytes| Hash32::digest(&bytes))
        .map_err(|_| ObjectiveGenerationError::Canonicalization)
}

fn generated_objective_hash(
    failure_core: Hash32,
    prerequisites: &BTreeSet<MechanismTarget>,
) -> Result<Hash32, ObjectiveGenerationError> {
    CanonicalValue::Array(vec![
        CanonicalValue::Text("cairn.generated-objective.v1".to_owned()),
        CanonicalValue::Bytes(failure_core.as_bytes().to_vec()),
        CanonicalValue::Array(
            prerequisites
                .iter()
                .map(|target| CanonicalValue::Text(target.0.clone()))
                .collect(),
        ),
    ])
    .encode_canonical()
    .map(|bytes| Hash32::digest(&bytes))
    .map_err(|_| ObjectiveGenerationError::Canonicalization)
}

fn search_failure_value(failure: SearchFailure) -> CanonicalValue {
    match failure {
        SearchFailure::CandidateCapacity { maximum } => CanonicalValue::Array(vec![
            CanonicalValue::Unsigned(0),
            CanonicalValue::Unsigned(maximum),
        ]),
        SearchFailure::NoDefinition { maximum_cost } => CanonicalValue::Array(vec![
            CanonicalValue::Unsigned(1),
            CanonicalValue::Unsigned(u64::from(maximum_cost)),
        ]),
    }
}

const fn value_class_code(class: ValueClass) -> u64 {
    class as u64
}

const fn two_parameter_family_code(family: TwoParameterFamily) -> u64 {
    family as u64
}

const fn three_parameter_family_code(family: ThreeParameterFamily) -> u64 {
    family as u64
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

/// Immutable semantic journal entry used to reconstruct developmental state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CurriculumEvent {
    ObjectiveAdded {
        objective: DevelopmentalObjective,
    },
    Learned {
        decision: CurriculumDecision,
        outcome: DevelopmentalOutcome,
    },
    Counterexample {
        target: MechanismTarget,
        evidence: String,
    },
}

/// Canonical replay input for a curriculum process. It contains no task
/// payloads, answers, verifier tokens, or mutable store snapshot.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CurriculumCheckpoint {
    semantics_version: String,
    objectives: Vec<DevelopmentalObjective>,
    maximum_decisions: u64,
    events: Vec<CurriculumEvent>,
    outstanding: Option<CurriculumDecision>,
}

impl CurriculumCheckpoint {
    #[must_use]
    pub fn objectives(&self) -> &[DevelopmentalObjective] {
        &self.objectives
    }

    #[must_use]
    pub fn events(&self) -> &[CurriculumEvent] {
        &self.events
    }

    #[must_use]
    pub const fn outstanding(&self) -> Option<&CurriculumDecision> {
        self.outstanding.as_ref()
    }
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
    ObjectiveAdmissionWhileOutstanding,
    UnsupportedCheckpointSemantics,
    CheckpointReplayMismatch,
}

/// Stateful deterministic developmental controller. It selects only eligible
/// unresolved objectives, derives priority from uncertainty/transfer/underuse,
/// and updates support solely from independently verified outcomes.
#[derive(Clone, Debug)]
pub struct AutonomousCurriculum {
    initial_objectives: Vec<DevelopmentalObjective>,
    objectives: BTreeMap<ExperienceId, DevelopmentalObjective>,
    attempts: BTreeMap<ExperienceId, u64>,
    completed: BTreeSet<ExperienceId>,
    mechanisms: BTreeMap<MechanismTarget, LearnedMechanismRecord>,
    history: Vec<DevelopmentalOutcome>,
    events: Vec<CurriculumEvent>,
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
        let initial_objectives = indexed.values().cloned().collect();
        Ok(Self {
            initial_objectives,
            objectives: indexed,
            attempts: BTreeMap::new(),
            completed: BTreeSet::new(),
            mechanisms: BTreeMap::new(),
            history: Vec::new(),
            events: Vec::new(),
            outstanding: None,
            issued: 0,
            maximum_decisions,
        })
    }

    /// Admit a newly generated objective at a deterministic event boundary.
    ///
    /// # Errors
    ///
    /// Rejects admission while work is outstanding and duplicate objective or
    /// produced-mechanism identities.
    pub fn add_objective(
        &mut self,
        objective: DevelopmentalObjective,
    ) -> Result<(), AutonomousCurriculumError> {
        if self.outstanding.is_some() {
            return Err(AutonomousCurriculumError::ObjectiveAdmissionWhileOutstanding);
        }
        if self.objectives.contains_key(&objective.id) {
            return Err(AutonomousCurriculumError::DuplicateObjective);
        }
        if self
            .objectives
            .values()
            .any(|existing| existing.produces == objective.produces)
        {
            return Err(AutonomousCurriculumError::DuplicateMechanismTarget);
        }
        self.objectives
            .insert(objective.id.clone(), objective.clone());
        self.events
            .push(CurriculumEvent::ObjectiveAdded { objective });
        Ok(())
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
        self.history.push(outcome.clone());
        self.events.push(CurriculumEvent::Learned {
            decision: decision.clone(),
            outcome,
        });
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
        if self.outstanding.is_some() {
            return Err(AutonomousCurriculumError::OutstandingDecision);
        }
        if !independently_verified || evidence.trim().is_empty() {
            return Err(AutonomousCurriculumError::UnverifiedOutcome);
        }
        let event_evidence = evidence.clone();
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
        let status = record.status;
        self.events.push(CurriculumEvent::Counterexample {
            target: target.clone(),
            evidence: event_evidence,
        });
        Ok(status)
    }

    /// Produce an immutable content-free replay checkpoint.
    #[must_use]
    pub fn checkpoint(&self) -> CurriculumCheckpoint {
        CurriculumCheckpoint {
            semantics_version: DEVELOPMENTAL_CURRICULUM_SEMANTICS_VERSION.to_owned(),
            objectives: self.initial_objectives.clone(),
            maximum_decisions: self.maximum_decisions,
            events: self.events.clone(),
            outstanding: self.outstanding.clone(),
        }
    }

    /// Reconstruct curriculum state only by replaying every journal entry
    /// through ordinary selection, outcome, and counterexample methods.
    ///
    /// # Errors
    ///
    /// Rejects unsupported semantics or any decision, evidence, ordering, or
    /// terminal-state divergence.
    pub fn restore(checkpoint: &CurriculumCheckpoint) -> Result<Self, AutonomousCurriculumError> {
        if checkpoint.semantics_version != DEVELOPMENTAL_CURRICULUM_SEMANTICS_VERSION {
            return Err(AutonomousCurriculumError::UnsupportedCheckpointSemantics);
        }
        let mut restored = Self::new(checkpoint.objectives.clone(), checkpoint.maximum_decisions)?;
        for event in &checkpoint.events {
            match event {
                CurriculumEvent::ObjectiveAdded { objective } => {
                    restored.add_objective(objective.clone())?;
                }
                CurriculumEvent::Learned { decision, outcome } => {
                    let replayed = restored.select_next()?;
                    if &replayed != decision {
                        return Err(AutonomousCurriculumError::CheckpointReplayMismatch);
                    }
                    restored.record(outcome.clone())?;
                }
                CurriculumEvent::Counterexample { target, evidence } => {
                    restored.record_counterexample(target, evidence.clone(), true)?;
                }
            }
        }
        if let Some(expected) = &checkpoint.outstanding {
            let replayed = restored.select_next()?;
            if &replayed != expected {
                return Err(AutonomousCurriculumError::CheckpointReplayMismatch);
            }
        }
        if restored.events != checkpoint.events || restored.outstanding != checkpoint.outstanding {
            return Err(AutonomousCurriculumError::CheckpointReplayMismatch);
        }
        Ok(restored)
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

impl CanonicalEncode for CurriculumCheckpoint {
    fn encode_canonical(&self) -> Result<Vec<u8>, cairn_core::CanonicalError> {
        curriculum_checkpoint_value(self).encode_canonical()
    }
}

/// Non-semantic provenance and resource observations for one checkpoint.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CurriculumCheckpointMetadata {
    pub parent_ids: Vec<ArtifactId>,
    pub producer: ProducerRecord,
    pub budget: BudgetRecord,
    pub created_at_logical: u64,
}

/// Canonical CAS boundary for replay-validated curriculum checkpoints.
#[derive(Clone, Copy, Debug, Default)]
pub struct CurriculumCheckpointArchive;

impl CurriculumCheckpointArchive {
    /// Persist a checkpoint only after deterministic semantic replay succeeds.
    ///
    /// # Errors
    ///
    /// Rejects missing lineage, replay divergence, artifact/canonicalization
    /// failures, and storage errors.
    pub fn persist(
        &self,
        cas: &Cas,
        checkpoint: &CurriculumCheckpoint,
        metadata: CurriculumCheckpointMetadata,
    ) -> Result<StoredArtifact, CurriculumCheckpointArchiveError> {
        if metadata.parent_ids.is_empty() {
            return Err(CurriculumCheckpointArchiveError::MissingParent);
        }
        AutonomousCurriculum::restore(checkpoint)
            .map_err(CurriculumCheckpointArchiveError::Replay)?;
        let envelope = ArtifactEnvelope::new(
            curriculum_checkpoint_schema()?,
            curriculum_checkpoint_kind()?,
            curriculum_checkpoint_semantics(),
            metadata.parent_ids,
            None,
            metadata.producer,
            metadata.budget,
            metadata.created_at_logical,
            checkpoint.clone(),
        )?;
        cas.put_envelope(&envelope)
            .map_err(CurriculumCheckpointArchiveError::Storage)
    }

    /// Load, bound, decode, and deterministically replay a checkpoint before
    /// returning usable curriculum state.
    ///
    /// # Errors
    ///
    /// Rejects corruption, unsupported versions, missing lineage, non-global
    /// scope, malformed payloads, and any replay divergence.
    pub fn load(
        &self,
        cas: &Cas,
        blob_id: BlobId,
    ) -> Result<AutonomousCurriculum, CurriculumCheckpointArchiveError> {
        let mut registry = SchemaRegistry::new();
        registry.register(SchemaDescriptor {
            name: CURRICULUM_CHECKPOINT_SCHEMA.to_owned(),
            current_version: CURRICULUM_CHECKPOINT_SCHEMA_VERSION,
            additive_read_versions: BTreeSet::new(),
            maximum_payload_bytes: 1_048_576,
        })?;
        let envelope = cas.get_envelope(
            blob_id,
            &registry,
            &BTreeSet::from([curriculum_checkpoint_semantics()]),
            CanonicalLimits {
                max_input_bytes: 1_200_000,
                max_depth: 32,
                max_collection_items: 100_000,
                max_scalar_bytes: 65_536,
            },
        )?;
        if envelope.artifact_kind != curriculum_checkpoint_kind()?
            || envelope.parent_ids.is_empty()
            || envelope.task_scope.is_some()
        {
            return Err(CurriculumCheckpointArchiveError::InvalidEnvelope);
        }
        let checkpoint = curriculum_checkpoint_from_value(envelope.payload)?;
        AutonomousCurriculum::restore(&checkpoint).map_err(CurriculumCheckpointArchiveError::Replay)
    }
}

#[derive(Debug)]
pub enum CurriculumCheckpointArchiveError {
    MissingParent,
    InvalidEnvelope,
    InvalidPayload(&'static str),
    Replay(AutonomousCurriculumError),
    Artifact(ArtifactError),
    Storage(CasError),
}

impl fmt::Display for CurriculumCheckpointArchiveError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPayload(message) => {
                write!(formatter, "invalid curriculum checkpoint: {message}")
            }
            Self::Replay(error) => write!(formatter, "curriculum replay failed: {error:?}"),
            Self::Artifact(error) => error.fmt(formatter),
            Self::Storage(error) => error.fmt(formatter),
            other => write!(formatter, "{other:?}"),
        }
    }
}

impl std::error::Error for CurriculumCheckpointArchiveError {}

impl From<ArtifactError> for CurriculumCheckpointArchiveError {
    fn from(value: ArtifactError) -> Self {
        Self::Artifact(value)
    }
}

impl From<CasError> for CurriculumCheckpointArchiveError {
    fn from(value: CasError) -> Self {
        Self::Storage(value)
    }
}

fn curriculum_checkpoint_schema() -> Result<SchemaRef, ArtifactError> {
    SchemaRef::new(
        CURRICULUM_CHECKPOINT_SCHEMA,
        CURRICULUM_CHECKPOINT_SCHEMA_VERSION,
    )
}

fn curriculum_checkpoint_kind() -> Result<ArtifactKind, ArtifactError> {
    ArtifactKind::new("curriculum.checkpoint")
}

const fn curriculum_checkpoint_semantics() -> SemanticsVersion {
    SemanticsVersion::new(1, 2, 0)
}

fn curriculum_checkpoint_value(checkpoint: &CurriculumCheckpoint) -> CanonicalValue {
    CanonicalValue::Array(vec![
        CanonicalValue::Text("cairn.curriculum.checkpoint.v1".to_owned()),
        CanonicalValue::Text(checkpoint.semantics_version.clone()),
        CanonicalValue::Unsigned(checkpoint.maximum_decisions),
        CanonicalValue::Array(
            checkpoint
                .objectives
                .iter()
                .map(curriculum_objective_value)
                .collect(),
        ),
        CanonicalValue::Array(
            checkpoint
                .events
                .iter()
                .map(curriculum_event_value)
                .collect(),
        ),
        checkpoint
            .outstanding
            .as_ref()
            .map_or(CanonicalValue::Null, curriculum_decision_value),
    ])
}

fn curriculum_objective_value(objective: &DevelopmentalObjective) -> CanonicalValue {
    CanonicalValue::Array(vec![
        CanonicalValue::Text(objective.id.0.clone()),
        CanonicalValue::Unsigned(learning_axis_code(objective.axis)),
        CanonicalValue::Text(objective.produces.0.clone()),
        CanonicalValue::Array(
            objective
                .prerequisites
                .iter()
                .map(|target| CanonicalValue::Text(target.0.clone()))
                .collect(),
        ),
        CanonicalValue::Unsigned(objective.competing_hypotheses),
        CanonicalValue::Unsigned(objective.expected_transfer_microunits),
        CanonicalValue::Unsigned(objective.action_cost),
        CanonicalValue::Unsigned(objective.irreversible_risk),
    ])
}

fn curriculum_decision_value(decision: &CurriculumDecision) -> CanonicalValue {
    CanonicalValue::Array(vec![
        CanonicalValue::Unsigned(decision.sequence),
        CanonicalValue::Text(decision.objective.0.clone()),
        CanonicalValue::Unsigned(learning_axis_code(decision.axis)),
        CanonicalValue::Text(decision.produces.0.clone()),
        CanonicalValue::Unsigned(decision.expected_information_microunits),
        CanonicalValue::Unsigned(decision.expected_transfer_microunits),
        CanonicalValue::Unsigned(decision.underuse_bonus),
    ])
}

fn curriculum_outcome_value(outcome: &DevelopmentalOutcome) -> CanonicalValue {
    CanonicalValue::Array(vec![
        CanonicalValue::Unsigned(outcome.decision_sequence),
        CanonicalValue::Text(outcome.objective.0.clone()),
        CanonicalValue::Text(outcome.mechanism_evidence.clone()),
        CanonicalValue::Bool(outcome.independently_verified),
        CanonicalValue::Unsigned(u64::from(outcome.held_out_transfer_cases)),
        CanonicalValue::Unsigned(outcome.candidate_work_without),
        CanonicalValue::Unsigned(outcome.candidate_work_with),
        CanonicalValue::Unsigned(u64::from(outcome.examples_without)),
        CanonicalValue::Unsigned(u64::from(outcome.examples_with)),
        CanonicalValue::Unsigned(u64::from(outcome.retained_prior_tasks)),
        CanonicalValue::Unsigned(u64::from(outcome.prior_tasks)),
        CanonicalValue::Unsigned(u64::from(outcome.negative_transfers)),
    ])
}

fn curriculum_event_value(event: &CurriculumEvent) -> CanonicalValue {
    match event {
        CurriculumEvent::ObjectiveAdded { objective } => CanonicalValue::Array(vec![
            CanonicalValue::Unsigned(2),
            curriculum_objective_value(objective),
            CanonicalValue::Null,
        ]),
        CurriculumEvent::Learned { decision, outcome } => CanonicalValue::Array(vec![
            CanonicalValue::Unsigned(0),
            curriculum_decision_value(decision),
            curriculum_outcome_value(outcome),
        ]),
        CurriculumEvent::Counterexample { target, evidence } => CanonicalValue::Array(vec![
            CanonicalValue::Unsigned(1),
            CanonicalValue::Text(target.0.clone()),
            CanonicalValue::Text(evidence.clone()),
        ]),
    }
}

fn curriculum_checkpoint_from_value(
    value: CanonicalValue,
) -> Result<CurriculumCheckpoint, CurriculumCheckpointArchiveError> {
    let values = checkpoint_array(value, 6, "checkpoint field count")?;
    if checkpoint_text(&values[0])? != "cairn.curriculum.checkpoint.v1" {
        return Err(CurriculumCheckpointArchiveError::InvalidPayload(
            "checkpoint marker mismatch",
        ));
    }
    let objectives = checkpoint_values(&values[3])?
        .iter()
        .cloned()
        .map(curriculum_objective_from_value)
        .collect::<Result<Vec<_>, _>>()?;
    let events = checkpoint_values(&values[4])?
        .iter()
        .cloned()
        .map(curriculum_event_from_value)
        .collect::<Result<Vec<_>, _>>()?;
    let outstanding = match &values[5] {
        CanonicalValue::Null => None,
        value => Some(curriculum_decision_from_value(value.clone())?),
    };
    Ok(CurriculumCheckpoint {
        semantics_version: checkpoint_text(&values[1])?.to_owned(),
        maximum_decisions: checkpoint_u64(&values[2])?,
        objectives,
        events,
        outstanding,
    })
}

fn curriculum_objective_from_value(
    value: CanonicalValue,
) -> Result<DevelopmentalObjective, CurriculumCheckpointArchiveError> {
    let values = checkpoint_array(value, 8, "objective field count")?;
    let prerequisites = checkpoint_values(&values[3])?
        .iter()
        .map(|value| checkpoint_text(value).map(|value| MechanismTarget(value.to_owned())))
        .collect::<Result<BTreeSet<_>, _>>()?;
    Ok(DevelopmentalObjective {
        id: ExperienceId(checkpoint_text(&values[0])?.to_owned()),
        axis: learning_axis_from_code(checkpoint_u64(&values[1])?)?,
        produces: MechanismTarget(checkpoint_text(&values[2])?.to_owned()),
        prerequisites,
        competing_hypotheses: checkpoint_u64(&values[4])?,
        expected_transfer_microunits: checkpoint_u64(&values[5])?,
        action_cost: checkpoint_u64(&values[6])?,
        irreversible_risk: checkpoint_u64(&values[7])?,
    })
}

fn curriculum_decision_from_value(
    value: CanonicalValue,
) -> Result<CurriculumDecision, CurriculumCheckpointArchiveError> {
    let values = checkpoint_array(value, 7, "decision field count")?;
    Ok(CurriculumDecision {
        sequence: checkpoint_u64(&values[0])?,
        objective: ExperienceId(checkpoint_text(&values[1])?.to_owned()),
        axis: learning_axis_from_code(checkpoint_u64(&values[2])?)?,
        produces: MechanismTarget(checkpoint_text(&values[3])?.to_owned()),
        expected_information_microunits: checkpoint_u64(&values[4])?,
        expected_transfer_microunits: checkpoint_u64(&values[5])?,
        underuse_bonus: checkpoint_u64(&values[6])?,
    })
}

fn curriculum_outcome_from_value(
    value: CanonicalValue,
) -> Result<DevelopmentalOutcome, CurriculumCheckpointArchiveError> {
    let values = checkpoint_array(value, 12, "outcome field count")?;
    Ok(DevelopmentalOutcome {
        decision_sequence: checkpoint_u64(&values[0])?,
        objective: ExperienceId(checkpoint_text(&values[1])?.to_owned()),
        mechanism_evidence: checkpoint_text(&values[2])?.to_owned(),
        independently_verified: checkpoint_bool(&values[3])?,
        held_out_transfer_cases: checkpoint_u32(&values[4])?,
        candidate_work_without: checkpoint_u64(&values[5])?,
        candidate_work_with: checkpoint_u64(&values[6])?,
        examples_without: checkpoint_u32(&values[7])?,
        examples_with: checkpoint_u32(&values[8])?,
        retained_prior_tasks: checkpoint_u32(&values[9])?,
        prior_tasks: checkpoint_u32(&values[10])?,
        negative_transfers: checkpoint_u32(&values[11])?,
    })
}

fn curriculum_event_from_value(
    value: CanonicalValue,
) -> Result<CurriculumEvent, CurriculumCheckpointArchiveError> {
    let values = checkpoint_array(value, 3, "event field count")?;
    match checkpoint_u64(&values[0])? {
        0 => Ok(CurriculumEvent::Learned {
            decision: curriculum_decision_from_value(values[1].clone())?,
            outcome: curriculum_outcome_from_value(values[2].clone())?,
        }),
        1 => Ok(CurriculumEvent::Counterexample {
            target: MechanismTarget(checkpoint_text(&values[1])?.to_owned()),
            evidence: checkpoint_text(&values[2])?.to_owned(),
        }),
        2 => {
            if values[2] != CanonicalValue::Null {
                return Err(CurriculumCheckpointArchiveError::InvalidPayload(
                    "objective event reserved field is not null",
                ));
            }
            Ok(CurriculumEvent::ObjectiveAdded {
                objective: curriculum_objective_from_value(values[1].clone())?,
            })
        }
        _ => Err(CurriculumCheckpointArchiveError::InvalidPayload(
            "unknown event code",
        )),
    }
}

fn checkpoint_array(
    value: CanonicalValue,
    expected: usize,
    error: &'static str,
) -> Result<Vec<CanonicalValue>, CurriculumCheckpointArchiveError> {
    let CanonicalValue::Array(values) = value else {
        return Err(CurriculumCheckpointArchiveError::InvalidPayload(error));
    };
    if values.len() != expected {
        return Err(CurriculumCheckpointArchiveError::InvalidPayload(error));
    }
    Ok(values)
}

fn checkpoint_values(
    value: &CanonicalValue,
) -> Result<&[CanonicalValue], CurriculumCheckpointArchiveError> {
    let CanonicalValue::Array(values) = value else {
        return Err(CurriculumCheckpointArchiveError::InvalidPayload(
            "value is not an array",
        ));
    };
    Ok(values)
}

fn checkpoint_text(value: &CanonicalValue) -> Result<&str, CurriculumCheckpointArchiveError> {
    let CanonicalValue::Text(value) = value else {
        return Err(CurriculumCheckpointArchiveError::InvalidPayload(
            "value is not text",
        ));
    };
    Ok(value)
}

fn checkpoint_u64(value: &CanonicalValue) -> Result<u64, CurriculumCheckpointArchiveError> {
    let CanonicalValue::Unsigned(value) = value else {
        return Err(CurriculumCheckpointArchiveError::InvalidPayload(
            "value is not unsigned",
        ));
    };
    Ok(*value)
}

fn checkpoint_u32(value: &CanonicalValue) -> Result<u32, CurriculumCheckpointArchiveError> {
    u32::try_from(checkpoint_u64(value)?)
        .map_err(|_| CurriculumCheckpointArchiveError::InvalidPayload("unsigned value exceeds u32"))
}

fn checkpoint_bool(value: &CanonicalValue) -> Result<bool, CurriculumCheckpointArchiveError> {
    let CanonicalValue::Bool(value) = value else {
        return Err(CurriculumCheckpointArchiveError::InvalidPayload(
            "value is not Boolean",
        ));
    };
    Ok(*value)
}

const fn learning_axis_code(axis: LearningAxis) -> u64 {
    match axis {
        LearningAxis::CausalLaw => 0,
        LearningAxis::Cardinality => 1,
        LearningAxis::Value => 2,
        LearningAxis::Layout => 3,
        LearningAxis::Composition => 4,
    }
}

fn learning_axis_from_code(code: u64) -> Result<LearningAxis, CurriculumCheckpointArchiveError> {
    match code {
        0 => Ok(LearningAxis::CausalLaw),
        1 => Ok(LearningAxis::Cardinality),
        2 => Ok(LearningAxis::Value),
        3 => Ok(LearningAxis::Layout),
        4 => Ok(LearningAxis::Composition),
        _ => Err(CurriculumCheckpointArchiveError::InvalidPayload(
            "unknown learning axis",
        )),
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
