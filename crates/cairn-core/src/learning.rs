use crate::{CanonicalEncode, CanonicalError, CanonicalValue, Hash32};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ObservationId(pub Hash32);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct TaskId(pub Hash32);

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ModalityId(pub String);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceReference {
    pub locator: String,
    pub content_hash: Hash32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ObservationCertainty {
    Exact,
    PartsPerMillion(u32),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Observation {
    pub id: ObservationId,
    pub modality: ModalityId,
    pub logical_time: Option<u64>,
    pub source: SourceReference,
    pub payload: CanonicalValue,
    pub certainty: ObservationCertainty,
}

/// Environment-neutral exact type for one opaque structured-observation value.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum EvidenceValueType {
    Boolean,
    Integer,
    Atom,
    Entity,
    EntitySet,
    Relation,
    Action,
    Effect,
    Mechanism,
}

/// Observation-local identity of one primitive or hypothesized evidence item.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct EvidenceElementId(pub Hash32);

/// One typed opaque evidence item. `value` is content, while `id` is the node
/// referenced by relations; neither carries a frontend-specific meaning.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct EvidenceElement {
    pub id: EvidenceElementId,
    pub value_type: EvidenceValueType,
    pub value: Hash32,
}

/// One finite ordered-arity relation over evidence items.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct EvidenceRelation {
    pub predicate: Hash32,
    pub participants: Vec<EvidenceElementId>,
}

/// Resource bounds for an exact relational observation envelope.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RelationalObservationLimits {
    pub maximum_elements: u32,
    pub maximum_relations: u32,
    pub maximum_relation_arity: u16,
}

impl Default for RelationalObservationLimits {
    fn default() -> Self {
        Self {
            maximum_elements: 1_000_000,
            maximum_relations: 4_000_000,
            maximum_relation_arity: 32,
        }
    }
}

