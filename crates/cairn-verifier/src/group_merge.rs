use cairn_core::{EntityId, Hash32, RelationRole};
use cairn_rkl::{
    ExecLimits, ReferenceExecutor, RuntimeEnvironment, TraceLevel, TypeChecker, TypeEnvironment,
    Value, VarName,
};
use cairn_theory::{
    GroupMergeTrace, GroupingHypothesisTrace, LearnedGroupMerge, LearnedGroupingMerge,
    RawGroupingEvidence, RawGroupingLimits, RawGroupingTrace,
};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedGroupMerge {
    mechanism: Hash32,
    experiences: BTreeSet<Hash32>,
    replayed_groups: u64,
    group_counts: BTreeSet<usize>,
    group_cardinalities: BTreeSet<usize>,
}

impl VerifiedGroupMerge {
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
    pub const fn group_counts(&self) -> &BTreeSet<usize> {
        &self.group_counts
    }
    #[must_use]
    pub const fn group_cardinalities(&self) -> &BTreeSet<usize> {
        &self.group_cardinalities
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GroupMergeVerifyError {
    InsufficientEvidence,
    DuplicateExperience,
    InvalidTrace,
    Definition,
    ReplayMismatch,
    Capacity,
    Canonicalization,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct GroupMergeVerifier;

impl GroupMergeVerifier {
    /// Independently recompute group aggregates and execute raw RKL.
    ///
    /// # Errors
    ///
    /// Rejects malformed or duplicate evidence, overflow, definition failure,
    /// and exact output-set mismatch.
    pub fn verify(
        &self,
        learned: &LearnedGroupMerge,
        traces: &[GroupMergeTrace],
        limits: ExecLimits,
    ) -> Result<VerifiedGroupMerge, GroupMergeVerifyError> {
        if traces.len() < 2 {
            return Err(GroupMergeVerifyError::InsufficientEvidence);
        }
        let definition = &learned.constructor.proposal.definition;
        let mut types = TypeEnvironment::new();
        for parameter in &definition.parameters {
            types.insert(parameter.name.clone(), parameter.ty.clone());
        }
        let checked = TypeChecker::new(10_000)
            .check(&definition.body, &types)
            .map_err(|_| GroupMergeVerifyError::Definition)?;
        let executor = ReferenceExecutor::without_definitions(limits, TraceLevel::Off);
        let mut receipt = VerifiedGroupMerge {
            mechanism: learned
                .semantic_hash()
                .map_err(|_| GroupMergeVerifyError::Canonicalization)?,
            experiences: BTreeSet::new(),
            replayed_groups: 0,
            group_counts: BTreeSet::new(),
            group_cardinalities: BTreeSet::new(),
        };
        for trace in traces {
            if trace.groups.is_empty() || trace.outputs.len() != trace.groups.len() {
                return Err(GroupMergeVerifyError::InvalidTrace);
            }
            if !receipt.experiences.insert(trace.episode) {
                return Err(GroupMergeVerifyError::DuplicateExperience);
            }
            let mut actual = BTreeSet::new();
            for group in &trace.groups {
                let minimum = group
                    .first()
                    .copied()
                    .ok_or(GroupMergeVerifyError::InvalidTrace)?;
                let maximum = group
                    .last()
                    .copied()
                    .ok_or(GroupMergeVerifyError::InvalidTrace)?;
                let sum = group
                    .iter()
                    .try_fold(0_i64, |sum, value| sum.checked_add(*value))
                    .ok_or(GroupMergeVerifyError::Capacity)?;
                let count =
                    i64::try_from(group.len()).map_err(|_| GroupMergeVerifyError::Capacity)?;
                let mut environment = RuntimeEnvironment::new();
                for (name, value) in [
                    ("group_count", count),
                    ("group_sum", sum),
                    ("group_minimum", minimum),
                    ("group_maximum", maximum),
                ] {
                    environment.insert(VarName(name.to_owned()), Value::Int(value));
                }
                match executor
                    .execute(&checked, &environment)
                    .map_err(|_| GroupMergeVerifyError::Definition)?
                    .result
                {
                    Value::Int(value) => {
                        actual.insert(value);
                    }
                    _ => return Err(GroupMergeVerifyError::Definition),
                }
                receipt.group_cardinalities.insert(group.len());
            }
            if actual != trace.outputs {
                return Err(GroupMergeVerifyError::ReplayMismatch);
            }
            receipt.replayed_groups = receipt
                .replayed_groups
                .checked_add(
                    u64::try_from(trace.groups.len())
                        .map_err(|_| GroupMergeVerifyError::Capacity)?,
                )
                .ok_or(GroupMergeVerifyError::Capacity)?;
            receipt.group_counts.insert(trace.groups.len());
        }
        Ok(receipt)
    }
}

/// Opaque verifier evidence that one relation-bound merge replayed exactly.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedGroupingReuse {
    mechanism: Hash32,
    experiences: BTreeSet<Hash32>,
    replayed_groups: u64,
    candidate_executions_without: u64,
    candidate_executions_with: u64,
}

impl VerifiedGroupingReuse {
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

/// Consolidator-issued authority to promote one relation-bound mechanism.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedGroupingPromotion {
    mechanism: Hash32,
    experiences: BTreeSet<Hash32>,
    verified_reuses: u32,
}

impl VerifiedGroupingPromotion {
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
pub enum GroupingPromotionError {
    Verification(GroupMergeVerifyError),
    MissingRelation,
    NoLearningReduction,
    InsufficientIndependentEvidence,
    MixedMechanism,
    OverlappingExperience,
    Capacity,
    Canonicalization,
    InvalidRawEvidence,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct GroupingMergeVerifier;

impl GroupingMergeVerifier {
    /// Independently lower the selected relation and replay its raw RKL body.
    ///
    /// # Errors
    ///
    /// Rejects missing relation evidence, non-positive counterfactual
    /// reduction, malformed traces, or an exact replay mismatch.
    pub fn verify_reuse(
        &self,
        learned: &LearnedGroupingMerge,
        traces: &[GroupingHypothesisTrace],
        candidate_executions_without: u64,
        candidate_executions_with: u64,
        limits: ExecLimits,
    ) -> Result<VerifiedGroupingReuse, GroupingPromotionError> {
        if candidate_executions_without == 0
            || candidate_executions_with >= candidate_executions_without
        {
            return Err(GroupingPromotionError::NoLearningReduction);
        }
        let lowered = traces
            .iter()
            .map(|trace| {
                let groups = trace
                    .candidates
                    .get(&learned.relation)
                    .ok_or(GroupingPromotionError::MissingRelation)?;
                Ok(GroupMergeTrace {
                    episode: trace.episode,
                    groups: groups.clone(),
                    outputs: trace.outputs.clone(),
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let replay = GroupMergeVerifier
            .verify(&learned.merge, &lowered, limits)
            .map_err(GroupingPromotionError::Verification)?;
        Ok(VerifiedGroupingReuse {
            mechanism: learned
                .semantic_hash()
                .map_err(|_| GroupingPromotionError::Canonicalization)?,
            experiences: replay.experiences().clone(),
            replayed_groups: replay.replayed_groups(),
            candidate_executions_without,
            candidate_executions_with,
        })
    }

    /// Independently construct selected connected components from raw edges
    /// and replay the relation-bound RKL merge.
    ///
    /// # Errors
    ///
    /// Rejects malformed/capped raw evidence, absent selected-role edges,
    /// non-positive counterfactual reduction, or exact replay mismatch.
    pub fn verify_raw_reuse(
        &self,
        learned: &LearnedGroupingMerge,
        traces: &[RawGroupingTrace],
        candidate_executions_without: u64,
        candidate_executions_with: u64,
        raw_limits: RawGroupingLimits,
        execution_limits: ExecLimits,
    ) -> Result<VerifiedGroupingReuse, GroupingPromotionError> {
        if candidate_executions_without == 0
            || candidate_executions_with >= candidate_executions_without
        {
            return Err(GroupingPromotionError::NoLearningReduction);
        }
        let lowered = traces
            .iter()
            .map(|trace| {
                Ok(GroupMergeTrace {
                    episode: trace.episode,
                    groups: verifier_selected_groups(
                        &trace.evidence,
                        &learned.relation,
                        raw_limits,
                    )?,
                    outputs: trace.outputs.clone(),
                })
            })
            .collect::<Result<Vec<_>, GroupingPromotionError>>()?;
        let replay = GroupMergeVerifier
            .verify(&learned.merge, &lowered, execution_limits)
            .map_err(GroupingPromotionError::Verification)?;
        Ok(VerifiedGroupingReuse {
            mechanism: learned
                .semantic_hash()
                .map_err(|_| GroupingPromotionError::Canonicalization)?,
            experiences: replay.experiences().clone(),
            replayed_groups: replay.replayed_groups(),
            candidate_executions_without,
            candidate_executions_with,
        })
    }
}

/// Separate consolidation authority for relation-bound reuse evidence.
#[derive(Clone, Copy, Debug, Default)]
pub struct GroupingPromotionConsolidator;

impl GroupingPromotionConsolidator {
    /// Combine two or more disjoint verifier receipts into one promotion.
    ///
    /// # Errors
    ///
    /// Rejects mixed identities, overlapping evidence, insufficient receipts,
    /// or any receipt lacking strict candidate-execution reduction.
    pub fn consolidate(
        &self,
        learned: &LearnedGroupingMerge,
        receipts: &[VerifiedGroupingReuse],
    ) -> Result<VerifiedGroupingPromotion, GroupingPromotionError> {
        if receipts.len() < 2 {
            return Err(GroupingPromotionError::InsufficientIndependentEvidence);
        }
        let mechanism = learned
            .semantic_hash()
            .map_err(|_| GroupingPromotionError::Canonicalization)?;
        let mut experiences = BTreeSet::new();
        for receipt in receipts {
            if receipt.mechanism != mechanism {
                return Err(GroupingPromotionError::MixedMechanism);
            }
            if receipt.candidate_executions_without == 0
                || receipt.candidate_executions_with >= receipt.candidate_executions_without
            {
                return Err(GroupingPromotionError::NoLearningReduction);
            }
            if !experiences.is_disjoint(&receipt.experiences) {
                return Err(GroupingPromotionError::OverlappingExperience);
            }
            experiences.extend(&receipt.experiences);
        }
        Ok(VerifiedGroupingPromotion {
            mechanism,
            experiences,
            verified_reuses: u32::try_from(receipts.len())
                .map_err(|_| GroupingPromotionError::Capacity)?,
        })
    }
}

fn verifier_selected_groups(
    evidence: &RawGroupingEvidence,
    selected: &RelationRole,
    limits: RawGroupingLimits,
) -> Result<Vec<BTreeSet<i64>>, GroupingPromotionError> {
    if evidence.entities.is_empty()
        || evidence.relations.is_empty()
        || evidence.entities.len() > limits.maximum_entities
        || evidence.relations.len() > limits.maximum_relations
    {
        return Err(GroupingPromotionError::InvalidRawEvidence);
    }
    let mut roles = BTreeSet::new();
    let mut unique = BTreeSet::new();
    let mut adjacency = evidence
        .entities
        .keys()
        .cloned()
        .map(|entity| (entity, BTreeSet::new()))
        .collect::<std::collections::BTreeMap<EntityId, BTreeSet<EntityId>>>();
    for relation in &evidence.relations {
        if !evidence.entities.contains_key(&relation.subject)
            || !evidence.entities.contains_key(&relation.object)
            || !unique.insert((
                relation.role.clone(),
                relation.subject.clone(),
                relation.object.clone(),
            ))
        {
            return Err(GroupingPromotionError::InvalidRawEvidence);
        }
        roles.insert(relation.role.clone());
        if &relation.role == selected {
            adjacency
                .get_mut(&relation.subject)
                .ok_or(GroupingPromotionError::InvalidRawEvidence)?
                .insert(relation.object.clone());
            adjacency
                .get_mut(&relation.object)
                .ok_or(GroupingPromotionError::InvalidRawEvidence)?
                .insert(relation.subject.clone());
        }
    }
    if roles.len() > limits.maximum_roles || !roles.contains(selected) {
        return Err(GroupingPromotionError::InvalidRawEvidence);
    }
    let mut unseen = evidence.entities.keys().cloned().collect::<BTreeSet<_>>();
    let mut groups = Vec::new();
    while let Some(start) = unseen.first().cloned() {
        let mut frontier = BTreeSet::from([start]);
        let mut values = BTreeSet::new();
        while let Some(entity) = frontier.pop_first() {
            if !unseen.remove(&entity) {
                continue;
            }
            values.insert(evidence.entities[&entity]);
            frontier.extend(
                adjacency[&entity]
                    .iter()
                    .filter(|neighbor| unseen.contains(*neighbor))
                    .cloned(),
            );
        }
        groups.push(values);
    }
    Ok(groups)
}
