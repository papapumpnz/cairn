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
    AttributedGroupingEntity, AttributedGroupingTrace, LearnedPredicateGroupingMerge,
    PredicateGroupingError, PredicateGroupingInventor, PredicateGroupingLimits,
};
use cairn_verifier::{
    PredicateGroupingPromotionError, PredicateGroupingVerifier, VerifiedPredicateGroupingPromotion,
    VerifiedPredicateGroupingReuse,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

pub const PREDICATE_GROUPING_COHORT_SCHEMA: &str = "cairn.memory.predicate-grouping-cohort";
pub const PREDICATE_GROUPING_COHORT_SCHEMA_VERSION: u32 = 1;

/// One proposal-only executable predicate/merge and its opaque independent
/// reuse evidence.
#[derive(Clone, Debug, PartialEq)]
pub struct PredicateGroupingModule {
    id: Hash32,
    mechanism: LearnedPredicateGroupingMerge,
    receipts: BTreeMap<Hash32, VerifiedPredicateGroupingReuse>,
    experiences: BTreeSet<Hash32>,
    status: ModuleStatus,
}

impl PredicateGroupingModule {
    #[must_use]
    pub const fn id(&self) -> Hash32 {
        self.id
    }
    #[must_use]
    pub const fn mechanism(&self) -> &LearnedPredicateGroupingMerge {
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
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PredicateGroupingStoreError {
    Canonicalization,
    IdentityMismatch,
    UnknownModule,
    EmptyReplay,
    DuplicateReceipt,
    OverlappingExperience,
}

/// Proof-gated memory for an invented executable pair predicate composed with
/// its learned aggregate. Persistent entries remain proposal sources only.
#[derive(Clone, Debug, Default)]
pub struct PredicateGroupingStore {
    modules: BTreeMap<Hash32, PredicateGroupingModule>,
}

impl PredicateGroupingStore {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            modules: BTreeMap::new(),
        }
    }

    /// Insert task-local learned structure without granting persistence.
    ///
    /// # Errors
    ///
    /// Rejects canonicalization failure or an identity collision.
    pub fn insert_provisional(
        &mut self,
        mechanism: LearnedPredicateGroupingMerge,
    ) -> Result<Hash32, PredicateGroupingStoreError> {
        let id = mechanism
            .semantic_hash()
            .map_err(|_| PredicateGroupingStoreError::Canonicalization)?;
        if let Some(existing) = self.modules.get(&id) {
            if existing.mechanism.semantic_hash().ok() != Some(id) {
                return Err(PredicateGroupingStoreError::IdentityMismatch);
            }
            return Ok(id);
        }
        self.modules.insert(
            id,
            PredicateGroupingModule {
                id,
                mechanism,
                receipts: BTreeMap::new(),
                experiences: BTreeSet::new(),
                status: ModuleStatus::Provisional,
            },
        );
        Ok(id)
    }

    /// Record one disjoint opaque verifier receipt.
    ///
    /// # Errors
    ///
    /// Rejects unknown/mismatched mechanisms, empty evidence, duplicate
    /// receipts, or overlapping experiences.
    pub fn record_verified_reuse(
        &mut self,
        id: Hash32,
        receipt: VerifiedPredicateGroupingReuse,
    ) -> Result<(), PredicateGroupingStoreError> {
        let module = self
            .modules
            .get_mut(&id)
            .ok_or(PredicateGroupingStoreError::UnknownModule)?;
        if receipt.mechanism() != id {
            return Err(PredicateGroupingStoreError::IdentityMismatch);
        }
        if receipt.replayed_groups() == 0 || receipt.experiences().is_empty() {
            return Err(PredicateGroupingStoreError::EmptyReplay);
        }
        let receipt_id = Hash32::digest(
            &receipt
                .experiences()
                .iter()
                .flat_map(|experience| experience.as_bytes().iter().copied())
                .collect::<Vec<_>>(),
        );
        if module.receipts.contains_key(&receipt_id) {
            return Err(PredicateGroupingStoreError::DuplicateReceipt);
        }
        if !module.experiences.is_disjoint(receipt.experiences()) {
            return Err(PredicateGroupingStoreError::OverlappingExperience);
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
        promotion: &VerifiedPredicateGroupingPromotion,
    ) -> bool {
        let Some(module) = self.modules.get_mut(&id) else {
            return false;
        };
        if module.status != ModuleStatus::Provisional
            || promotion.mechanism() != id
            || promotion.experiences() != &module.experiences
            || usize::try_from(promotion.verified_reuses()).ok() != Some(module.receipts.len())
            || promotion.verified_reuses() < 2
        {
            return false;
        }
        module.status = ModuleStatus::Persistent;
        true
    }

    pub fn persistent_mechanisms(&self) -> impl Iterator<Item = &LearnedPredicateGroupingMerge> {
        self.modules
            .values()
            .filter(|module| module.status == ModuleStatus::Persistent)
            .map(|module| &module.mechanism)
    }

    #[must_use]
    pub fn module(&self, id: Hash32) -> Option<&PredicateGroupingModule> {
        self.modules.get(&id)
    }
}

/// Canonical, untrusted evidence sufficient to reconstruct one invented
/// predicate/aggregate cohort after process loss. It contains no status,
/// verifier receipt, or promotion authority.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PredicateGroupingCohortArtifact {
    mechanism: Hash32,
    traces: Vec<AttributedGroupingTrace>,
    replayed_groups: u64,
    candidate_executions_without: u64,
    candidate_executions_with: u64,
}

impl PredicateGroupingCohortArtifact {
    #[must_use]
    pub const fn mechanism(&self) -> Hash32 {
        self.mechanism
    }

    #[must_use]
    pub fn traces(&self) -> &[AttributedGroupingTrace] {
        &self.traces
    }

    fn from_verified(
        learned: &LearnedPredicateGroupingMerge,
        traces: &[AttributedGroupingTrace],
        receipt: &VerifiedPredicateGroupingReuse,
    ) -> Result<Self, PredicateGroupingCohortArchiveError> {
        let mechanism = learned
            .semantic_hash()
            .map_err(|_| PredicateGroupingCohortArchiveError::Canonicalization)?;
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
            return Err(PredicateGroupingCohortArchiveError::IdentityMismatch);
        }
        let mut traces = traces.to_vec();
        traces.sort_by_key(|trace| trace.episode);
        Ok(Self {
            mechanism,
            traces,
            replayed_groups: receipt.replayed_groups(),
            candidate_executions_without: receipt.candidate_executions_without(),
            candidate_executions_with: receipt.candidate_executions_with(),
        })
    }

    fn cohort_hash(&self) -> Result<Hash32, PredicateGroupingCohortArchiveError> {
        self.encode_canonical()
            .map(|bytes| Hash32::digest(&bytes))
            .map_err(|_| PredicateGroupingCohortArchiveError::Canonicalization)
    }

    fn from_value(value: CanonicalValue) -> Result<Self, PredicateGroupingCohortArchiveError> {
        let CanonicalValue::Array(fields) = value else {
            return Err(PredicateGroupingCohortArchiveError::InvalidPayload);
        };
        let [marker, mechanism, traces, replayed, candidates_without, candidates_with] =
            fields.as_slice()
        else {
            return Err(PredicateGroupingCohortArchiveError::InvalidPayload);
        };
        if marker != &CanonicalValue::Text("cairn.memory.predicate-grouping-cohort.v1".to_owned()) {
            return Err(PredicateGroupingCohortArchiveError::InvalidPayload);
        }
        let CanonicalValue::Array(trace_values) = traces else {
            return Err(PredicateGroupingCohortArchiveError::InvalidPayload);
        };
        let mut traces = trace_values
            .iter()
            .map(decode_predicate_trace)
            .collect::<Result<Vec<_>, _>>()?;
        traces.sort_by_key(|trace| trace.episode);
        let experiences = traces
            .iter()
            .map(|trace| trace.episode)
            .collect::<BTreeSet<_>>();
        if traces.len() < 2 || experiences.len() != traces.len() {
            return Err(PredicateGroupingCohortArchiveError::InvalidPayload);
        }
        Ok(Self {
            mechanism: decode_predicate_hash(mechanism)?,
            traces,
            replayed_groups: decode_predicate_u64(replayed)?,
            candidate_executions_without: decode_predicate_u64(candidates_without)?,
            candidate_executions_with: decode_predicate_u64(candidates_with)?,
        })
    }
}

