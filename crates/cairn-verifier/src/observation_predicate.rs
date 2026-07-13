use crate::{PredicateGroupingVerifier, PredicateGroupingVerifyError};
use cairn_core::Hash32;
use cairn_rkl::{
    ExecLimits, ReferenceExecutor, RuntimeEnvironment, TraceLevel, TypeChecker, TypeEnvironment,
    Value,
};
use cairn_theory::{
    AttributedGroupingEntity, AttributedGroupingTrace, LearnedObservationPredicateGrouping,
    ObservationPredicateLimits, PredicateGroupingLimits, PrimitivePairEntity, PrimitivePairTrace,
};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedObservationPredicateGrouping {
    mechanism: Hash32,
    experiences: BTreeSet<Hash32>,
    replayed_groups: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedObservationPredicateReuse {
    mechanism: Hash32,
    experiences: BTreeSet<Hash32>,
    replayed_groups: u64,
    candidate_executions_without: u64,
    candidate_executions_with: u64,
    support: ObservationPredicateSupportGeometry,
}

/// Exact tested boundary carried by observation-feature reuse evidence. It is
/// descriptive evidence, not permission to extrapolate correctness.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ObservationPredicateSupportGeometry {
    pub observation_arity: usize,
    pub minimum_entities: usize,
    pub maximum_entities: usize,
    pub minimum_observation: i64,
    pub maximum_observation: i64,
}

impl ObservationPredicateSupportGeometry {
    #[must_use]
    pub fn structurally_applies(&self, entities: &[PrimitivePairEntity]) -> bool {
        !entities.is_empty()
            && entities
                .iter()
                .all(|entity| entity.observations.len() == self.observation_arity)
    }
}

