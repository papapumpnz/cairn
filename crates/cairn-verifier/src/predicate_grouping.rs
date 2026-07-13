use crate::{GroupMergeVerifier, GroupMergeVerifyError};
use cairn_core::Hash32;
use cairn_rkl::{
    ExecLimits, ReferenceExecutor, RuntimeEnvironment, TraceLevel, TypeChecker, TypeEnvironment,
    Value,
};
use cairn_theory::{
    AttributedGroupingEntity, AttributedGroupingTrace, GroupMergeTrace,
    LearnedPredicateGroupingMerge, PredicateGroupingLimits,
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedPredicateGrouping {
    mechanism: Hash32,
    experiences: BTreeSet<Hash32>,
    replayed_groups: u64,
}

/// Opaque evidence that a learned predicate/merge reduced bounded search and
/// independently replayed one disjoint cohort.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedPredicateGroupingReuse {
    mechanism: Hash32,
    experiences: BTreeSet<Hash32>,
    replayed_groups: u64,
    candidate_executions_without: u64,
    candidate_executions_with: u64,
}

impl VerifiedPredicateGroupingReuse {
    #[must_use]
    pub const fn mechanism(&self) -> Hash32 {
        self.mechanism
    }
    #[must_use]
    pub const fn experiences(&self) -> &BTreeSet<Hash32> {
        &self.experiences
    }
    #[must_use]
    pub const fn replayed_groups(&self) -> u64 {
        self.replayed_groups
    }
    #[must_use]
    pub const fn candidate_executions_without(&self) -> u64 {
        self.candidate_executions_without
    }
    #[must_use]
    pub const fn candidate_executions_with(&self) -> u64 {
        self.candidate_executions_with
    }
}

/// Consolidator authority covering the complete disjoint predicate-reuse
/// ledger of one mechanism.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedPredicateGroupingPromotion {
    mechanism: Hash32,
    experiences: BTreeSet<Hash32>,
    verified_reuses: u32,
}