impl CanonicalEncode for PredicateGroupingCohortArtifact {
    fn encode_canonical(&self) -> Result<Vec<u8>, cairn_core::CanonicalError> {
        CanonicalValue::Array(vec![
            CanonicalValue::Text("cairn.memory.predicate-grouping-cohort.v1".to_owned()),
            CanonicalValue::Bytes(self.mechanism.as_bytes().to_vec()),
            CanonicalValue::Array(self.traces.iter().map(encode_predicate_trace).collect()),
            CanonicalValue::Unsigned(self.replayed_groups),
            CanonicalValue::Unsigned(self.candidate_executions_without),
            CanonicalValue::Unsigned(self.candidate_executions_with),
        ])
        .encode_canonical()
    }
}

/// Fresh authority reconstructed from one untrusted persisted cohort.
#[derive(Clone, Debug, PartialEq)]
pub struct ReplayedPredicateGroupingCohort {
    pub learned: LearnedPredicateGroupingMerge,
    pub receipt: VerifiedPredicateGroupingReuse,
}

/// CAS archive for invented predicate evidence. Loading never restores trust.
#[derive(Clone, Copy, Debug, Default)]
pub struct PredicateGroupingCohortArchive;

impl PredicateGroupingCohortArchive {
    /// Persist one independently learned and replayed cohort.
    ///
    /// # Errors
    ///
    /// Rejects missing lineage, identity mismatch, or canonical storage errors.
    pub fn persist(
        &self,
        cas: &Cas,
        learned: &LearnedPredicateGroupingMerge,
        traces: &[AttributedGroupingTrace],
        receipt: &VerifiedPredicateGroupingReuse,
        metadata: ReuseArtifactMetadata,
    ) -> Result<StoredArtifact, PredicateGroupingCohortArchiveError> {
        if metadata.parent_ids.is_empty() {
            return Err(PredicateGroupingCohortArchiveError::MissingParent);
        }
        let payload = PredicateGroupingCohortArtifact::from_verified(learned, traces, receipt)?;
        let scope = payload.cohort_hash()?;
        let envelope = ArtifactEnvelope::new(
            predicate_cohort_schema()?,
            predicate_cohort_kind()?,
            predicate_cohort_semantics(),
            metadata.parent_ids,
            Some(scope),
            metadata.producer,
            metadata.budget,
            metadata.created_at_logical,
            payload,
        )?;
        cas.put_envelope(&envelope)
            .map_err(PredicateGroupingCohortArchiveError::Storage)
    }