/// An authenticated observation whose payload has exact, modality-neutral
/// element and relation semantics.
#[derive(Clone, Debug, PartialEq)]
pub struct RelationalObservation {
    pub observation: Observation,
    pub elements: Vec<EvidenceElement>,
    pub relations: Vec<EvidenceRelation>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RelationalObservationError {
    Observation(ObservationError),
    Capacity {
        resource: &'static str,
        requested: u64,
        maximum: u64,
    },
    DuplicateElement(EvidenceElementId),
    EmptyRelation,
    DuplicateRelation,
    UnknownParticipant(EvidenceElementId),
    PayloadMismatch,
}

impl From<ObservationError> for RelationalObservationError {
    fn from(value: ObservationError) -> Self {
        Self::Observation(value)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Intervention {
    pub action: String,
    pub arguments: Vec<ObservationId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Demonstration {
    pub context: Vec<ObservationId>,
    pub intervention: Option<Intervention>,
    pub outcome: Vec<ObservationId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PredictionRequest {
    pub context: Vec<ObservationId>,
    pub intervention: Option<Intervention>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LearningProblem {
    pub id: TaskId,
    pub observations: Vec<Observation>,
    pub demonstrations: Vec<Demonstration>,
    pub requested_predictions: Vec<PredictionRequest>,
    pub attempt_limit: u8,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CollectionDemonstration {
    pub input: Vec<i64>,
    pub output: BTreeSet<i64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CollectionProcedureProblem {
    pub maximum_cardinality: u32,
    pub demonstrations: Vec<CollectionDemonstration>,
    pub prediction_input: Vec<i64>,
    pub maximum_predictions: usize,
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct EntityId(pub Hash32);

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct FeatureId(pub String);

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RelationRole(pub String);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EntityRecord {
    pub id: EntityId,
    pub attributes: BTreeMap<FeatureId, i64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RelationRecord {
    pub role: RelationRole,
    pub subject: EntityId,
    pub object: EntityId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RelationalEntityCollection {
    pub entities: Vec<EntityRecord>,
    pub relations: Vec<RelationRecord>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EntitySelectionDemonstration {
    pub collection: RelationalEntityCollection,
    pub selected: BTreeSet<EntityId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EntitySelectionProblem {
    pub demonstrations: Vec<EntitySelectionDemonstration>,
    pub prediction: RelationalEntityCollection,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ObservationError {
    InvalidConfidence(u32),
    EmptyModality,
    EmptySource,
    Canonical(CanonicalError),
    IdentityMismatch,
}

impl From<CanonicalError> for ObservationError {
    fn from(value: CanonicalError) -> Self {
        Self::Canonical(value)
    }
}

impl Observation {
    /// Construct an observation whose identity covers modality, time, source,
    /// certainty, and canonical payload without any adapter-specific meaning.
    ///
    /// # Errors
    ///
    /// Rejects empty schema/source identifiers, invalid confidence, or an
    /// unencodable payload.
    pub fn new(
        modality: ModalityId,
        logical_time: Option<u64>,
        source: SourceReference,
        payload: CanonicalValue,
        certainty: ObservationCertainty,
    ) -> Result<Self, ObservationError> {
        validate_fields(&modality, &source, certainty)?;
        let id = observation_id(&modality, logical_time, &source, &payload, certainty)?;
        Ok(Self {
            id,
            modality,
            logical_time,
            source,
            payload,
            certainty,
        })
    }

    /// Recompute and verify the observation identity at an adapter boundary.
    ///
    /// # Errors
    ///
    /// Returns the exact validation or identity mismatch.
    pub fn verify(&self) -> Result<(), ObservationError> {
        validate_fields(&self.modality, &self.source, self.certainty)?;
        let actual = observation_id(
            &self.modality,
            self.logical_time,
            &self.source,
            &self.payload,
            self.certainty,
        )?;
        if actual != self.id {
            return Err(ObservationError::IdentityMismatch);
        }
        Ok(())
    }
}

impl RelationalObservation {
    /// Construct a canonical structured observation. Element and relation input
    /// order is non-semantic and normalized before observation identity is
    /// calculated.
    ///
    /// # Errors
    ///
    /// Rejects capacity excess, duplicate elements/relations, empty relations,
    /// unknown participants, or ordinary observation identity failures.
    pub fn new(
        modality: ModalityId,
        logical_time: Option<u64>,
        source: SourceReference,
        certainty: ObservationCertainty,
        mut elements: Vec<EvidenceElement>,
        mut relations: Vec<EvidenceRelation>,
        limits: RelationalObservationLimits,
    ) -> Result<Self, RelationalObservationError> {
        normalize_relational_evidence(&mut elements, &mut relations, limits)?;
        let payload = relational_payload(&elements, &relations);
        let observation = Observation::new(modality, logical_time, source, payload, certainty)?;
        Ok(Self {
            observation,
            elements,
            relations,
        })
    }

    /// Revalidate canonical evidence ordering, references, payload equality,
    /// and the underlying observation identity.
    ///
    /// # Errors
    ///
    /// Returns the exact structural, payload, or observation failure.
    pub fn verify(
        &self,
        limits: RelationalObservationLimits,
    ) -> Result<(), RelationalObservationError> {
        let mut elements = self.elements.clone();
        let mut relations = self.relations.clone();
        normalize_relational_evidence(&mut elements, &mut relations, limits)?;
        if elements != self.elements
            || relations != self.relations
            || self.observation.payload != relational_payload(&elements, &relations)
        {
            return Err(RelationalObservationError::PayloadMismatch);
        }
        self.observation.verify()?;
        Ok(())
    }
}

fn normalize_relational_evidence(
    elements: &mut [EvidenceElement],
    relations: &mut [EvidenceRelation],
    limits: RelationalObservationLimits,
) -> Result<(), RelationalObservationError> {
    check_relational_capacity(
        "elements",
        elements.len(),
        usize::try_from(limits.maximum_elements).unwrap_or(usize::MAX),
    )?;
    check_relational_capacity(
        "relations",
        relations.len(),
        usize::try_from(limits.maximum_relations).unwrap_or(usize::MAX),
    )?;
    elements.sort_unstable();
    if let Some(duplicate) = elements
        .windows(2)
        .find(|pair| pair[0].id == pair[1].id)
        .map(|pair| pair[0].id)
    {
        return Err(RelationalObservationError::DuplicateElement(duplicate));
    }
    let ids = elements
        .iter()
        .map(|element| element.id)
        .collect::<BTreeSet<_>>();
    for relation in relations.iter() {
        if relation.participants.is_empty() {
            return Err(RelationalObservationError::EmptyRelation);
        }
        check_relational_capacity(
            "relation_arity",
            relation.participants.len(),
            usize::from(limits.maximum_relation_arity),
        )?;
        if let Some(unknown) = relation
            .participants
            .iter()
            .find(|participant| !ids.contains(participant))
        {
            return Err(RelationalObservationError::UnknownParticipant(*unknown));
        }
    }
    relations.sort_unstable();
    if relations.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(RelationalObservationError::DuplicateRelation);
    }
    Ok(())
}

fn check_relational_capacity(
    resource: &'static str,
    requested: usize,
    maximum: usize,
) -> Result<(), RelationalObservationError> {
    if requested > maximum {
        return Err(RelationalObservationError::Capacity {
            resource,
            requested: u64::try_from(requested).unwrap_or(u64::MAX),
            maximum: u64::try_from(maximum).unwrap_or(u64::MAX),
        });
    }
    Ok(())
}

fn relational_payload(
    elements: &[EvidenceElement],
    relations: &[EvidenceRelation],
) -> CanonicalValue {
    CanonicalValue::Array(vec![
        CanonicalValue::Text("cairn.relational-observation.v1".to_owned()),
        CanonicalValue::Array(
            elements
                .iter()
                .map(|element| {
                    CanonicalValue::Array(vec![
                        CanonicalValue::Bytes(element.id.0.as_bytes().to_vec()),
                        CanonicalValue::Unsigned(evidence_type_code(element.value_type)),
                        CanonicalValue::Bytes(element.value.as_bytes().to_vec()),
                    ])
                })
                .collect(),
        ),
        CanonicalValue::Array(
            relations
                .iter()
                .map(|relation| {
                    CanonicalValue::Array(vec![
                        CanonicalValue::Bytes(relation.predicate.as_bytes().to_vec()),
                        CanonicalValue::Array(
                            relation
                                .participants
                                .iter()
                                .map(|participant| {
                                    CanonicalValue::Bytes(participant.0.as_bytes().to_vec())
                                })
                                .collect(),
                        ),
                    ])
                })
                .collect(),
        ),
    ])
}

const fn evidence_type_code(value_type: EvidenceValueType) -> u64 {
    match value_type {
        EvidenceValueType::Boolean => 0,
        EvidenceValueType::Integer => 1,
        EvidenceValueType::Atom => 2,
        EvidenceValueType::Entity => 3,
        EvidenceValueType::EntitySet => 4,
        EvidenceValueType::Relation => 5,
        EvidenceValueType::Action => 6,
        EvidenceValueType::Effect => 7,
        EvidenceValueType::Mechanism => 8,
    }
}

fn validate_fields(
    modality: &ModalityId,
    source: &SourceReference,
    certainty: ObservationCertainty,
) -> Result<(), ObservationError> {
    if modality.0.is_empty() {
        return Err(ObservationError::EmptyModality);
    }
    if source.locator.is_empty() {
        return Err(ObservationError::EmptySource);
    }
    if let ObservationCertainty::PartsPerMillion(value) = certainty {
        if value > 1_000_000 {
            return Err(ObservationError::InvalidConfidence(value));
        }
    }
    Ok(())
}

fn observation_id(
    modality: &ModalityId,
    logical_time: Option<u64>,
    source: &SourceReference,
    payload: &CanonicalValue,
    certainty: ObservationCertainty,
) -> Result<ObservationId, CanonicalError> {
    let time = logical_time.map_or(CanonicalValue::Null, CanonicalValue::Unsigned);
    let certainty = match certainty {
        ObservationCertainty::Exact => {
            CanonicalValue::Array(vec![CanonicalValue::Text("exact".to_owned())])
        }
        ObservationCertainty::PartsPerMillion(value) => CanonicalValue::Array(vec![
            CanonicalValue::Text("ppm".to_owned()),
            CanonicalValue::Unsigned(u64::from(value)),
        ]),
    };
    let envelope = CanonicalValue::Array(vec![
        CanonicalValue::Text("cairn.observation.v1".to_owned()),
        CanonicalValue::Text(modality.0.clone()),
        time,
        CanonicalValue::Text(source.locator.clone()),
        CanonicalValue::Bytes(source.content_hash.as_bytes().to_vec()),
        certainty,
        payload.clone(),
    ]);
    Ok(ObservationId(Hash32::digest(&envelope.encode_canonical()?)))
}
