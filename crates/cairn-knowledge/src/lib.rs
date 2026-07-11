//! Source-bound relational knowledge with explicit contradiction retention.

use cairn_core::{Hash32, Observation, ObservationId};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct EntityId(pub Hash32);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ClaimId(pub Hash32);

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum RelationKind {
    Attribute,
    Causal,
    Taxonomic,
    Temporal,
    Procedure,
    Custom,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ClaimPredicate {
    pub kind: RelationKind,
    pub name: String,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ClaimTerm {
    Entity(EntityId),
    Integer(i64),
    Text(String),
    Mechanism(Hash32),
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ValidityInterval {
    pub start: Option<u64>,
    pub end: Option<u64>,
}

impl ValidityInterval {
    #[must_use]
    pub fn contains(self, instant: u64) -> bool {
        self.start.is_none_or(|start| start <= instant) && self.end.is_none_or(|end| instant <= end)
    }

    #[must_use]
    pub fn overlaps(self, other: Self) -> bool {
        let self_start = self.start.unwrap_or(0);
        let other_start = other.start.unwrap_or(0);
        let self_end = self.end.unwrap_or(u64::MAX);
        let other_end = other.end.unwrap_or(u64::MAX);
        self_start <= other_end && other_start <= self_end
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClaimDraft {
    pub subject: EntityId,
    pub predicate: ClaimPredicate,
    pub object: ClaimTerm,
    pub validity: ValidityInterval,
    pub confidence_ppm: u32,
    pub sources: BTreeSet<ObservationId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KnowledgeClaim {
    pub id: ClaimId,
    pub subject: EntityId,
    pub predicate: ClaimPredicate,
    pub object: ClaimTerm,
    pub validity: ValidityInterval,
    pub confidence_ppm: u32,
    pub sources: BTreeSet<ObservationId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClaimInsertion {
    pub claim: ClaimId,
    pub contradictions: Vec<ClaimId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EntityMergeProposal {
    pub left: EntityId,
    pub right: EntityId,
    pub confidence_ppm: u32,
    pub evidence: BTreeSet<ObservationId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum KnowledgeError {
    InvalidObservation,
    EmptyPredicate,
    InvalidConfidence,
    InvalidValidity,
    EmptySources,
    UnknownSource(ObservationId),
    DuplicateClaim,
    InvalidMerge,
}

#[derive(Clone, Debug, Default)]
pub struct KnowledgeBase {
    observations: BTreeSet<ObservationId>,
    claims: BTreeMap<ClaimId, KnowledgeClaim>,
    by_relation: BTreeMap<(EntityId, ClaimPredicate), Vec<ClaimId>>,
    contradictions: BTreeSet<(ClaimId, ClaimId)>,
}

impl KnowledgeBase {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            observations: BTreeSet::new(),
            claims: BTreeMap::new(),
            by_relation: BTreeMap::new(),
            contradictions: BTreeSet::new(),
        }
    }

    /// Admit a source observation whose identity will bind later claims.
    ///
    /// # Errors
    ///
    /// Rejects an invalid or identity-mismatched observation.
    pub fn observe(&mut self, observation: &Observation) -> Result<bool, KnowledgeError> {
        observation
            .verify()
            .map_err(|_| KnowledgeError::InvalidObservation)?;
        Ok(self.observations.insert(observation.id))
    }

    /// Insert a typed claim while retaining every contradictory claim.
    ///
    /// # Errors
    ///
    /// Rejects malformed predicates, confidence/validity, missing provenance,
    /// unknown sources, and exact duplicates.
    pub fn insert(&mut self, draft: ClaimDraft) -> Result<ClaimInsertion, KnowledgeError> {
        validate_draft(&draft, &self.observations)?;
        let id = ClaimId(Hash32::digest(&claim_bytes(&draft)));
        if self.claims.contains_key(&id) {
            return Err(KnowledgeError::DuplicateClaim);
        }
        let key = (draft.subject, draft.predicate.clone());
        let contradictions = self
            .by_relation
            .get(&key)
            .into_iter()
            .flatten()
            .filter_map(|existing_id| {
                let existing = &self.claims[existing_id];
                (existing.object != draft.object && existing.validity.overlaps(draft.validity))
                    .then_some(*existing_id)
            })
            .collect::<Vec<_>>();
        for existing in &contradictions {
            self.contradictions.insert(ordered_pair(id, *existing));
        }
        self.by_relation.entry(key).or_default().push(id);
        self.claims.insert(
            id,
            KnowledgeClaim {
                id,
                subject: draft.subject,
                predicate: draft.predicate,
                object: draft.object,
                validity: draft.validity,
                confidence_ppm: draft.confidence_ppm,
                sources: draft.sources,
            },
        );
        Ok(ClaimInsertion {
            claim: id,
            contradictions,
        })
    }

    #[must_use]
    pub fn claims_for(
        &self,
        subject: EntityId,
        predicate: &ClaimPredicate,
        instant: Option<u64>,
    ) -> Vec<&KnowledgeClaim> {
        self.by_relation
            .get(&(subject, predicate.clone()))
            .into_iter()
            .flatten()
            .filter_map(|id| self.claims.get(id))
            .filter(|claim| instant.is_none_or(|instant| claim.validity.contains(instant)))
            .collect()
    }

    pub fn contradictions(&self) -> impl Iterator<Item = (ClaimId, ClaimId)> + '_ {
        self.contradictions.iter().copied()
    }

    /// Propose, but do not apply, a reversible entity merge.
    ///
    /// # Errors
    ///
    /// Rejects self-merges, invalid confidence, or absent provenance.
    pub fn propose_merge(
        &self,
        proposal: EntityMergeProposal,
    ) -> Result<EntityMergeProposal, KnowledgeError> {
        if proposal.left == proposal.right
            || proposal.confidence_ppm > 1_000_000
            || proposal.evidence.is_empty()
            || proposal
                .evidence
                .iter()
                .any(|source| !self.observations.contains(source))
        {
            return Err(KnowledgeError::InvalidMerge);
        }
        Ok(proposal)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.claims.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.claims.is_empty()
    }
}

fn validate_draft(
    draft: &ClaimDraft,
    observations: &BTreeSet<ObservationId>,
) -> Result<(), KnowledgeError> {
    if draft.predicate.name.is_empty() {
        return Err(KnowledgeError::EmptyPredicate);
    }
    if draft.confidence_ppm > 1_000_000 {
        return Err(KnowledgeError::InvalidConfidence);
    }
    if draft
        .validity
        .start
        .zip(draft.validity.end)
        .is_some_and(|(start, end)| start > end)
    {
        return Err(KnowledgeError::InvalidValidity);
    }
    if draft.sources.is_empty() {
        return Err(KnowledgeError::EmptySources);
    }
    if let Some(source) = draft
        .sources
        .iter()
        .find(|source| !observations.contains(source))
    {
        return Err(KnowledgeError::UnknownSource(*source));
    }
    Ok(())
}

fn ordered_pair(left: ClaimId, right: ClaimId) -> (ClaimId, ClaimId) {
    if left < right {
        (left, right)
    } else {
        (right, left)
    }
}

fn claim_bytes(draft: &ClaimDraft) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(draft.subject.0.as_bytes());
    bytes.push(relation_kind_code(draft.predicate.kind));
    push_bytes(&mut bytes, draft.predicate.name.as_bytes());
    match &draft.object {
        ClaimTerm::Entity(entity) => {
            bytes.push(0);
            bytes.extend_from_slice(entity.0.as_bytes());
        }
        ClaimTerm::Integer(value) => {
            bytes.push(1);
            bytes.extend_from_slice(&value.to_be_bytes());
        }
        ClaimTerm::Text(value) => {
            bytes.push(2);
            push_bytes(&mut bytes, value.as_bytes());
        }
        ClaimTerm::Mechanism(mechanism) => {
            bytes.push(3);
            bytes.extend_from_slice(mechanism.as_bytes());
        }
    }
    for bound in [draft.validity.start, draft.validity.end] {
        match bound {
            Some(value) => {
                bytes.push(1);
                bytes.extend_from_slice(&value.to_be_bytes());
            }
            None => bytes.push(0),
        }
    }
    bytes.extend_from_slice(&draft.confidence_ppm.to_be_bytes());
    for source in &draft.sources {
        bytes.extend_from_slice(source.0.as_bytes());
    }
    bytes
}

fn push_bytes(target: &mut Vec<u8>, value: &[u8]) {
    target.extend_from_slice(&u64::try_from(value.len()).unwrap_or(u64::MAX).to_be_bytes());
    target.extend_from_slice(value);
}

const fn relation_kind_code(kind: RelationKind) -> u8 {
    match kind {
        RelationKind::Attribute => 0,
        RelationKind::Causal => 1,
        RelationKind::Taxonomic => 2,
        RelationKind::Temporal => 3,
        RelationKind::Procedure => 4,
        RelationKind::Custom => 5,
    }
}