    /// Load structurally checked but untrusted evidence.
    ///
    /// # Errors
    ///
    /// Rejects envelope, schema, semantics, lineage, hash, payload, or bound
    /// violations.
    pub fn load(
        &self,
        cas: &Cas,
        blob_id: BlobId,
    ) -> Result<PredicateGroupingCohortArtifact, PredicateGroupingCohortArchiveError> {
        let mut registry = SchemaRegistry::new();
        registry.register(SchemaDescriptor {
            name: PREDICATE_GROUPING_COHORT_SCHEMA.to_owned(),
            current_version: PREDICATE_GROUPING_COHORT_SCHEMA_VERSION,
            additive_read_versions: BTreeSet::new(),
            maximum_payload_bytes: 8_388_608,
        })?;
        let envelope = cas.get_envelope(
            blob_id,
            &registry,
            &BTreeSet::from([predicate_cohort_semantics()]),
            CanonicalLimits {
                max_input_bytes: 10_485_760,
                max_depth: 32,
                max_collection_items: 262_144,
                max_scalar_bytes: 8_388_608,
            },
        )?;
        if envelope.artifact_kind != predicate_cohort_kind()? || envelope.parent_ids.is_empty() {
            return Err(PredicateGroupingCohortArchiveError::InvalidEnvelope);
        }
        let artifact = PredicateGroupingCohortArtifact::from_value(envelope.payload)?;
        if envelope.task_scope != Some(artifact.cohort_hash()?) {
            return Err(PredicateGroupingCohortArchiveError::InvalidEnvelope);
        }
        Ok(artifact)
    }

