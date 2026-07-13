use crate::{ModuleStatus, ReuseArtifactMetadata};
use cairn_cas::{Cas, CasError, StoredArtifact};
use cairn_core::{
    ArtifactEnvelope, ArtifactError, ArtifactKind, BlobId, CanonicalEncode, CanonicalLimits,
    CanonicalValue, EntityId, Hash32, SchemaDescriptor, SchemaRef, SchemaRegistry,
    SemanticsVersion,
};
use cairn_invent::InventionLimits;
use cairn_rkl::ExecLimits;
use cairn_theory::{
    LearnedObservationPredicateGrouping, ObservationPredicateError, ObservationPredicateInventor,
    ObservationPredicateLimits, PredicateGroupingLimits, PrimitivePairEntity, PrimitivePairTrace,
};
use cairn_verifier::{
    ObservationPredicatePromotionError, ObservationPredicateSupportGeometry,
    ObservationPredicateVerifier, VerifiedObservationPredicatePromotion,
    VerifiedObservationPredicateReuse,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

pub const OBSERVATION_PREDICATE_COHORT_SCHEMA: &str = "cairn.memory.observation-predicate-cohort";
pub const OBSERVATION_PREDICATE_COHORT_SCHEMA_VERSION: u32 = 2;

#[derive(Clone, Debug, PartialEq)]
pub struct ObservationPredicateModule {
    id: Hash32,
    mechanism: LearnedObservationPredicateGrouping,
    receipts: BTreeMap<Hash32, VerifiedObservationPredicateReuse>,
    experiences: BTreeSet<Hash32>,
    status: ModuleStatus,
    support: Option<ObservationPredicateSupportGeometry>,
}

impl ObservationPredicateModule {
    #[must_use]
    pub const fn id(&self) -> Hash32 {
        self.id
    }
    #[must_use]
    pub const fn mechanism(&self) -> &LearnedObservationPredicateGrouping {
        &self.mechanism
    }
    #[must_use]
    pub const fn status(&self) -> ModuleStatus {
        self.status
    }
    #[must_use]
    pub fn verified_reuses(&self) -> usize {
        self.receipts.len()
    }
    #[must_use]
    pub const fn support(&self) -> Option<ObservationPredicateSupportGeometry> {
        self.support
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ObservationPredicateStoreError {
    Canonicalization,
    IdentityMismatch,
    UnknownModule,
    EmptyReplay,
    DuplicateReceipt,
    OverlappingExperience,
}

/// Proof-gated memory binding executable observation features, relation, and
/// aggregate. Persistent entries remain proposal sources only.
#[derive(Clone, Debug, Default)]
pub struct ObservationPredicateStore {
    modules: BTreeMap<Hash32, ObservationPredicateModule>,
}

impl ObservationPredicateStore {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            modules: BTreeMap::new(),
        }
    }

    /// Insert one task-local learned composition provisionally.
    ///
    /// # Errors
    ///
    /// Rejects canonicalization failure or identity collision.
    pub fn insert_provisional(
        &mut self,
        mechanism: LearnedObservationPredicateGrouping,
    ) -> Result<Hash32, ObservationPredicateStoreError> {
        let id = mechanism
            .semantic_hash()
            .map_err(|_| ObservationPredicateStoreError::Canonicalization)?;
        if let Some(existing) = self.modules.get(&id) {
            if existing.mechanism.semantic_hash().ok() != Some(id) {
                return Err(ObservationPredicateStoreError::IdentityMismatch);
            }
            return Ok(id);
        }
        self.modules.insert(
            id,
            ObservationPredicateModule {
                id,
                mechanism,
                receipts: BTreeMap::new(),
                experiences: BTreeSet::new(),
                status: ModuleStatus::Provisional,
                support: None,
            },
        );
        Ok(id)
    }

    /// Add one disjoint opaque verifier receipt.
    ///
    /// # Errors
    ///
    /// Rejects unknown/mismatched identity, empty replay, duplicates, or
    /// overlapping experiences.
    pub fn record_verified_reuse(
        &mut self,
        id: Hash32,
        receipt: VerifiedObservationPredicateReuse,
    ) -> Result<(), ObservationPredicateStoreError> {
        let module = self
            .modules
            .get_mut(&id)
            .ok_or(ObservationPredicateStoreError::UnknownModule)?;
        if receipt.mechanism() != id {
            return Err(ObservationPredicateStoreError::IdentityMismatch);
        }
        if receipt.replayed_groups() == 0 || receipt.experiences().is_empty() {
            return Err(ObservationPredicateStoreError::EmptyReplay);
        }
        let receipt_id = Hash32::digest(
            &receipt
                .experiences()
                .iter()
                .flat_map(|experience| experience.as_bytes().iter().copied())
                .collect::<Vec<_>>(),
        );
        if module.receipts.contains_key(&receipt_id) {
            return Err(ObservationPredicateStoreError::DuplicateReceipt);
        }
        if !module.experiences.is_disjoint(receipt.experiences()) {
            return Err(ObservationPredicateStoreError::OverlappingExperience);
        }
        module.experiences.extend(receipt.experiences());
        module.receipts.insert(receipt_id, receipt);
        Ok(())
    }

    /// Apply only a consolidator token covering the complete local ledger.
    #[must_use]
    pub fn apply_verified_promotion(
        &mut self,
        id: Hash32,
        promotion: &VerifiedObservationPredicatePromotion,
    ) -> bool {
        let Some(module) = self.modules.get_mut(&id) else {
            return false;
        };
        if module.status != ModuleStatus::Provisional
            || promotion.mechanism() != id
            || promotion.experiences() != &module.experiences
            || usize::try_from(promotion.verified_reuses()).ok() != Some(module.receipts.len())
            || promotion.verified_reuses() < 2
            || promotion.support().observation_arity != module.mechanism.observation_arity
        {
            return false;
        }
        module.status = ModuleStatus::Persistent;
        module.support = Some(promotion.support());
        true
    }

    pub fn persistent_mechanisms(
        &self,
    ) -> impl Iterator<Item = &LearnedObservationPredicateGrouping> {
        self.modules
            .values()
            .filter(|module| module.status == ModuleStatus::Persistent)
            .map(|module| &module.mechanism)
    }

    pub fn persistent_modules(&self) -> impl Iterator<Item = &ObservationPredicateModule> {
        self.modules
            .values()
            .filter(|module| module.status == ModuleStatus::Persistent)
    }

    #[must_use]
    pub fn module(&self, id: Hash32) -> Option<&ObservationPredicateModule> {
        self.modules.get(&id)
    }
}

/// Canonical evidence from which one observation-feature mechanism can be
/// re-invented after process loss. It deliberately contains no status,
/// verifier receipt, promotion certificate, or executable learned object.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObservationPredicateCohortArtifact {
    mechanism: Hash32,
    traces: Vec<PrimitivePairTrace>,
    replayed_groups: u64,
    candidate_executions_without: u64,
    candidate_executions_with: u64,
}

