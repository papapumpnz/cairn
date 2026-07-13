use crate::{ObservationPredicateVerifier, ObservationPredicateVerifyError};
use cairn_core::{EntityId, Hash32, RelationRole};
use cairn_rkl::ExecLimits;
use cairn_theory::{
    EntityObservationLimits, LearnedEntityObservationGrouping, ObservationPredicateLimits,
    PredicateGroupingLimits, PrimitiveObservationEntity, PrimitiveObservationTrace,
    RawEntityObservationEvidence, RawEntityObservationTrace,
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedEntityObservationGrouping {
    mechanism: Hash32,
    experiences: BTreeSet<Hash32>,
    replayed_groups: u64,
}

impl VerifiedEntityObservationGrouping {
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
pub enum EntityObservationVerifyError {
    InsufficientEvidence,
    DuplicateEpisode,
    BindingMismatch,
    InvalidEvidence,
    InvalidInterpretation,
    Capacity,
    Canonicalization,
    Observation(ObservationPredicateVerifyError),
}

/// Independent reconstruction of selected entity interpretations followed by
/// independent feature, predicate, and aggregate replay.
#[derive(Clone, Copy, Debug, Default)]
pub struct EntityObservationVerifier;

impl EntityObservationVerifier {
    /// Reconstruct every component binding with a disjoint-set reference path
    /// and independently replay the complete learned mechanism.
    ///
    /// # Errors
    ///
    /// Rejects missing/extra bindings, malformed evidence, incompatible entity
    /// interpretations, resource caps, or downstream replay failure.
    pub fn verify(
        &self,
        learned: &LearnedEntityObservationGrouping,
        traces: &[RawEntityObservationTrace],
        entity_limits: EntityObservationLimits,
        observation_limits: ObservationPredicateLimits,
        predicate_limits: PredicateGroupingLimits,
        execution_limits: ExecLimits,
    ) -> Result<VerifiedEntityObservationGrouping, EntityObservationVerifyError> {
        if traces.len() < 2 {
            return Err(EntityObservationVerifyError::InsufficientEvidence);
        }
        let experiences = traces
            .iter()
            .map(|trace| trace.episode)
            .collect::<BTreeSet<_>>();
        if experiences.len() != traces.len() {
            return Err(EntityObservationVerifyError::DuplicateEpisode);
        }
        if learned
            .selected_relations
            .keys()
            .copied()
            .collect::<BTreeSet<_>>()
            != experiences
        {
            return Err(EntityObservationVerifyError::BindingMismatch);
        }
        let lowered = traces
            .iter()
            .map(|trace| {
                let relation = learned
                    .selected_relations
                    .get(&trace.episode)
                    .ok_or(EntityObservationVerifyError::BindingMismatch)?;
                verifier_lower_episode(trace, relation, entity_limits, observation_limits)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let replay = ObservationPredicateVerifier
            .verify(
                &learned.mechanism,
                &lowered,
                observation_limits,
                predicate_limits,
                execution_limits,
            )
            .map_err(EntityObservationVerifyError::Observation)?;
        let mechanism = learned
            .mechanism_hash()
            .map_err(|_| EntityObservationVerifyError::Canonicalization)?;
        if replay.mechanism() != mechanism || replay.experiences() != &experiences {
            return Err(EntityObservationVerifyError::BindingMismatch);
        }
        Ok(VerifiedEntityObservationGrouping {
            mechanism,
            experiences,
            replayed_groups: replay.replayed_groups(),
        })
    }
}

fn verifier_lower_episode(
    trace: &RawEntityObservationTrace,
    relation: &RelationRole,
    entity_limits: EntityObservationLimits,
    observation_limits: ObservationPredicateLimits,
) -> Result<PrimitiveObservationTrace, EntityObservationVerifyError> {
    verifier_validate_evidence(&trace.evidence, entity_limits)?;
    let components = verifier_components(&trace.evidence, relation)?;
    if components.len() < 2 || components.len() > observation_limits.maximum_entities {
        return Err(EntityObservationVerifyError::InvalidInterpretation);
    }
    let mut entities = Vec::with_capacity(components.len());
    for (index, component) in components.into_iter().enumerate() {
        if component.len() < 2 || component.len() > observation_limits.maximum_observations {
            return Err(EntityObservationVerifyError::InvalidInterpretation);
        }
        let mut identity = b"cairn.entity-observation.v1".to_vec();
        let observations = component
            .iter()
            .map(|member| {
                identity.extend_from_slice(member.0.as_bytes());
                trace
                    .evidence
                    .primitives
                    .get(member)
                    .map(|primitive| primitive.observation)
                    .ok_or(EntityObservationVerifyError::InvalidEvidence)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let surrogate = 100_000_i64
            .checked_add(i64::try_from(index).map_err(|_| EntityObservationVerifyError::Capacity)?)
            .ok_or(EntityObservationVerifyError::Capacity)?;
        entities.push(PrimitiveObservationEntity {
            id: EntityId(Hash32::digest(&identity)),
            value: surrogate,
            observations,
        });
    }
    Ok(PrimitiveObservationTrace {
        episode: trace.episode,
        entities,
        outputs: trace.outputs.clone(),
    })
}

fn verifier_validate_evidence(
    evidence: &RawEntityObservationEvidence,
    limits: EntityObservationLimits,
) -> Result<(), EntityObservationVerifyError> {
    if evidence.primitives.is_empty() || evidence.relations.is_empty() {
        return Err(EntityObservationVerifyError::InvalidEvidence);
    }
    if evidence.primitives.len() > limits.maximum_primitives
        || evidence.relations.len() > limits.maximum_relations
    {
        return Err(EntityObservationVerifyError::Capacity);
    }
    let mut roles = BTreeSet::new();
    let mut relations = BTreeSet::new();
    for relation in &evidence.relations {
        if !evidence.primitives.contains_key(&relation.subject)
            || !evidence.primitives.contains_key(&relation.object)
        {
            return Err(EntityObservationVerifyError::InvalidEvidence);
        }
        roles.insert(relation.role.clone());
        if !relations.insert((
            relation.role.clone(),
            relation.subject.clone(),
            relation.object.clone(),
        )) {
            return Err(EntityObservationVerifyError::InvalidEvidence);
        }
    }
    if roles.len() > limits.maximum_roles_per_episode {
        return Err(EntityObservationVerifyError::Capacity);
    }
    Ok(())
}

fn verifier_components(
    evidence: &RawEntityObservationEvidence,
    relation: &RelationRole,
) -> Result<Vec<BTreeSet<EntityId>>, EntityObservationVerifyError> {
    let mut parents = evidence
        .primitives
        .keys()
        .cloned()
        .map(|primitive| (primitive.clone(), primitive))
        .collect::<BTreeMap<_, _>>();
    for edge in evidence
        .relations
        .iter()
        .filter(|edge| &edge.role == relation)
    {
        let left = verifier_root(&parents, &edge.subject)?;
        let right = verifier_root(&parents, &edge.object)?;
        if left != right {
            let (minimum, maximum) = if left < right {
                (left, right)
            } else {
                (right, left)
            };
            parents.insert(maximum, minimum);
        }
    }
    let mut groups = BTreeMap::<EntityId, BTreeSet<EntityId>>::new();
    for primitive in evidence.primitives.keys() {
        let root = verifier_root(&parents, primitive)?;
        groups.entry(root).or_default().insert(primitive.clone());
    }
    Ok(groups.into_values().collect())
}

fn verifier_root(
    parents: &BTreeMap<EntityId, EntityId>,
    node: &EntityId,
) -> Result<EntityId, EntityObservationVerifyError> {
    let mut current = node.clone();
    for _ in 0..=parents.len() {
        let parent = parents
            .get(&current)
            .ok_or(EntityObservationVerifyError::InvalidEvidence)?;
        if parent == &current {
            return Ok(current);
        }
        current = parent.clone();
    }
    Err(EntityObservationVerifyError::InvalidEvidence)
}