    /// Re-invent and independently replay loaded evidence from a blank process.
    ///
    /// # Errors
    ///
    /// Rejects changed identity/work, invention failure, or verifier failure.
    pub fn replay(
        &self,
        artifact: &PredicateGroupingCohortArtifact,
        limits: PredicateGroupingLimits,
        invention_limits: InventionLimits,
        execution_limits: ExecLimits,
    ) -> Result<ReplayedPredicateGroupingCohort, PredicateGroupingCohortArchiveError> {
        let learned = PredicateGroupingInventor
            .invent(artifact.traces(), limits, invention_limits)
            .map_err(PredicateGroupingCohortArchiveError::Invention)?;
        if learned
            .semantic_hash()
            .map_err(|_| PredicateGroupingCohortArchiveError::Canonicalization)?
            != artifact.mechanism
        {
            return Err(PredicateGroupingCohortArchiveError::IdentityMismatch);
        }
        let receipt = PredicateGroupingVerifier
            .verify_reuse(
                &learned,
                artifact.traces(),
                limits,
                execution_limits,
                artifact.candidate_executions_without,
                artifact.candidate_executions_with,
            )
            .map_err(PredicateGroupingCohortArchiveError::Verification)?;
        if receipt.mechanism() != artifact.mechanism
            || receipt.replayed_groups() != artifact.replayed_groups
            || receipt.candidate_executions_without() != artifact.candidate_executions_without
            || receipt.candidate_executions_with() != artifact.candidate_executions_with
        {
            return Err(PredicateGroupingCohortArchiveError::ReplayMismatch);
        }
        Ok(ReplayedPredicateGroupingCohort { learned, receipt })
    }
}

#[derive(Debug)]
pub enum PredicateGroupingCohortArchiveError {
    MissingParent,
    InvalidEnvelope,
    InvalidPayload,
    Canonicalization,
    IdentityMismatch,
    ReplayMismatch,
    Invention(PredicateGroupingError),
    Verification(PredicateGroupingPromotionError),
    Artifact(ArtifactError),
    Storage(CasError),
}

impl fmt::Display for PredicateGroupingCohortArchiveError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for PredicateGroupingCohortArchiveError {}

impl From<ArtifactError> for PredicateGroupingCohortArchiveError {
    fn from(value: ArtifactError) -> Self {
        Self::Artifact(value)
    }
}

impl From<CasError> for PredicateGroupingCohortArchiveError {
    fn from(value: CasError) -> Self {
        Self::Storage(value)
    }
}

fn encode_predicate_trace(trace: &AttributedGroupingTrace) -> CanonicalValue {
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
                        predicate_signed(entity.value),
                        CanonicalValue::Array(
                            entity
                                .features
                                .iter()
                                .copied()
                                .map(predicate_signed)
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
                .map(predicate_signed)
                .collect(),
        ),
    ])
}