impl ObservationPredicateCohortArtifact {
    #[must_use]
    pub const fn mechanism(&self) -> Hash32 {
        self.mechanism
    }

    #[must_use]
    pub fn traces(&self) -> &[PrimitivePairTrace] {
        &self.traces
    }

    fn from_verified(
        learned: &LearnedObservationPredicateGrouping,
        traces: &[PrimitivePairTrace],
        receipt: &VerifiedObservationPredicateReuse,
    ) -> Result<Self, ObservationPredicateCohortArchiveError> {
        let mechanism = learned
            .semantic_hash()
            .map_err(|_| ObservationPredicateCohortArchiveError::Canonicalization)?;
        let experiences = traces
            .iter()
            .map(|trace| trace.episode)
            .collect::<BTreeSet<_>>();
        if traces.len() < 2
            || experiences.len() != traces.len()
            || mechanism != receipt.mechanism()
            || &experiences != receipt.experiences()
            || receipt.replayed_groups() == 0
        {
            return Err(ObservationPredicateCohortArchiveError::IdentityMismatch);
        }
        let mut traces = traces.to_vec();
        canonicalize_observation_traces(&mut traces);
        Ok(Self {
            mechanism,
            traces,
            replayed_groups: receipt.replayed_groups(),
            candidate_executions_without: receipt.candidate_executions_without(),
            candidate_executions_with: receipt.candidate_executions_with(),
        })
    }

    fn cohort_hash(&self) -> Result<Hash32, ObservationPredicateCohortArchiveError> {
        self.encode_canonical()
            .map(|bytes| Hash32::digest(&bytes))
            .map_err(|_| ObservationPredicateCohortArchiveError::Canonicalization)
    }

