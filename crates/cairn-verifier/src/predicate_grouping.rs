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
            let mut environment = RuntimeEnvironment::new();
            for feature in 0..feature_count {
                environment.insert(
                    definition.parameters[feature].name.clone(),
                    Value::Int(entities[left].features[feature]),
                );
                environment.insert(
                    definition.parameters[feature_count + feature].name.clone(),
                    Value::Int(entities[right].features[feature]),
                );
            }
            let Value::Bool(related) = executor
                .execute(&typed_expression, &environment)
                .map_err(|_| PredicateGroupingVerifyError::RelationExecution)?
                .result
            else {
                return Err(PredicateGroupingVerifyError::RelationExecution);
            };
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
