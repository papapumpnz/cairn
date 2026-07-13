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
    EntityObservationLimits, LearnedRawRelationObservationGrouping, ObservationPredicateLimits,
    PredicateGroupingLimits, RawAttributedObservationPrimitive, RawRelationMechanism,
    RawRelationObservationInventor, RawRelationObservationTrace, RelationObservationError,
    RelationObservationLimits,
};
use cairn_verifier::{
    RawRelationObservationPromotionError, RawRelationObservationVerifier,
    RawRelationSupportGeometry, VerifiedRawRelationObservationPromotion,
    VerifiedRawRelationObservationReuse,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

pub const RAW_RELATION_COHORT_SCHEMA: &str = "cairn.memory.raw-relation-cohort";
pub const RAW_RELATION_COHORT_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, PartialEq)]
pub struct RawRelationModule {
    id: Hash32,
    mechanism: RawRelationMechanism,
    receipts: BTreeMap<Hash32, VerifiedRawRelationObservationReuse>,
    experiences: BTreeSet<Hash32>,
    status: ModuleStatus,
    support: Option<RawRelationSupportGeometry>,
}

impl RawRelationModule {
    #[must_use]
    pub const fn id(&self) -> Hash32 {
        self.id
    }
    #[must_use]
    pub const fn mechanism(&self) -> &RawRelationMechanism {
        &self.mechanism
    }
    #[must_use]
    pub const fn status(&self) -> ModuleStatus {
        self.status
    }
    #[must_use]
    pub const fn support(&self) -> Option<RawRelationSupportGeometry> {
        self.support
    }
    #[must_use]
    pub fn verified_reuses(&self) -> usize {
        self.receipts.len()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RawRelationStoreError {
    Canonicalization,
    IdentityMismatch,
    UnknownModule,
    EmptyReplay,
    DuplicateReceipt,
    OverlappingExperience,
}

/// Proof-gated complete primitive-relation mechanisms. Persistent modules are
/// proposal sources only and retain no task-local attribute binding.
#[derive(Clone, Debug, Default)]
pub struct RawRelationStore {
    modules: BTreeMap<Hash32, RawRelationModule>,
}

impl RawRelationStore {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            modules: BTreeMap::new(),
        }
    }