    fn from_value(value: CanonicalValue) -> Result<Self, ObservationPredicateCohortArchiveError> {
        let CanonicalValue::Array(fields) = value else {
            return Err(ObservationPredicateCohortArchiveError::InvalidPayload);
        };
        let [marker, mechanism, traces, replayed, candidates_without, candidates_with] =
            fields.as_slice()
        else {
            return Err(ObservationPredicateCohortArchiveError::InvalidPayload);
        };
        if marker
            != &CanonicalValue::Text("cairn.memory.observation-predicate-cohort.v2".to_owned())
        {
            return Err(ObservationPredicateCohortArchiveError::InvalidPayload);
        }
        let CanonicalValue::Array(trace_values) = traces else {
            return Err(ObservationPredicateCohortArchiveError::InvalidPayload);
        };
        let mut traces = trace_values
            .iter()
            .map(decode_observation_trace)
            .collect::<Result<Vec<_>, _>>()?;
        canonicalize_observation_traces(&mut traces);
        let experiences = traces
            .iter()
            .map(|trace| trace.episode)
            .collect::<BTreeSet<_>>();
        if traces.len() < 2 || experiences.len() != traces.len() {
            return Err(ObservationPredicateCohortArchiveError::InvalidPayload);
        }
        Ok(Self {
            mechanism: decode_observation_hash(mechanism)?,
            traces,
            replayed_groups: decode_observation_u64(replayed)?,
            candidate_executions_without: decode_observation_u64(candidates_without)?,
            candidate_executions_with: decode_observation_u64(candidates_with)?,
        })
    }
}

impl CanonicalEncode for ObservationPredicateCohortArtifact {
    fn encode_canonical(&self) -> Result<Vec<u8>, cairn_core::CanonicalError> {
        CanonicalValue::Array(vec![
            CanonicalValue::Text("cairn.memory.observation-predicate-cohort.v2".to_owned()),
            CanonicalValue::Bytes(self.mechanism.as_bytes().to_vec()),
            CanonicalValue::Array(self.traces.iter().map(encode_observation_trace).collect()),
            CanonicalValue::Unsigned(self.replayed_groups),
            CanonicalValue::Unsigned(self.candidate_executions_without),
            CanonicalValue::Unsigned(self.candidate_executions_with),
        ])
        .encode_canonical()
    }
}

/// Fresh learned structure and verifier authority reconstructed solely from
/// persisted evidence.
#[derive(Clone, Debug, PartialEq)]
pub struct ReplayedObservationPredicateCohort {
    pub learned: LearnedObservationPredicateGrouping,
    pub receipt: VerifiedObservationPredicateReuse,
}

/// Content-addressed archive for untrusted observation evidence. Loading
/// cannot restore learned or promotion authority.
#[derive(Clone, Copy, Debug, Default)]
pub struct ObservationPredicateCohortArchive;

impl ObservationPredicateCohortArchive {
    /// Persist one independently verified evidence cohort.
    ///
    /// # Errors
    ///
    /// Rejects missing lineage, mismatched evidence, or canonical storage
    /// failures.
    pub fn persist(
        &self,
        cas: &Cas,
        learned: &LearnedObservationPredicateGrouping,
        traces: &[PrimitivePairTrace],
        receipt: &VerifiedObservationPredicateReuse,
        metadata: ReuseArtifactMetadata,
    ) -> Result<StoredArtifact, ObservationPredicateCohortArchiveError> {
        if metadata.parent_ids.is_empty() {
            return Err(ObservationPredicateCohortArchiveError::MissingParent);
        }
        let payload = ObservationPredicateCohortArtifact::from_verified(learned, traces, receipt)?;
        let scope = payload.cohort_hash()?;
        let envelope = ArtifactEnvelope::new(
            observation_cohort_schema()?,
            observation_cohort_kind()?,
            observation_cohort_semantics(),
            metadata.parent_ids,
            Some(scope),
            metadata.producer,
            metadata.budget,
            metadata.created_at_logical,
            payload,
        )?;
        cas.put_envelope(&envelope)
            .map_err(ObservationPredicateCohortArchiveError::Storage)
    }

