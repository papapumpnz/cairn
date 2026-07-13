use crate::{
    LearnedObservationPredicateGrouping, ObservationPredicateError, ObservationPredicateInventor,
    ObservationPredicateLimits, PredicateGroupingLimits, PrimitiveObservationEntity,
    PrimitiveObservationTrace,
};
use cairn_core::{EntityId, Hash32, RelationRecord, RelationRole};
use cairn_invent::InventionLimits;
use std::collections::{BTreeMap, BTreeSet};

/// One raw primitive before an entity interpretation has been selected.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RawObservationPrimitive {
    pub observation: i64,
}

/// Unselected primitive evidence and competing anonymous binary relations.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RawEntityObservationEvidence {
    pub primitives: BTreeMap<EntityId, RawObservationPrimitive>,
    pub relations: Vec<RelationRecord>,
}

/// One episode whose entity interpretation must be chosen through downstream
/// explanatory adequacy.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RawEntityObservationTrace {
    pub episode: Hash32,
    pub evidence: RawEntityObservationEvidence,
    pub outputs: BTreeSet<i64>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EntityObservationLimits {
    pub maximum_primitives: usize,
    pub maximum_relations: usize,
    pub maximum_roles_per_episode: usize,
    pub maximum_binding_sets: usize,
}

impl Default for EntityObservationLimits {
    fn default() -> Self {
        Self {
            maximum_primitives: 16_384,
            maximum_relations: 65_536,
            maximum_roles_per_episode: 64,
            maximum_binding_sets: 4_096,
        }
    }
}

/// A task-local entity binding plus a source-independent learned mechanism.
/// Bindings are evidence lineage and do not enter reusable mechanism identity.
#[derive(Clone, Debug, PartialEq)]
pub struct LearnedEntityObservationGrouping {
    pub selected_relations: BTreeMap<Hash32, RelationRole>,
    pub mechanism: LearnedObservationPredicateGrouping,
    pub binding_sets_evaluated: usize,
}