impl VerifiedPredicateGroupingPromotion {
    #[must_use]
    pub const fn mechanism(&self) -> Hash32 {
        self.mechanism
    }
    #[must_use]
    pub const fn experiences(&self) -> &BTreeSet<Hash32> {
        &self.experiences
    }
    #[must_use]
    pub const fn verified_reuses(&self) -> u32 {
        self.verified_reuses
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PredicateGroupingPromotionError {
    Verification(PredicateGroupingVerifyError),
    NoLearningReduction,
    InsufficientIndependentEvidence,
    MixedMechanism,
    OverlappingExperience,
    Capacity,
    Canonicalization,
}

impl VerifiedPredicateGrouping {
    #[must_use]
    pub const fn mechanism(&self) -> Hash32 {
        self.mechanism
    }
    #[must_use]
    pub const fn experiences(&self) -> &BTreeSet<Hash32> {
        &self.experiences
    }
    #[must_use]
    pub const fn replayed_groups(&self) -> u64 {
        self.replayed_groups
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PredicateGroupingVerifyError {
    InsufficientEvidence,
    DuplicateExperience,
    InvalidEntities,
    Capacity,
    RelationDefinition,
    RelationExecution,
    Merge(GroupMergeVerifyError),
    Canonicalization,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct PredicateGroupingVerifier;

impl PredicateGroupingVerifier {
    /// Independently execute the raw RKL pair predicate, reconstruct complete
    /// components, and replay the merge definition against every output.
    ///
    /// # Errors
    ///
    /// Rejects malformed/capped evidence, duplicate experience, definition or
    /// execution failure, and exact output mismatch.
    pub fn verify(
        &self,
        learned: &LearnedPredicateGroupingMerge,
        traces: &[AttributedGroupingTrace],
        limits: PredicateGroupingLimits,
        execution_limits: ExecLimits,
    ) -> Result<VerifiedPredicateGrouping, PredicateGroupingVerifyError> {
        if traces.len() < 2 {
            return Err(PredicateGroupingVerifyError::InsufficientEvidence);
        }
        let experiences = traces
            .iter()
            .map(|trace| trace.episode)
            .collect::<BTreeSet<_>>();
        if experiences.len() != traces.len() {
            return Err(PredicateGroupingVerifyError::DuplicateExperience);
        }
        let merge_traces = traces
            .iter()
            .map(|trace| {
                Ok(GroupMergeTrace {
                    episode: trace.episode,
                    groups: verifier_groups(
                        &learned.relation,
                        &trace.entities,
                        limits,
                        execution_limits,
                    )?,
                    outputs: trace.outputs.clone(),
                })
            })
            .collect::<Result<Vec<_>, PredicateGroupingVerifyError>>()?;
        let merge = GroupMergeVerifier
            .verify(&learned.merge, &merge_traces, execution_limits)
            .map_err(PredicateGroupingVerifyError::Merge)?;
        Ok(VerifiedPredicateGrouping {
            mechanism: learned
                .semantic_hash()
                .map_err(|_| PredicateGroupingVerifyError::Canonicalization)?,
            experiences,
            replayed_groups: merge.replayed_groups(),
        })
    }

    /// Replay one cohort and bind its exact receipt to measured search
    /// reduction. This receipt has no promotion authority by itself.
    ///
    /// # Errors
    ///
    /// Rejects absent reduction or any ordinary independent replay failure.
    pub fn verify_reuse(
        &self,
        learned: &LearnedPredicateGroupingMerge,
        traces: &[AttributedGroupingTrace],
        limits: PredicateGroupingLimits,
        execution_limits: ExecLimits,
        candidate_executions_without: u64,
        candidate_executions_with: u64,
    ) -> Result<VerifiedPredicateGroupingReuse, PredicateGroupingPromotionError> {
        if candidate_executions_without == 0
            || candidate_executions_with >= candidate_executions_without
        {
            return Err(PredicateGroupingPromotionError::NoLearningReduction);
        }
        let replay = self
            .verify(learned, traces, limits, execution_limits)
            .map_err(PredicateGroupingPromotionError::Verification)?;
        Ok(VerifiedPredicateGroupingReuse {
            mechanism: replay.mechanism,
            experiences: replay.experiences,
            replayed_groups: replay.replayed_groups,
            candidate_executions_without,
            candidate_executions_with,
        })
    }
}

/// Separate consolidation authority for independently verified predicate
/// grouping cohorts.
#[derive(Clone, Copy, Debug, Default)]
pub struct PredicateGroupingPromotionConsolidator;

impl PredicateGroupingPromotionConsolidator {
    /// Cover two or more disjoint reuse receipts with one promotion token.
    ///
    /// # Errors
    ///
    /// Rejects mixed mechanisms, overlapping evidence, insufficient receipts,
    /// absent reduction, overflow, or canonicalization failure.
    pub fn consolidate(
        &self,
        learned: &LearnedPredicateGroupingMerge,
        receipts: &[VerifiedPredicateGroupingReuse],
    ) -> Result<VerifiedPredicateGroupingPromotion, PredicateGroupingPromotionError> {
        if receipts.len() < 2 {
            return Err(PredicateGroupingPromotionError::InsufficientIndependentEvidence);
        }
        let mechanism = learned
            .semantic_hash()
            .map_err(|_| PredicateGroupingPromotionError::Canonicalization)?;
        let mut experiences = BTreeSet::new();
        for receipt in receipts {
            if receipt.mechanism != mechanism {
                return Err(PredicateGroupingPromotionError::MixedMechanism);
            }
            if receipt.candidate_executions_without == 0
                || receipt.candidate_executions_with >= receipt.candidate_executions_without
            {
                return Err(PredicateGroupingPromotionError::NoLearningReduction);
            }
            if !experiences.is_disjoint(&receipt.experiences) {
                return Err(PredicateGroupingPromotionError::OverlappingExperience);
            }
            experiences.extend(&receipt.experiences);
        }
        Ok(VerifiedPredicateGroupingPromotion {
            mechanism,
            experiences,
            verified_reuses: u32::try_from(receipts.len())
                .map_err(|_| PredicateGroupingPromotionError::Capacity)?,
        })
    }
}

fn verifier_groups(
    definition: &cairn_rkl::Definition,
    entities: &[AttributedGroupingEntity],
    limits: PredicateGroupingLimits,
    execution_limits: ExecLimits,
) -> Result<Vec<BTreeSet<i64>>, PredicateGroupingVerifyError> {
    if entities.len() < 2
        || entities.len() > limits.maximum_entities
        || definition.parameters.is_empty()
        || definition.parameters.len() % 2 != 0
    {
        return Err(PredicateGroupingVerifyError::InvalidEntities);
    }
    let feature_count = definition.parameters.len() / 2;
    if feature_count > limits.maximum_features
        || entities
            .iter()
            .any(|entity| entity.features.len() != feature_count)
    {
        return Err(PredicateGroupingVerifyError::Capacity);
    }
    let ids = entities
        .iter()
        .map(|entity| &entity.id)
        .collect::<BTreeSet<_>>();
    if ids.len() != entities.len() {
        return Err(PredicateGroupingVerifyError::InvalidEntities);
    }
    let mut type_environment = TypeEnvironment::new();
    for parameter in &definition.parameters {
        type_environment.insert(parameter.name.clone(), parameter.ty.clone());
    }
    let typed_expression = TypeChecker::new(10_000)
        .check(&definition.body, &type_environment)
        .map_err(|_| PredicateGroupingVerifyError::RelationDefinition)?;
    let executor = ReferenceExecutor::without_definitions(execution_limits, TraceLevel::Off);
    let mut adjacency = (0..entities.len())
        .map(|index| (index, BTreeSet::new()))
        .collect::<BTreeMap<_, _>>();
    for left in 0..entities.len() {
        for right in (left + 1)..entities.len() {
            let related = verifier_pair_relation(
                &executor,
                &typed_expression,
                definition,
                &entities[left],
                &entities[right],
                feature_count,
            )? || verifier_pair_relation(
                &executor,
                &typed_expression,
                definition,
                &entities[right],
                &entities[left],
                feature_count,
            )?;
            if related {
                adjacency
                    .get_mut(&left)
                    .ok_or(PredicateGroupingVerifyError::RelationExecution)?
                    .insert(right);
                adjacency
                    .get_mut(&right)
                    .ok_or(PredicateGroupingVerifyError::RelationExecution)?
                    .insert(left);
            }
        }
    }
    let mut unseen = (0..entities.len()).collect::<BTreeSet<_>>();
    let mut groups = Vec::new();
    while let Some(start) = unseen.pop_first() {
        let mut frontier = BTreeSet::from([start]);
        let mut values = BTreeSet::new();
        while let Some(entity) = frontier.pop_first() {
            values.insert(entities[entity].value);
            frontier.extend(
                adjacency[&entity]
                    .iter()
                    .filter(|neighbor| unseen.remove(*neighbor))
                    .copied(),
            );
        }
        groups.push(values);
    }
    Ok(groups)
}

fn verifier_pair_relation(
    executor: &ReferenceExecutor,
    expression: &cairn_rkl::TypedExpr,
    definition: &cairn_rkl::Definition,
    left: &AttributedGroupingEntity,
    right: &AttributedGroupingEntity,
    feature_count: usize,
) -> Result<bool, PredicateGroupingVerifyError> {
    let mut environment = RuntimeEnvironment::new();
    for feature in 0..feature_count {
        environment.insert(
            definition.parameters[feature].name.clone(),
            Value::Int(left.features[feature]),
        );
        environment.insert(
            definition.parameters[feature_count + feature].name.clone(),
            Value::Int(right.features[feature]),
        );
    }
    match executor
        .execute(expression, &environment)
        .map_err(|_| PredicateGroupingVerifyError::RelationExecution)?
        .result
    {
        Value::Bool(related) => Ok(related),
        _ => Err(PredicateGroupingVerifyError::RelationExecution),
    }
}
