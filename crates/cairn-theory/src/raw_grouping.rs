use crate::{
    GroupingHypothesisTrace, GroupingRelationInventor, GroupingSelectionError, LearnedGroupingMerge,
};
use cairn_core::{EntityId, Hash32, RelationRecord, RelationRole};
use cairn_invent::InventionLimits;
use std::collections::{BTreeMap, BTreeSet};

/// Opaque scalar evidence attached to entities plus unselected relation edges.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RawGroupingEvidence {
    pub entities: BTreeMap<EntityId, i64>,
    pub relations: Vec<RelationRecord>,
}

/// One learning episode before relation edges have been grouped into entities.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RawGroupingTrace {
    pub episode: Hash32,
    pub evidence: RawGroupingEvidence,
    pub outputs: BTreeSet<i64>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RawGroupingLimits {
    pub maximum_entities: usize,
    pub maximum_relations: usize,
    pub maximum_roles: usize,
}

impl Default for RawGroupingLimits {
    fn default() -> Self {
        Self {
            maximum_entities: 4_096,
            maximum_relations: 16_384,
            maximum_roles: 256,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RawGroupingError {
    EmptyEntities,
    NoRelations,
    Capacity,
    UnknownEntity(EntityId),
    DuplicateRelation,
    Selection(GroupingSelectionError),
}

/// Reference proposer for relation-defined connected entity collections.
#[derive(Clone, Copy, Debug, Default)]
pub struct RawRelationGroupingInventor;

impl RawRelationGroupingInventor {
    /// Construct one exact partition hypothesis per observed relation role.
    ///
    /// Relation direction is ignored for collection membership. Every entity
    /// appears exactly once in every candidate partition, including isolated
    /// singleton entities.
    ///
    /// # Errors
    ///
    /// Rejects empty/capped evidence, unknown endpoints, duplicate edges, or
    /// absence of any relation role.
    pub fn lower(
        &self,
        evidence: &RawGroupingEvidence,
        limits: RawGroupingLimits,
    ) -> Result<BTreeMap<RelationRole, Vec<BTreeSet<i64>>>, RawGroupingError> {
        validate_raw_evidence(evidence, limits)?;
        let roles = evidence
            .relations
            .iter()
            .map(|relation| relation.role.clone())
            .collect::<BTreeSet<_>>();
        roles
            .into_iter()
            .map(|role| {
                let groups = connected_value_groups(evidence, &role)?;
                Ok((role, groups))
            })
            .collect()
    }

    /// Invent a grouping relation and merge directly from raw relation edges.
    ///
    /// # Errors
    ///
    /// Returns raw-evidence or joint-theory selection failure.
    pub fn invent(
        &self,
        traces: &[RawGroupingTrace],
        raw_limits: RawGroupingLimits,
        invention_limits: InventionLimits,
    ) -> Result<LearnedGroupingMerge, RawGroupingError> {
        let lowered = traces
            .iter()
            .map(|trace| {
                Ok(GroupingHypothesisTrace {
                    episode: trace.episode,
                    candidates: self.lower(&trace.evidence, raw_limits)?,
                    outputs: trace.outputs.clone(),
                })
            })
            .collect::<Result<Vec<_>, RawGroupingError>>()?;
        GroupingRelationInventor
            .invent(&lowered, invention_limits)
            .map_err(RawGroupingError::Selection)
    }

    /// Lower current raw evidence and execute a learned relation-bound merge.
    ///
    /// # Errors
    ///
    /// Returns raw-evidence, missing-role, or exact execution failure.
    pub fn predict(
        &self,
        learned: &LearnedGroupingMerge,
        evidence: &RawGroupingEvidence,
        limits: RawGroupingLimits,
    ) -> Result<BTreeSet<i64>, RawGroupingError> {
        let candidates = self.lower(evidence, limits)?;
        GroupingRelationInventor
            .predict(learned, &candidates)
            .map_err(RawGroupingError::Selection)
    }
}

fn validate_raw_evidence(
    evidence: &RawGroupingEvidence,
    limits: RawGroupingLimits,
) -> Result<(), RawGroupingError> {
    if evidence.entities.is_empty() {
        return Err(RawGroupingError::EmptyEntities);
    }
    if evidence.relations.is_empty() {
        return Err(RawGroupingError::NoRelations);
    }
    if evidence.entities.len() > limits.maximum_entities
        || evidence.relations.len() > limits.maximum_relations
    {
        return Err(RawGroupingError::Capacity);
    }
    let mut roles = BTreeSet::new();
    let mut relations = BTreeSet::new();
    for relation in &evidence.relations {
        if !evidence.entities.contains_key(&relation.subject) {
            return Err(RawGroupingError::UnknownEntity(relation.subject.clone()));
        }
        if !evidence.entities.contains_key(&relation.object) {
            return Err(RawGroupingError::UnknownEntity(relation.object.clone()));
        }
        roles.insert(relation.role.clone());
        if !relations.insert((
            relation.role.clone(),
            relation.subject.clone(),
            relation.object.clone(),
        )) {
            return Err(RawGroupingError::DuplicateRelation);
        }
    }
    if roles.len() > limits.maximum_roles {
        return Err(RawGroupingError::Capacity);
    }
    Ok(())
}

fn connected_value_groups(
    evidence: &RawGroupingEvidence,
    role: &RelationRole,
) -> Result<Vec<BTreeSet<i64>>, RawGroupingError> {
    let mut adjacency = evidence
        .entities
        .keys()
        .cloned()
        .map(|entity| (entity, BTreeSet::new()))
        .collect::<BTreeMap<_, _>>();
    for relation in evidence
        .relations
        .iter()
        .filter(|relation| &relation.role == role)
    {
        adjacency
            .get_mut(&relation.subject)
            .ok_or_else(|| RawGroupingError::UnknownEntity(relation.subject.clone()))?
            .insert(relation.object.clone());
        adjacency
            .get_mut(&relation.object)
            .ok_or_else(|| RawGroupingError::UnknownEntity(relation.object.clone()))?
            .insert(relation.subject.clone());
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