impl LearnedEntityObservationGrouping {
    /// Identity of the reusable mechanism, excluding task-local relation names.
    ///
    /// # Errors
    ///
    /// Returns canonical feature-mechanism encoding failure.
    pub fn mechanism_hash(&self) -> Result<Hash32, cairn_core::CanonicalError> {
        self.mechanism.semantic_hash()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EntityObservationError {
    InsufficientEpisodes,
    DuplicateEpisode,
    EmptyEvidence,
    NoRelations,
    UnknownPrimitive(EntityId),
    DuplicateRelation,
    InvalidInterpretation,
    Capacity,
    NoExactTheory,
    Ambiguous,
    Observation(ObservationPredicateError),
}

/// Joint reference learner for entity interpretation and executable feature,
/// predicate, and aggregate invention.
#[derive(Clone, Copy, Debug, Default)]
pub struct EntityObservationInventor;

impl EntityObservationInventor {
    /// Materialize one task-local entity interpretation as generic primitive
    /// observations without assigning it semantic correctness.
    ///
    /// # Errors
    ///
    /// Rejects malformed evidence, incompatible component arity, or capacity.
    pub fn lower(
        &self,
        trace: &RawEntityObservationTrace,
        relation: &RelationRole,
        entity_limits: EntityObservationLimits,
        observation_limits: ObservationPredicateLimits,
    ) -> Result<PrimitiveObservationTrace, EntityObservationError> {
        lower_episode(trace, relation, entity_limits, observation_limits)
    }

    /// Enumerate one connected-component interpretation per anonymous relation
    /// role and select a complete cross-episode binding only through exact
    /// downstream theory induction.
    ///
    /// # Errors
    ///
    /// Rejects malformed/capped evidence, absence of an exact interpretation,
    /// or multiple exact interpretations.
    pub fn invent(
        &self,
        traces: &[RawEntityObservationTrace],
        entity_limits: EntityObservationLimits,
        observation_limits: ObservationPredicateLimits,
        predicate_limits: PredicateGroupingLimits,
        invention_limits: InventionLimits,
    ) -> Result<LearnedEntityObservationGrouping, EntityObservationError> {
        validate_traces(traces, entity_limits)?;
        let roles = traces
            .iter()
            .map(|trace| relation_roles(&trace.evidence, entity_limits))
            .collect::<Result<Vec<_>, _>>()?;
        let binding_sets =
            enumerate_binding_sets(traces, &roles, entity_limits.maximum_binding_sets)?;
        let mut exact = Vec::new();
        let mut binding_sets_evaluated = 0_usize;
        for bindings in binding_sets {
            binding_sets_evaluated = binding_sets_evaluated.saturating_add(1);
            let lowered = traces
                .iter()
                .map(|trace| {
                    let relation = bindings
                        .get(&trace.episode)
                        .ok_or(EntityObservationError::InvalidInterpretation)?;
                    lower_episode(trace, relation, entity_limits, observation_limits)
                })
                .collect::<Result<Vec<_>, _>>();
            let Ok(lowered) = lowered else {
                continue;
            };
            if let Ok(mechanism) = ObservationPredicateInventor.invent(
                &lowered,
                observation_limits,
                predicate_limits,
                invention_limits,
            ) {
                exact.push(LearnedEntityObservationGrouping {
                    selected_relations: bindings,
                    mechanism,
                    binding_sets_evaluated,
                });
            }
        }
        match exact.len() {
            0 => Err(EntityObservationError::NoExactTheory),
            1 => {
                let mut learned = exact.pop().ok_or(EntityObservationError::NoExactTheory)?;
                learned.binding_sets_evaluated = binding_sets_evaluated;
                Ok(learned)
            }
            _ => Err(EntityObservationError::Ambiguous),
        }
    }

    /// Lower one current episode under an explicit task-local binding and
    /// execute the source-independent mechanism as an untrusted proposal.
    ///
    /// # Errors
    ///
    /// Rejects malformed interpretations or exact execution failure.
    pub fn predict(
        &self,
        mechanism: &LearnedObservationPredicateGrouping,
        trace: &RawEntityObservationTrace,
        relation: &RelationRole,
        entity_limits: EntityObservationLimits,
        observation_limits: ObservationPredicateLimits,
        predicate_limits: PredicateGroupingLimits,
    ) -> Result<BTreeSet<i64>, EntityObservationError> {
        let lowered = lower_episode(trace, relation, entity_limits, observation_limits)?;
        ObservationPredicateInventor
            .predict(
                mechanism,
                &lowered.entities,
                observation_limits,
                predicate_limits,
            )
            .map_err(EntityObservationError::Observation)
    }
}

fn validate_traces(
    traces: &[RawEntityObservationTrace],
    limits: EntityObservationLimits,
) -> Result<(), EntityObservationError> {
    if traces.len() < 2 {
        return Err(EntityObservationError::InsufficientEpisodes);
    }
    let episodes = traces
        .iter()
        .map(|trace| trace.episode)
        .collect::<BTreeSet<_>>();
    if episodes.len() != traces.len() {
        return Err(EntityObservationError::DuplicateEpisode);
    }
    for trace in traces {
        validate_evidence(&trace.evidence, limits)?;
    }
    Ok(())
}

fn validate_evidence(
    evidence: &RawEntityObservationEvidence,
    limits: EntityObservationLimits,
) -> Result<(), EntityObservationError> {
    if evidence.primitives.is_empty() {
        return Err(EntityObservationError::EmptyEvidence);
    }
    if evidence.relations.is_empty() {
        return Err(EntityObservationError::NoRelations);
    }
    if evidence.primitives.len() > limits.maximum_primitives
        || evidence.relations.len() > limits.maximum_relations
        || limits.maximum_binding_sets == 0
        || limits.maximum_roles_per_episode == 0
    {
        return Err(EntityObservationError::Capacity);
    }
    let mut relations = BTreeSet::new();
    for relation in &evidence.relations {
        if !evidence.primitives.contains_key(&relation.subject) {
            return Err(EntityObservationError::UnknownPrimitive(
                relation.subject.clone(),
            ));
        }
        if !evidence.primitives.contains_key(&relation.object) {
            return Err(EntityObservationError::UnknownPrimitive(
                relation.object.clone(),
            ));
        }
        if !relations.insert((
            relation.role.clone(),
            relation.subject.clone(),
            relation.object.clone(),
        )) {
            return Err(EntityObservationError::DuplicateRelation);
        }
    }
    Ok(())
}

fn relation_roles(
    evidence: &RawEntityObservationEvidence,
    limits: EntityObservationLimits,
) -> Result<Vec<RelationRole>, EntityObservationError> {
    let roles = evidence
        .relations
        .iter()
        .map(|relation| relation.role.clone())
        .collect::<BTreeSet<_>>();
    if roles.len() > limits.maximum_roles_per_episode {
        return Err(EntityObservationError::Capacity);
    }
    Ok(roles.into_iter().collect())
}

fn enumerate_binding_sets(
    traces: &[RawEntityObservationTrace],
    roles: &[Vec<RelationRole>],
    maximum: usize,
) -> Result<Vec<BTreeMap<Hash32, RelationRole>>, EntityObservationError> {
    fn extend(
        traces: &[RawEntityObservationTrace],
        roles: &[Vec<RelationRole>],
        index: usize,
        maximum: usize,
        current: &mut BTreeMap<Hash32, RelationRole>,
        output: &mut Vec<BTreeMap<Hash32, RelationRole>>,
    ) -> Result<(), EntityObservationError> {
        if index == traces.len() {
            if output.len() >= maximum {
                return Err(EntityObservationError::Capacity);
            }
            output.push(current.clone());
            return Ok(());
        }
        for role in &roles[index] {
            current.insert(traces[index].episode, role.clone());
            extend(traces, roles, index + 1, maximum, current, output)?;
            current.remove(&traces[index].episode);
        }
        Ok(())
    }
    let mut output = Vec::new();
    extend(traces, roles, 0, maximum, &mut BTreeMap::new(), &mut output)?;
    Ok(output)
}

fn lower_episode(
    trace: &RawEntityObservationTrace,
    relation: &RelationRole,
    entity_limits: EntityObservationLimits,
    observation_limits: ObservationPredicateLimits,
) -> Result<PrimitiveObservationTrace, EntityObservationError> {
    validate_evidence(&trace.evidence, entity_limits)?;
    let components = connected_components(&trace.evidence, relation)?;
    if components.len() < 2 || components.len() > observation_limits.maximum_entities {
        return Err(EntityObservationError::InvalidInterpretation);
    }
    let mut entities = Vec::with_capacity(components.len());
    for (index, component) in components.into_iter().enumerate() {
        if component.len() < 2 || component.len() > observation_limits.maximum_observations {
            return Err(EntityObservationError::InvalidInterpretation);
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
                    .ok_or_else(|| EntityObservationError::UnknownPrimitive(member.clone()))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let surrogate = 100_000_i64
            .checked_add(i64::try_from(index).map_err(|_| EntityObservationError::Capacity)?)
            .ok_or(EntityObservationError::Capacity)?;
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

fn connected_components(
    evidence: &RawEntityObservationEvidence,
    role: &RelationRole,
) -> Result<Vec<BTreeSet<EntityId>>, EntityObservationError> {
    let mut adjacency = evidence
        .primitives
        .keys()
        .cloned()
        .map(|primitive| (primitive, BTreeSet::new()))
        .collect::<BTreeMap<_, _>>();
    for relation in evidence
        .relations
        .iter()
        .filter(|relation| &relation.role == role)
    {
        adjacency
            .get_mut(&relation.subject)
            .ok_or_else(|| EntityObservationError::UnknownPrimitive(relation.subject.clone()))?
            .insert(relation.object.clone());
        adjacency
            .get_mut(&relation.object)
            .ok_or_else(|| EntityObservationError::UnknownPrimitive(relation.object.clone()))?
            .insert(relation.subject.clone());
    }
    let mut unseen = evidence.primitives.keys().cloned().collect::<BTreeSet<_>>();
    let mut output = Vec::new();
    while let Some(seed) = unseen.pop_first() {
        let mut component = BTreeSet::from([seed]);
        let mut frontier = component.clone();
        while let Some(node) = frontier.pop_first() {
            for neighbor in adjacency.get(&node).into_iter().flatten() {
                if unseen.remove(neighbor) {
                    component.insert(neighbor.clone());
                    frontier.insert(neighbor.clone());
                }
            }
        }
        output.push(component);
    }
    Ok(output)
}
