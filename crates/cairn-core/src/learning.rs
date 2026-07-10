use crate::{CanonicalEncode, CanonicalError, CanonicalValue, Hash32};

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