    /// Load structurally validated but untrusted evidence.
    ///
    /// # Errors
    ///
    /// Rejects schema, semantics, lineage, hash, payload, or bound failures.
    pub fn load(
        &self,
        cas: &Cas,
        blob_id: BlobId,
    ) -> Result<ObservationPredicateCohortArtifact, ObservationPredicateCohortArchiveError> {
        let mut registry = SchemaRegistry::new();
        registry.register(SchemaDescriptor {
            name: OBSERVATION_PREDICATE_COHORT_SCHEMA.to_owned(),
            current_version: OBSERVATION_PREDICATE_COHORT_SCHEMA_VERSION,
            additive_read_versions: BTreeSet::new(),
            maximum_payload_bytes: 8_388_608,
        })?;
        let envelope = cas.get_envelope(
            blob_id,
            &registry,
            &BTreeSet::from([observation_cohort_semantics()]),
            CanonicalLimits {
                max_input_bytes: 10_485_760,
                max_depth: 32,
                max_collection_items: 262_144,
                max_scalar_bytes: 8_388_608,
            },
        )?;
        if envelope.artifact_kind != observation_cohort_kind()? || envelope.parent_ids.is_empty() {
            return Err(ObservationPredicateCohortArchiveError::InvalidEnvelope);
        }
        let artifact = ObservationPredicateCohortArtifact::from_value(envelope.payload)?;
        if envelope.task_scope != Some(artifact.cohort_hash()?) {
            return Err(ObservationPredicateCohortArchiveError::InvalidEnvelope);
        }
        Ok(artifact)
    }

    /// Re-invent and independently replay loaded evidence from blank authority.
    ///
    /// # Errors
    ///
    /// Rejects invention, identity, work-accounting, or verifier mismatch.
    pub fn replay(
        &self,
        artifact: &ObservationPredicateCohortArtifact,
        limits: ObservationPredicateLimits,
        predicate_limits: PredicateGroupingLimits,
        invention_limits: InventionLimits,
        execution_limits: ExecLimits,
    ) -> Result<ReplayedObservationPredicateCohort, ObservationPredicateCohortArchiveError> {
        let learned = ObservationPredicateInventor
            .invent(
                artifact.traces(),
                limits,
                predicate_limits,
                invention_limits,
            )
            .map_err(ObservationPredicateCohortArchiveError::Invention)?;
        if learned
            .semantic_hash()
            .map_err(|_| ObservationPredicateCohortArchiveError::Canonicalization)?
            != artifact.mechanism
        {
            return Err(ObservationPredicateCohortArchiveError::IdentityMismatch);
        }
        let receipt = ObservationPredicateVerifier
            .verify_reuse(
                &learned,
                artifact.traces(),
                limits,
                predicate_limits,
                execution_limits,
            )
            .map_err(ObservationPredicateCohortArchiveError::Verification)?;
        if receipt.mechanism() != artifact.mechanism
            || receipt.replayed_groups() != artifact.replayed_groups
            || receipt.candidate_executions_without() != artifact.candidate_executions_without
            || receipt.candidate_executions_with() != artifact.candidate_executions_with
        {
            return Err(ObservationPredicateCohortArchiveError::ReplayMismatch);
        }
        Ok(ReplayedObservationPredicateCohort { learned, receipt })
    }
}

#[derive(Debug)]
pub enum ObservationPredicateCohortArchiveError {
    MissingParent,
    InvalidEnvelope,
    InvalidPayload,
    Canonicalization,
    IdentityMismatch,
    ReplayMismatch,
    Invention(ObservationPredicateError),
    Verification(ObservationPredicatePromotionError),
    Artifact(ArtifactError),
    Storage(CasError),
}

impl fmt::Display for ObservationPredicateCohortArchiveError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for ObservationPredicateCohortArchiveError {}

impl From<ArtifactError> for ObservationPredicateCohortArchiveError {
    fn from(value: ArtifactError) -> Self {
        Self::Artifact(value)
    }
}

impl From<CasError> for ObservationPredicateCohortArchiveError {
    fn from(value: CasError) -> Self {
        Self::Storage(value)
    }
}

fn canonicalize_observation_traces(traces: &mut [PrimitivePairTrace]) {
    for trace in traces.iter_mut() {
        trace.entities.sort_by(|left, right| left.id.cmp(&right.id));
    }
    traces.sort_by_key(|trace| trace.episode);
}

fn encode_observation_trace(trace: &PrimitivePairTrace) -> CanonicalValue {
    let mut entities = trace.entities.clone();
    entities.sort_by(|left, right| left.id.cmp(&right.id));
    CanonicalValue::Array(vec![
        CanonicalValue::Bytes(trace.episode.as_bytes().to_vec()),
        CanonicalValue::Array(
            entities
                .iter()
                .map(|entity| {
                    CanonicalValue::Array(vec![
                        CanonicalValue::Bytes(entity.id.0.as_bytes().to_vec()),
                        observation_signed(entity.value),
                        CanonicalValue::Array(
                            entity
                                .observations
                                .iter()
                                .copied()
                                .map(observation_signed)
                                .collect(),
                        ),
                    ])
                })
                .collect(),
        ),
        CanonicalValue::Array(
            trace
                .outputs
                .iter()
                .copied()
                .map(observation_signed)
                .collect(),
        ),
    ])
}