    /// Insert a source-independent mechanism provisionally.
    ///
    /// # Errors
    ///
    /// Rejects canonicalization failure or semantic identity collision.
    pub fn insert_provisional(
        &mut self,
        mechanism: RawRelationMechanism,
    ) -> Result<Hash32, RawRelationStoreError> {
        let key_arity = usize::try_from(mechanism.key_arity)
            .map_err(|_| RawRelationStoreError::IdentityMismatch)?;
        if !mechanism.operator.accepts_arity(key_arity)
            || mechanism.relation_definition.parameters.len() != key_arity.saturating_mul(2)
        {
            return Err(RawRelationStoreError::IdentityMismatch);
        }
        let id = mechanism
            .semantic_hash()
            .map_err(|_| RawRelationStoreError::Canonicalization)?;
        if let Some(existing) = self.modules.get(&id) {
            if existing.mechanism.semantic_hash().ok() != Some(id) {
                return Err(RawRelationStoreError::IdentityMismatch);
            }
            return Ok(id);
        }
        self.modules.insert(
            id,
            RawRelationModule {
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

    /// Record one disjoint independent replay receipt.
    ///
    /// # Errors
    ///
    /// Rejects unknown identity, empty replay, duplicates, or overlapping
    /// episode evidence.
    pub fn record_verified_reuse(
        &mut self,
        id: Hash32,
        receipt: VerifiedRawRelationObservationReuse,
    ) -> Result<(), RawRelationStoreError> {
        let module = self
            .modules
            .get_mut(&id)
            .ok_or(RawRelationStoreError::UnknownModule)?;
        if receipt.mechanism() != id {
            return Err(RawRelationStoreError::IdentityMismatch);
        }
        if receipt.replayed_groups() == 0 || receipt.experiences().is_empty() {
            return Err(RawRelationStoreError::EmptyReplay);
        }
        let receipt_id = Hash32::digest(
            &receipt
                .experiences()
                .iter()
                .flat_map(|experience| experience.as_bytes().iter().copied())
                .collect::<Vec<_>>(),
        );
        if module.receipts.contains_key(&receipt_id) {
            return Err(RawRelationStoreError::DuplicateReceipt);
        }
        if !module.experiences.is_disjoint(receipt.experiences()) {
            return Err(RawRelationStoreError::OverlappingExperience);
        }
        module.experiences.extend(receipt.experiences());
        module.receipts.insert(receipt_id, receipt);
        Ok(())
    }

    /// Apply only a consolidator certificate covering the complete local
    /// evidence ledger.
    #[must_use]
    pub fn apply_verified_promotion(
        &mut self,
        id: Hash32,
        promotion: &VerifiedRawRelationObservationPromotion,
    ) -> bool {
        let Some(module) = self.modules.get_mut(&id) else {
            return false;
        };
        if module.status != ModuleStatus::Provisional
            || promotion.mechanism() != id
            || promotion.experiences() != &module.experiences
            || usize::try_from(promotion.verified_reuses()).ok() != Some(module.receipts.len())
            || promotion.verified_reuses() < 2
            || promotion.support().operator != module.mechanism.operator
            || promotion.support().key_arity != module.mechanism.key_arity
        {
            return false;
        }
        module.status = ModuleStatus::Persistent;
        module.support = Some(promotion.support());
        true
    }

    pub fn persistent_modules(&self) -> impl Iterator<Item = &RawRelationModule> {
        self.modules
            .values()
            .filter(|module| module.status == ModuleStatus::Persistent)
    }

    #[must_use]
    pub fn module(&self, id: Hash32) -> Option<&RawRelationModule> {
        self.modules.get(&id)
    }
}

/// Canonical evidence from which a complete relation mechanism must be freshly
/// re-invented. No learned object, status, receipt, or certificate is stored.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RawRelationCohortArtifact {
    mechanism: Hash32,
    traces: Vec<RawRelationObservationTrace>,
    replayed_groups: u64,
    bindings_without: u64,
    bindings_with: u64,
}

impl RawRelationCohortArtifact {
    #[must_use]
    pub const fn mechanism(&self) -> Hash32 {
        self.mechanism
    }
    #[must_use]
    pub fn traces(&self) -> &[RawRelationObservationTrace] {
        &self.traces
    }

    fn from_verified(
        learned: &LearnedRawRelationObservationGrouping,
        traces: &[RawRelationObservationTrace],
        receipt: &VerifiedRawRelationObservationReuse,
    ) -> Result<Self, RawRelationCohortArchiveError> {
        let mechanism = learned
            .mechanism_hash()
            .map_err(|_| RawRelationCohortArchiveError::Canonicalization)?;
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
            return Err(RawRelationCohortArchiveError::IdentityMismatch);
        }
        let mut traces = traces.to_vec();
        traces.sort_by_key(|trace| trace.episode);
        Ok(Self {
            mechanism,
            traces,
            replayed_groups: receipt.replayed_groups(),
            bindings_without: receipt.bindings_without(),
            bindings_with: receipt.bindings_with(),
        })
    }

    fn cohort_hash(&self) -> Result<Hash32, RawRelationCohortArchiveError> {
        self.encode_canonical()
            .map(|bytes| Hash32::digest(&bytes))
            .map_err(|_| RawRelationCohortArchiveError::Canonicalization)
    }

    fn from_value(value: CanonicalValue) -> Result<Self, RawRelationCohortArchiveError> {
        let CanonicalValue::Array(fields) = value else {
            return Err(RawRelationCohortArchiveError::InvalidPayload);
        };
        let [marker, mechanism, traces, groups, without, with] = fields.as_slice() else {
            return Err(RawRelationCohortArchiveError::InvalidPayload);
        };
        if marker != &CanonicalValue::Text("cairn.memory.raw-relation-cohort.v1".to_owned()) {
            return Err(RawRelationCohortArchiveError::InvalidPayload);
        }
        let CanonicalValue::Array(trace_values) = traces else {
            return Err(RawRelationCohortArchiveError::InvalidPayload);
        };
        let mut traces = trace_values
            .iter()
            .map(decode_raw_relation_trace)
            .collect::<Result<Vec<_>, _>>()?;
        traces.sort_by_key(|trace| trace.episode);
        let experiences = traces
            .iter()
            .map(|trace| trace.episode)
            .collect::<BTreeSet<_>>();
        if traces.len() < 2 || experiences.len() != traces.len() {
            return Err(RawRelationCohortArchiveError::InvalidPayload);
        }
        Ok(Self {
            mechanism: decode_hash(mechanism)?,
            traces,
            replayed_groups: decode_u64(groups)?,
            bindings_without: decode_u64(without)?,
            bindings_with: decode_u64(with)?,
        })
    }
}

impl CanonicalEncode for RawRelationCohortArtifact {
    fn encode_canonical(&self) -> Result<Vec<u8>, cairn_core::CanonicalError> {
        CanonicalValue::Array(vec![
            CanonicalValue::Text("cairn.memory.raw-relation-cohort.v1".to_owned()),
            CanonicalValue::Bytes(self.mechanism.as_bytes().to_vec()),
            CanonicalValue::Array(self.traces.iter().map(encode_raw_relation_trace).collect()),
            CanonicalValue::Unsigned(self.replayed_groups),
            CanonicalValue::Unsigned(self.bindings_without),
            CanonicalValue::Unsigned(self.bindings_with),
        ])
        .encode_canonical()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReplayedRawRelationCohort {
    pub learned: LearnedRawRelationObservationGrouping,
    pub receipt: VerifiedRawRelationObservationReuse,
}

/// Content-addressed evidence archive. Loading restores no authority; replay
/// performs fresh invention and independent verification.
#[derive(Clone, Copy, Debug, Default)]
pub struct RawRelationCohortArchive;

impl RawRelationCohortArchive {
    /// Persist one independently verified raw-relation cohort.
    ///
    /// # Errors
    ///
    /// Rejects missing lineage, mismatched evidence, or canonical storage
    /// failure.
    pub fn persist(
        &self,
        cas: &Cas,
        learned: &LearnedRawRelationObservationGrouping,
        traces: &[RawRelationObservationTrace],
        receipt: &VerifiedRawRelationObservationReuse,
        metadata: ReuseArtifactMetadata,
    ) -> Result<StoredArtifact, RawRelationCohortArchiveError> {
        if metadata.parent_ids.is_empty() {
            return Err(RawRelationCohortArchiveError::MissingParent);
        }
        let payload = RawRelationCohortArtifact::from_verified(learned, traces, receipt)?;
        let scope = payload.cohort_hash()?;
        let envelope = ArtifactEnvelope::new(
            raw_relation_schema()?,
            raw_relation_kind()?,
            raw_relation_semantics(),
            metadata.parent_ids,
            Some(scope),
            metadata.producer,
            metadata.budget,
            metadata.created_at_logical,
            payload,
        )?;
        cas.put_envelope(&envelope)
            .map_err(RawRelationCohortArchiveError::Storage)
    }

    /// Load structurally checked but untrusted evidence.
    ///
    /// # Errors
    ///
    /// Rejects schema, semantics, lineage, hash, payload, or bound failures.
    pub fn load(
        &self,
        cas: &Cas,
        blob_id: BlobId,
    ) -> Result<RawRelationCohortArtifact, RawRelationCohortArchiveError> {
        let mut registry = SchemaRegistry::new();
        registry.register(SchemaDescriptor {
            name: RAW_RELATION_COHORT_SCHEMA.to_owned(),
            current_version: RAW_RELATION_COHORT_SCHEMA_VERSION,
            additive_read_versions: BTreeSet::new(),
            maximum_payload_bytes: 16_777_216,
        })?;
        let envelope = cas.get_envelope(
            blob_id,
            &registry,
            &BTreeSet::from([raw_relation_semantics()]),
            CanonicalLimits {
                max_input_bytes: 20_971_520,
                max_depth: 32,
                max_collection_items: 524_288,
                max_scalar_bytes: 16_777_216,
            },
        )?;
        if envelope.artifact_kind != raw_relation_kind()? || envelope.parent_ids.is_empty() {
            return Err(RawRelationCohortArchiveError::InvalidEnvelope);
        }
        let artifact = RawRelationCohortArtifact::from_value(envelope.payload)?;
        if envelope.task_scope != Some(artifact.cohort_hash()?) {
            return Err(RawRelationCohortArchiveError::InvalidEnvelope);
        }
        Ok(artifact)
    }

    /// Re-invent and independently replay loaded evidence from blank authority.
    ///
    /// # Errors
    ///
    /// Rejects invention, identity, work accounting, or verifier mismatch.
    #[allow(clippy::too_many_arguments)]
    pub fn replay(
        &self,
        artifact: &RawRelationCohortArtifact,
        relation_limits: RelationObservationLimits,
        entity_limits: EntityObservationLimits,
        observation_limits: ObservationPredicateLimits,
        predicate_limits: PredicateGroupingLimits,
        invention_limits: InventionLimits,
        execution_limits: ExecLimits,
    ) -> Result<ReplayedRawRelationCohort, RawRelationCohortArchiveError> {
        let learned = RawRelationObservationInventor
            .invent(
                artifact.traces(),
                relation_limits,
                entity_limits,
                observation_limits,
                predicate_limits,
                invention_limits,
            )
            .map_err(RawRelationCohortArchiveError::Invention)?;
        if learned
            .mechanism_hash()
            .map_err(|_| RawRelationCohortArchiveError::Canonicalization)?
            != artifact.mechanism
        {
            return Err(RawRelationCohortArchiveError::IdentityMismatch);
        }
        let receipt = RawRelationObservationVerifier
            .verify_reuse(
                &learned,
                artifact.traces(),
                relation_limits,
                entity_limits,
                observation_limits,
                predicate_limits,
                execution_limits,
            )
            .map_err(RawRelationCohortArchiveError::Verification)?;
        if receipt.mechanism() != artifact.mechanism
            || receipt.replayed_groups() != artifact.replayed_groups
            || receipt.bindings_without() != artifact.bindings_without
            || receipt.bindings_with() != artifact.bindings_with
        {
            return Err(RawRelationCohortArchiveError::ReplayMismatch);
        }
        Ok(ReplayedRawRelationCohort { learned, receipt })
    }
}

#[derive(Debug)]
pub enum RawRelationCohortArchiveError {
    MissingParent,
    InvalidEnvelope,
    InvalidPayload,
    Canonicalization,
    IdentityMismatch,
    ReplayMismatch,
    Invention(RelationObservationError),
    Verification(RawRelationObservationPromotionError),
    Artifact(ArtifactError),
    Storage(CasError),
}

impl fmt::Display for RawRelationCohortArchiveError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for RawRelationCohortArchiveError {}

impl From<ArtifactError> for RawRelationCohortArchiveError {
    fn from(value: ArtifactError) -> Self {
        Self::Artifact(value)
    }
}

impl From<CasError> for RawRelationCohortArchiveError {
    fn from(value: CasError) -> Self {
        Self::Storage(value)
    }
}

fn encode_raw_relation_trace(trace: &RawRelationObservationTrace) -> CanonicalValue {
    CanonicalValue::Array(vec![
        CanonicalValue::Bytes(trace.episode.as_bytes().to_vec()),
        CanonicalValue::Array(
            trace
                .primitives
                .iter()
                .map(|(id, primitive)| {
                    CanonicalValue::Array(vec![
                        CanonicalValue::Bytes(id.0.as_bytes().to_vec()),
                        signed(primitive.observation),
                        CanonicalValue::Array(
                            primitive.attributes.iter().copied().map(signed).collect(),
                        ),
                    ])
                })
                .collect(),
        ),
        CanonicalValue::Array(trace.outputs.iter().copied().map(signed).collect()),
    ])
}

fn decode_raw_relation_trace(
    value: &CanonicalValue,
) -> Result<RawRelationObservationTrace, RawRelationCohortArchiveError> {
    let CanonicalValue::Array(fields) = value else {
        return Err(RawRelationCohortArchiveError::InvalidPayload);
    };
    let [episode, primitives, outputs] = fields.as_slice() else {
        return Err(RawRelationCohortArchiveError::InvalidPayload);
    };
    let CanonicalValue::Array(primitive_values) = primitives else {
        return Err(RawRelationCohortArchiveError::InvalidPayload);
    };
    let primitives = primitive_values
        .iter()
        .map(|value| {
            let CanonicalValue::Array(fields) = value else {
                return Err(RawRelationCohortArchiveError::InvalidPayload);
            };
            let [id, observation, attributes] = fields.as_slice() else {
                return Err(RawRelationCohortArchiveError::InvalidPayload);
            };
            let CanonicalValue::Array(attribute_values) = attributes else {
                return Err(RawRelationCohortArchiveError::InvalidPayload);
            };
            if attribute_values.is_empty() {
                return Err(RawRelationCohortArchiveError::InvalidPayload);
            }
            Ok((
                EntityId(decode_hash(id)?),
                RawAttributedObservationPrimitive {
                    observation: decode_i64(observation)?,
                    attributes: attribute_values
                        .iter()
                        .map(decode_i64)
                        .collect::<Result<Vec<_>, _>>()?,
                },
            ))
        })
        .collect::<Result<BTreeMap<_, _>, _>>()?;
    if primitives.len() != primitive_values.len() {
        return Err(RawRelationCohortArchiveError::InvalidPayload);
    }
    let CanonicalValue::Array(output_values) = outputs else {
        return Err(RawRelationCohortArchiveError::InvalidPayload);
    };
    let outputs = output_values
        .iter()
        .map(decode_i64)
        .collect::<Result<BTreeSet<_>, _>>()?;
    if outputs.len() != output_values.len() {
        return Err(RawRelationCohortArchiveError::InvalidPayload);
    }
    Ok(RawRelationObservationTrace {
        episode: decode_hash(episode)?,
        primitives,
        outputs,
    })
}

const fn signed(value: i64) -> CanonicalValue {
    if value >= 0 {
        CanonicalValue::Unsigned(value.unsigned_abs())
    } else {
        CanonicalValue::Negative(value)
    }
}

fn decode_hash(value: &CanonicalValue) -> Result<Hash32, RawRelationCohortArchiveError> {
    let CanonicalValue::Bytes(bytes) = value else {
        return Err(RawRelationCohortArchiveError::InvalidPayload);
    };
    let fixed: [u8; 32] = bytes
        .as_slice()
        .try_into()
        .map_err(|_| RawRelationCohortArchiveError::InvalidPayload)?;
    Ok(Hash32::from_bytes(fixed))
}

fn decode_i64(value: &CanonicalValue) -> Result<i64, RawRelationCohortArchiveError> {
    match value {
        CanonicalValue::Unsigned(value) => {
            i64::try_from(*value).map_err(|_| RawRelationCohortArchiveError::InvalidPayload)
        }
        CanonicalValue::Negative(value) if *value < 0 => Ok(*value),
        _ => Err(RawRelationCohortArchiveError::InvalidPayload),
    }
}

fn decode_u64(value: &CanonicalValue) -> Result<u64, RawRelationCohortArchiveError> {
    let CanonicalValue::Unsigned(value) = value else {
        return Err(RawRelationCohortArchiveError::InvalidPayload);
    };
    Ok(*value)
}

fn raw_relation_schema() -> Result<SchemaRef, ArtifactError> {
    SchemaRef::new(
        RAW_RELATION_COHORT_SCHEMA,
        RAW_RELATION_COHORT_SCHEMA_VERSION,
    )
}

fn raw_relation_kind() -> Result<ArtifactKind, ArtifactError> {
    ArtifactKind::new("memory.raw-relation-cohort")
}

const fn raw_relation_semantics() -> SemanticsVersion {
    SemanticsVersion::new(2, 0, 0)
}