fn decode_predicate_trace(
    value: &CanonicalValue,
) -> Result<AttributedGroupingTrace, PredicateGroupingCohortArchiveError> {
    let CanonicalValue::Array(fields) = value else {
        return Err(PredicateGroupingCohortArchiveError::InvalidPayload);
    };
    let [episode, entities, outputs] = fields.as_slice() else {
        return Err(PredicateGroupingCohortArchiveError::InvalidPayload);
    };
    let CanonicalValue::Array(entity_values) = entities else {
        return Err(PredicateGroupingCohortArchiveError::InvalidPayload);
    };
    let entities = entity_values
        .iter()
        .map(|value| {
            let CanonicalValue::Array(fields) = value else {
                return Err(PredicateGroupingCohortArchiveError::InvalidPayload);
            };
            let [id, value, features] = fields.as_slice() else {
                return Err(PredicateGroupingCohortArchiveError::InvalidPayload);
            };
            let CanonicalValue::Array(feature_values) = features else {
                return Err(PredicateGroupingCohortArchiveError::InvalidPayload);
            };
            Ok(AttributedGroupingEntity {
                id: EntityId(decode_predicate_hash(id)?),
                value: decode_predicate_i64(value)?,
                features: feature_values
                    .iter()
                    .map(decode_predicate_i64)
                    .collect::<Result<Vec<_>, _>>()?,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let ids = entities
        .iter()
        .map(|entity| &entity.id)
        .collect::<BTreeSet<_>>();
    if entities.len() != entity_values.len() || ids.len() != entities.len() {
        return Err(PredicateGroupingCohortArchiveError::InvalidPayload);
    }
    let CanonicalValue::Array(output_values) = outputs else {
        return Err(PredicateGroupingCohortArchiveError::InvalidPayload);
    };
    let outputs = output_values
        .iter()
        .map(decode_predicate_i64)
        .collect::<Result<BTreeSet<_>, _>>()?;
    if outputs.len() != output_values.len() {
        return Err(PredicateGroupingCohortArchiveError::InvalidPayload);
    }
    Ok(AttributedGroupingTrace {
        episode: decode_predicate_hash(episode)?,
        entities,
        outputs,
    })
}

const fn predicate_signed(value: i64) -> CanonicalValue {
    if value >= 0 {
        CanonicalValue::Unsigned(value.unsigned_abs())
    } else {
        CanonicalValue::Negative(value)
    }
}

fn decode_predicate_hash(
    value: &CanonicalValue,
) -> Result<Hash32, PredicateGroupingCohortArchiveError> {
    let CanonicalValue::Bytes(bytes) = value else {
        return Err(PredicateGroupingCohortArchiveError::InvalidPayload);
    };
    let fixed: [u8; 32] = bytes
        .as_slice()
        .try_into()
        .map_err(|_| PredicateGroupingCohortArchiveError::InvalidPayload)?;
    Ok(Hash32::from_bytes(fixed))
}

fn decode_predicate_i64(
    value: &CanonicalValue,
) -> Result<i64, PredicateGroupingCohortArchiveError> {
    match value {
        CanonicalValue::Unsigned(value) => {
            i64::try_from(*value).map_err(|_| PredicateGroupingCohortArchiveError::InvalidPayload)
        }
        CanonicalValue::Negative(value) if *value < 0 => Ok(*value),
        _ => Err(PredicateGroupingCohortArchiveError::InvalidPayload),
    }
}

fn decode_predicate_u64(
    value: &CanonicalValue,
) -> Result<u64, PredicateGroupingCohortArchiveError> {
    let CanonicalValue::Unsigned(value) = value else {
        return Err(PredicateGroupingCohortArchiveError::InvalidPayload);
    };
    Ok(*value)
}

fn predicate_cohort_schema() -> Result<SchemaRef, ArtifactError> {
    SchemaRef::new(
        PREDICATE_GROUPING_COHORT_SCHEMA,
        PREDICATE_GROUPING_COHORT_SCHEMA_VERSION,
    )
}

fn predicate_cohort_kind() -> Result<ArtifactKind, ArtifactError> {
    ArtifactKind::new("memory.predicate-grouping-cohort")
}

const fn predicate_cohort_semantics() -> SemanticsVersion {
    SemanticsVersion::new(1, 0, 0)
}