impl VerifiedObservationPredicateReuse {
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
    #[must_use]
    pub const fn support(&self) -> ObservationPredicateSupportGeometry {
        self.support
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedObservationPredicatePromotion {
    mechanism: Hash32,
    experiences: BTreeSet<Hash32>,
    verified_reuses: u32,
    support: ObservationPredicateSupportGeometry,
}

impl VerifiedObservationPredicatePromotion {
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
    #[must_use]
    pub const fn support(&self) -> ObservationPredicateSupportGeometry {
        self.support
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ObservationPredicatePromotionError {
    Verification(ObservationPredicateVerifyError),
    NoLearningReduction,
    InsufficientIndependentEvidence,
    MixedMechanism,
    OverlappingExperience,
    Capacity,
    Canonicalization,
}

impl VerifiedObservationPredicateGrouping {
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
pub enum ObservationPredicateVerifyError {
    InsufficientEvidence,
    DuplicateExperience,
    InvalidEntities,
    Capacity,
    FeatureDefinition,
    FeatureExecution,
    Grouping(PredicateGroupingVerifyError),
    Canonicalization,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ObservationPredicateVerifier;

impl ObservationPredicateVerifier {
    /// Independently execute learned observation-feature RKL, construct fresh
    /// attributed evidence, and invoke independent relation/aggregate replay.
    ///
    /// # Errors
    ///
    /// Rejects malformed/capped evidence, feature definition or execution
    /// failure, downstream replay failure, or canonicalization failure.
    pub fn verify(
        &self,
        learned: &LearnedObservationPredicateGrouping,
        traces: &[PrimitivePairTrace],
        limits: ObservationPredicateLimits,
        predicate_limits: PredicateGroupingLimits,
        execution_limits: ExecLimits,
    ) -> Result<VerifiedObservationPredicateGrouping, ObservationPredicateVerifyError> {
        if traces.len() < 2 {
            return Err(ObservationPredicateVerifyError::InsufficientEvidence);
        }
        let experiences = traces
            .iter()
            .map(|trace| trace.episode)
            .collect::<BTreeSet<_>>();
        if experiences.len() != traces.len() {
            return Err(ObservationPredicateVerifyError::DuplicateExperience);
        }
        let attributed = traces
            .iter()
            .map(|trace| {
                Ok(AttributedGroupingTrace {
                    episode: trace.episode,
                    entities: verifier_lower_entities(
                        &trace.entities,
                        learned,
                        limits,
                        execution_limits,
                    )?,
                    outputs: trace.outputs.clone(),
                })
            })
            .collect::<Result<Vec<_>, ObservationPredicateVerifyError>>()?;
        let grouping = PredicateGroupingVerifier
            .verify(
                &learned.grouping,
                &attributed,
                predicate_limits,
                execution_limits,
            )
            .map_err(ObservationPredicateVerifyError::Grouping)?;
        for permutation in verifier_observation_permutations(
            learned.observation_arity,
            limits.maximum_observation_permutations,
        )?
        .into_iter()
        .skip(1)
        {
            let permuted = traces
                .iter()
                .map(|trace| {
                    Ok(AttributedGroupingTrace {
                        episode: trace.episode,
                        entities: verifier_lower_entities(
                            &permute_primitive_observations(&trace.entities, &permutation)?,
                            learned,
                            limits,
                            execution_limits,
                        )?,
                        outputs: trace.outputs.clone(),
                    })
                })
                .collect::<Result<Vec<_>, ObservationPredicateVerifyError>>()?;
            PredicateGroupingVerifier
                .verify(
                    &learned.grouping,
                    &permuted,
                    predicate_limits,
                    execution_limits,
                )
                .map_err(ObservationPredicateVerifyError::Grouping)?;
        }
        Ok(VerifiedObservationPredicateGrouping {
            mechanism: learned
                .semantic_hash()
                .map_err(|_| ObservationPredicateVerifyError::Canonicalization)?,
            experiences,
            replayed_groups: grouping.replayed_groups(),
        })
    }

    /// Bind independent feature/relation replay to strict candidate reduction.
    ///
    /// # Errors
    ///
    /// Rejects absent reduction or any ordinary replay failure.
    pub fn verify_reuse(
        &self,
        learned: &LearnedObservationPredicateGrouping,
        traces: &[PrimitivePairTrace],
        limits: ObservationPredicateLimits,
        predicate_limits: PredicateGroupingLimits,
        execution_limits: ExecLimits,
    ) -> Result<VerifiedObservationPredicateReuse, ObservationPredicatePromotionError> {
        let candidate_executions_without = fresh_candidate_work(learned)?;
        let candidate_executions_with = 1;
        if candidate_executions_with >= candidate_executions_without {
            return Err(ObservationPredicatePromotionError::NoLearningReduction);
        }
        let replay = self
            .verify(learned, traces, limits, predicate_limits, execution_limits)
            .map_err(ObservationPredicatePromotionError::Verification)?;
        let support = observation_support(traces)?;
        Ok(VerifiedObservationPredicateReuse {
            mechanism: replay.mechanism,
            experiences: replay.experiences,
            replayed_groups: replay.replayed_groups,
            candidate_executions_without,
            candidate_executions_with,
            support,
        })
    }
}

fn fresh_candidate_work(
    learned: &LearnedObservationPredicateGrouping,
) -> Result<u64, ObservationPredicatePromotionError> {
    learned
        .feature_candidates_evaluated
        .checked_add(
            u64::try_from(learned.feature_sets_evaluated)
                .map_err(|_| ObservationPredicatePromotionError::Capacity)?,
        )
        .and_then(|work| {
            work.checked_add(u64::try_from(learned.grouping.relation_candidates_evaluated).ok()?)
        })
        .and_then(|work| {
            work.checked_add(
                learned
                    .grouping
                    .merge
                    .constructor
                    .proposal
                    .candidates_evaluated,
            )
        })
        .ok_or(ObservationPredicatePromotionError::Capacity)
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ObservationPredicatePromotionConsolidator;

impl ObservationPredicatePromotionConsolidator {
    /// Cover two or more disjoint feature/relation reuse receipts.
    ///
    /// # Errors
    ///
    /// Rejects insufficient, mixed, overlapping, non-reducing, overflowing, or
    /// non-canonical evidence.
    pub fn consolidate(
        &self,
        learned: &LearnedObservationPredicateGrouping,
        receipts: &[VerifiedObservationPredicateReuse],
    ) -> Result<VerifiedObservationPredicatePromotion, ObservationPredicatePromotionError> {
        if receipts.len() < 2 {
            return Err(ObservationPredicatePromotionError::InsufficientIndependentEvidence);
        }
        let mechanism = learned
            .semantic_hash()
            .map_err(|_| ObservationPredicatePromotionError::Canonicalization)?;
        let mut experiences = BTreeSet::new();
        let mut support = None;
        for receipt in receipts {
            if receipt.mechanism != mechanism {
                return Err(ObservationPredicatePromotionError::MixedMechanism);
            }
            if receipt.candidate_executions_without == 0
                || receipt.candidate_executions_with >= receipt.candidate_executions_without
            {
                return Err(ObservationPredicatePromotionError::NoLearningReduction);
            }
            if !experiences.is_disjoint(&receipt.experiences) {
                return Err(ObservationPredicatePromotionError::OverlappingExperience);
            }
            support = Some(match support {
                None => receipt.support,
                Some(existing) => merge_observation_support(existing, receipt.support)?,
            });
            experiences.extend(&receipt.experiences);
        }
        Ok(VerifiedObservationPredicatePromotion {
            mechanism,
            experiences,
            verified_reuses: u32::try_from(receipts.len())
                .map_err(|_| ObservationPredicatePromotionError::Capacity)?,
            support: support
                .ok_or(ObservationPredicatePromotionError::InsufficientIndependentEvidence)?,
        })
    }
}

fn observation_support(
    traces: &[PrimitivePairTrace],
) -> Result<ObservationPredicateSupportGeometry, ObservationPredicatePromotionError> {
    let mut entities = traces.iter().flat_map(|trace| trace.entities.iter());
    let first = entities
        .next()
        .ok_or(ObservationPredicatePromotionError::InsufficientIndependentEvidence)?;
    let observation_arity = first.observations.len();
    let mut minimum_entities = usize::MAX;
    let mut maximum_entities = 0_usize;
    let mut minimum_observation = i64::MAX;
    let mut maximum_observation = i64::MIN;
    for trace in traces {
        minimum_entities = minimum_entities.min(trace.entities.len());
        maximum_entities = maximum_entities.max(trace.entities.len());
        for entity in &trace.entities {
            if entity.observations.len() != observation_arity {
                return Err(ObservationPredicatePromotionError::MixedMechanism);
            }
            for observation in &entity.observations {
                minimum_observation = minimum_observation.min(*observation);
                maximum_observation = maximum_observation.max(*observation);
            }
        }
    }
    Ok(ObservationPredicateSupportGeometry {
        observation_arity,
        minimum_entities,
        maximum_entities,
        minimum_observation,
        maximum_observation,
    })
}

fn merge_observation_support(
    left: ObservationPredicateSupportGeometry,
    right: ObservationPredicateSupportGeometry,
) -> Result<ObservationPredicateSupportGeometry, ObservationPredicatePromotionError> {
    if left.observation_arity != right.observation_arity {
        return Err(ObservationPredicatePromotionError::MixedMechanism);
    }
    Ok(ObservationPredicateSupportGeometry {
        observation_arity: left.observation_arity,
        minimum_entities: left.minimum_entities.min(right.minimum_entities),
        maximum_entities: left.maximum_entities.max(right.maximum_entities),
        minimum_observation: left.minimum_observation.min(right.minimum_observation),
        maximum_observation: left.maximum_observation.max(right.maximum_observation),
    })
}

fn verifier_lower_entities(
    entities: &[PrimitivePairEntity],
    learned: &LearnedObservationPredicateGrouping,
    limits: ObservationPredicateLimits,
    execution_limits: ExecLimits,
) -> Result<Vec<AttributedGroupingEntity>, ObservationPredicateVerifyError> {
    if entities.len() < 2 {
        return Err(ObservationPredicateVerifyError::InvalidEntities);
    }
    if entities.len() > limits.maximum_entities {
        return Err(ObservationPredicateVerifyError::Capacity);
    }
    if learned.observation_arity < 2
        || learned.observation_arity > limits.maximum_observations
        || entities
            .iter()
            .any(|entity| entity.observations.len() != learned.observation_arity)
        || learned
            .feature_definitions
            .iter()
            .any(|definition| definition.parameters.len() != learned.observation_arity)
    {
        return Err(ObservationPredicateVerifyError::InvalidEntities);
    }
    let ids = entities
        .iter()
        .map(|entity| &entity.id)
        .collect::<BTreeSet<_>>();
    if ids.len() != entities.len() {
        return Err(ObservationPredicateVerifyError::InvalidEntities);
    }
    let checked = learned
        .feature_definitions
        .iter()
        .map(|definition| {
            let mut environment = TypeEnvironment::new();
            for parameter in &definition.parameters {
                environment.insert(parameter.name.clone(), parameter.ty.clone());
            }
            TypeChecker::new(10_000)
                .check(&definition.body, &environment)
                .map_err(|_| ObservationPredicateVerifyError::FeatureDefinition)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let executor = ReferenceExecutor::without_definitions(execution_limits, TraceLevel::Off);
    entities
        .iter()
        .map(|entity| {
            let mut environment = RuntimeEnvironment::new();
            for (index, observation) in entity.observations.iter().copied().enumerate() {
                let parameter = learned
                    .feature_definitions
                    .first()
                    .and_then(|definition| definition.parameters.get(index))
                    .ok_or(ObservationPredicateVerifyError::FeatureDefinition)?;
                environment.insert(parameter.name.clone(), Value::Int(observation));
            }
            let features = checked
                .iter()
                .map(|expression| {
                    match executor
                        .execute(expression, &environment)
                        .map_err(|_| ObservationPredicateVerifyError::FeatureExecution)?
                        .result
                    {
                        Value::Int(value) => Ok(value),
                        _ => Err(ObservationPredicateVerifyError::FeatureExecution),
                    }
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(AttributedGroupingEntity {
                id: entity.id.clone(),
                value: entity.value,
                features,
            })
        })
        .collect()
}

fn permute_primitive_observations(
    entities: &[PrimitivePairEntity],
    permutation: &[usize],
) -> Result<Vec<PrimitivePairEntity>, ObservationPredicateVerifyError> {
    entities
        .iter()
        .map(|entity| {
            let observations = permutation
                .iter()
                .map(|source| {
                    entity
                        .observations
                        .get(*source)
                        .copied()
                        .ok_or(ObservationPredicateVerifyError::InvalidEntities)
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(PrimitivePairEntity {
                id: entity.id.clone(),
                value: entity.value,
                observations,
            })
        })
        .collect()
}

fn verifier_observation_permutations(
    arity: usize,
    maximum: usize,
) -> Result<Vec<Vec<usize>>, ObservationPredicateVerifyError> {
    fn extend(
        arity: usize,
        maximum: usize,
        current: &mut Vec<usize>,
        output: &mut Vec<Vec<usize>>,
    ) -> Result<(), ObservationPredicateVerifyError> {
        if current.len() == arity {
            if output.len() >= maximum {
                return Err(ObservationPredicateVerifyError::Capacity);
            }
            output.push(current.clone());
            return Ok(());
        }
        for index in 0..arity {
            if current.contains(&index) {
                continue;
            }
            current.push(index);
            extend(arity, maximum, current, output)?;
            current.pop();
        }
        Ok(())
    }
    if arity < 2 || maximum == 0 {
        return Err(ObservationPredicateVerifyError::InvalidEntities);
    }
    let mut output = Vec::new();
    extend(arity, maximum, &mut Vec::new(), &mut output)?;
    Ok(output)
}