fn decode_observation_trace(
    value: &CanonicalValue,
) -> Result<PrimitivePairTrace, ObservationPredicateCohortArchiveError> {
    let CanonicalValue::Array(fields) = value else {
        return Err(ObservationPredicateCohortArchiveError::InvalidPayload);
    };
    let [episode, entities, outputs] = fields.as_slice() else {
        return Err(ObservationPredicateCohortArchiveError::InvalidPayload);
    };
    let CanonicalValue::Array(entity_values) = entities else {
        return Err(ObservationPredicateCohortArchiveError::InvalidPayload);
    };
    let entities = entity_values
        .iter()
        .map(|value| {
            let CanonicalValue::Array(fields) = value else {
                return Err(ObservationPredicateCohortArchiveError::InvalidPayload);
            };
            let [id, scalar, observations] = fields.as_slice() else {
                return Err(ObservationPredicateCohortArchiveError::InvalidPayload);
            };
            let CanonicalValue::Array(observation_values) = observations else {
                return Err(ObservationPredicateCohortArchiveError::InvalidPayload);
            };
            if observation_values.len() < 2 {
                return Err(ObservationPredicateCohortArchiveError::InvalidPayload);
            }
            Ok(PrimitivePairEntity {
                id: EntityId(decode_observation_hash(id)?),
                value: decode_observation_i64(scalar)?,
                observations: observation_values
                    .iter()
                    .map(decode_observation_i64)
                    .collect::<Result<Vec<_>, _>>()?,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let ids = entities
        .iter()
        .map(|entity| &entity.id)
        .collect::<BTreeSet<_>>();
    if ids.len() != entities.len() {
        return Err(ObservationPredicateCohortArchiveError::InvalidPayload);
    }
    let CanonicalValue::Array(output_values) = outputs else {
        return Err(ObservationPredicateCohortArchiveError::InvalidPayload);
    };
    let outputs = output_values
        .iter()
        .map(decode_observation_i64)
        .collect::<Result<BTreeSet<_>, _>>()?;
    if outputs.len() != output_values.len() {
        return Err(ObservationPredicateCohortArchiveError::InvalidPayload);
    }
    Ok(PrimitivePairTrace {
        episode: decode_observation_hash(episode)?,
        entities,
        outputs,
    })
}

const fn observation_signed(value: i64) -> CanonicalValue {
    if value >= 0 {
        CanonicalValue::Unsigned(value.unsigned_abs())
    } else {
        CanonicalValue::Negative(value)
    }
}

fn decode_observation_hash(
    value: &CanonicalValue,
) -> Result<Hash32, ObservationPredicateCohortArchiveError> {
    let CanonicalValue::Bytes(bytes) = value else {
        return Err(ObservationPredicateCohortArchiveError::InvalidPayload);
    };
    let fixed: [u8; 32] = bytes
        .as_slice()
        .try_into()
        .map_err(|_| ObservationPredicateCohortArchiveError::InvalidPayload)?;
    Ok(Hash32::from_bytes(fixed))
}

fn decode_observation_i64(
    value: &CanonicalValue,
) -> Result<i64, ObservationPredicateCohortArchiveError> {
    match value {
        CanonicalValue::Unsigned(value) => i64::try_from(*value)
            .map_err(|_| ObservationPredicateCohortArchiveError::InvalidPayload),
        CanonicalValue::Negative(value) if *value < 0 => Ok(*value),
        _ => Err(ObservationPredicateCohortArchiveError::InvalidPayload),
    }
}

fn decode_observation_u64(
    value: &CanonicalValue,
) -> Result<u64, ObservationPredicateCohortArchiveError> {
    let CanonicalValue::Unsigned(value) = value else {
        return Err(ObservationPredicateCohortArchiveError::InvalidPayload);
    };
    Ok(*value)
}

fn observation_cohort_schema() -> Result<SchemaRef, ArtifactError> {
    SchemaRef::new(
        OBSERVATION_PREDICATE_COHORT_SCHEMA,
        OBSERVATION_PREDICATE_COHORT_SCHEMA_VERSION,
    )
}

fn observation_cohort_kind() -> Result<ArtifactKind, ArtifactError> {
    ArtifactKind::new("memory.observation-predicate-cohort")
}

const fn observation_cohort_semantics() -> SemanticsVersion {
    SemanticsVersion::new(3, 0, 0)
}
