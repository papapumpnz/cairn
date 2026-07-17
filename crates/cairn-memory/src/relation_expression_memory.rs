//! Evidence-only persistence for learned multi-operator relation expressions.
//!
//! The archive stores canonical observations and verifier-derived accounting,
//! never an executable expression, receipt, promotion certificate, or module
//! status. A fresh process must re-invent and independently replay each cohort
//! before a separate consolidator can restore proposal-only persistence.

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
    EntityObservationLimits, LearnedRelationExpressionGrouping, ObservationPredicateLimits,
    PredicateGroupingLimits, RawAttributedObservationPrimitive, RawRelationObservationTrace,
    RelationExpressionError, RelationExpressionInventor, RelationExpressionLimits,
    RelationExpressionMechanism,
};
use cairn_verifier::{
    RelationExpressionVerifier, RelationExpressionVerifyError, VerifiedRelationExpressionPromotion,
    VerifiedRelationExpressionReuse,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

pub const RELATION_EXPRESSION_COHORT_SCHEMA: &str = "cairn.memory.relation-expression-cohort";
pub const RELATION_EXPRESSION_COHORT_SCHEMA_VERSION: u32 = 1;

/// One proof-gated relation-expression proposal and its local evidence ledger.
#[derive(Clone, Debug, PartialEq)]
pub struct RelationExpressionModule {
    id: Hash32,
    mechanism: RelationExpressionMechanism,
    receipts: BTreeMap<Hash32, VerifiedRelationExpressionReuse>,
    experiences: BTreeSet<Hash32>,
    status: ModuleStatus,
}

impl RelationExpressionModule {
    #[must_use]
    pub const fn id(&self) -> Hash32 {
        self.id
    }

    #[must_use]
    pub const fn mechanism(&self) -> &RelationExpressionMechanism {
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
pub enum RelationExpressionStoreError {
    Canonicalization,
    IdentityMismatch,
    UnknownModule,
    EmptyReplay,
    DuplicateReceipt,
    OverlappingExperience,
}

/// Proposal-only memory for complete learned relation-expression mechanisms.
#[derive(Clone, Debug, Default)]
pub struct RelationExpressionStore {
    modules: BTreeMap<Hash32, RelationExpressionModule>,
}

impl RelationExpressionStore {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            modules: BTreeMap::new(),
        }
    }

    /// Insert a source-independent expression provisionally.
    ///
    /// # Errors
    ///
    /// Rejects malformed expression/definition shape or canonical identity
    /// failure.
    pub fn insert_provisional(
        &mut self,
        mechanism: RelationExpressionMechanism,
    ) -> Result<Hash32, RelationExpressionStoreError> {
        let arity = usize::try_from(mechanism.expression.parameter_count())
            .map_err(|_| RelationExpressionStoreError::IdentityMismatch)?;
        if arity < 3
            || mechanism.expression.binary_node_count() == 0
            || mechanism.relation_definition.parameters.len() != arity.saturating_mul(2)
        {
            return Err(RelationExpressionStoreError::IdentityMismatch);
        }
        let id = mechanism
            .semantic_hash()
            .map_err(|_| RelationExpressionStoreError::Canonicalization)?;
        if let Some(existing) = self.modules.get(&id) {
            if existing.mechanism.semantic_hash().ok() != Some(id) {
                return Err(RelationExpressionStoreError::IdentityMismatch);
            }
            return Ok(id);
        }
        self.modules.insert(
            id,
            RelationExpressionModule {
                id,
                mechanism,
                receipts: BTreeMap::new(),
                experiences: BTreeSet::new(),
                status: ModuleStatus::Provisional,
            },
        );
        Ok(id)
    }

    /// Record one independently verified disjoint reuse cohort.
    ///
    /// # Errors
    ///
    /// Rejects unknown/mismatched identity, empty evidence, duplicate receipt,
    /// or overlapping experiences.
    pub fn record_verified_reuse(
        &mut self,
        id: Hash32,
        receipt: VerifiedRelationExpressionReuse,
    ) -> Result<(), RelationExpressionStoreError> {
        let module = self
            .modules
            .get_mut(&id)
            .ok_or(RelationExpressionStoreError::UnknownModule)?;
        if receipt.mechanism() != id {
            return Err(RelationExpressionStoreError::IdentityMismatch);
        }
        if receipt.replayed_groups() == 0 || receipt.experiences().is_empty() {
            return Err(RelationExpressionStoreError::EmptyReplay);
        }
        let receipt_id = Hash32::digest(
            &receipt
                .experiences()
                .iter()
                .flat_map(|experience| experience.as_bytes().iter().copied())
                .collect::<Vec<_>>(),
        );
        if module.receipts.contains_key(&receipt_id) {
            return Err(RelationExpressionStoreError::DuplicateReceipt);
        }
        if !module.experiences.is_disjoint(receipt.experiences()) {
            return Err(RelationExpressionStoreError::OverlappingExperience);
        }
        module.experiences.extend(receipt.experiences());
        module.receipts.insert(receipt_id, receipt);
        Ok(())
    }

    /// Apply a consolidator certificate only when it covers the complete local
    /// evidence ledger.
    #[must_use]
    pub fn apply_verified_promotion(
        &mut self,
        id: Hash32,
        promotion: &VerifiedRelationExpressionPromotion,
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

    pub fn persistent_modules(&self) -> impl Iterator<Item = &RelationExpressionModule> {
        self.modules
            .values()
            .filter(|module| module.status == ModuleStatus::Persistent)
    }

    #[must_use]
    pub fn module(&self, id: Hash32) -> Option<&RelationExpressionModule> {
        self.modules.get(&id)
    }
}

/// Canonical evidence from which an expression must be freshly re-invented.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RelationExpressionCohortArtifact {
    mechanism: Hash32,
    traces: Vec<RawRelationObservationTrace>,
    replayed_groups: u64,
    expressions_without: u64,
    expressions_with: u64,
}

impl RelationExpressionCohortArtifact {
    #[must_use]
    pub const fn mechanism(&self) -> Hash32 {
        self.mechanism
    }

    #[must_use]
    pub fn traces(&self) -> &[RawRelationObservationTrace] {
        &self.traces
    }

    fn from_verified(
        learned: &LearnedRelationExpressionGrouping,
        traces: &[RawRelationObservationTrace],
        receipt: &VerifiedRelationExpressionReuse,
    ) -> Result<Self, RelationExpressionCohortArchiveError> {
        let mechanism = learned
            .mechanism_hash()
            .map_err(|_| RelationExpressionCohortArchiveError::Canonicalization)?;
        let experiences = traces
            .iter()
            .map(|trace| trace.episode)
            .collect::<BTreeSet<_>>();
        if traces.len() < 2
            || experiences.len() != traces.len()
            || mechanism != receipt.mechanism()
            || &experiences != receipt.experiences()
            || receipt.replayed_groups() == 0
            || receipt.expressions_without() <= receipt.expressions_with()
        {
            return Err(RelationExpressionCohortArchiveError::IdentityMismatch);
        }
        let mut traces = traces.to_vec();
        traces.sort_by_key(|trace| trace.episode);
        Ok(Self {
            mechanism,
            traces,
            replayed_groups: receipt.replayed_groups(),
            expressions_without: receipt.expressions_without(),
            expressions_with: receipt.expressions_with(),
        })
    }

    fn cohort_hash(&self) -> Result<Hash32, RelationExpressionCohortArchiveError> {
        self.encode_canonical()
            .map(|bytes| Hash32::digest(&bytes))
            .map_err(|_| RelationExpressionCohortArchiveError::Canonicalization)
    }

    fn from_value(value: CanonicalValue) -> Result<Self, RelationExpressionCohortArchiveError> {
        let CanonicalValue::Array(fields) = value else {
            return Err(RelationExpressionCohortArchiveError::InvalidPayload);
        };
        let [marker, mechanism, traces, groups, without, with] = fields.as_slice() else {
            return Err(RelationExpressionCohortArchiveError::InvalidPayload);
        };
        if marker != &CanonicalValue::Text("cairn.memory.relation-expression-cohort.v1".to_owned())
        {
            return Err(RelationExpressionCohortArchiveError::InvalidPayload);
        }
        let CanonicalValue::Array(trace_values) = traces else {
            return Err(RelationExpressionCohortArchiveError::InvalidPayload);
        };
        let mut traces = trace_values
            .iter()
            .map(decode_trace)
            .collect::<Result<Vec<_>, _>>()?;
        traces.sort_by_key(|trace| trace.episode);
        let experiences = traces
            .iter()
            .map(|trace| trace.episode)
            .collect::<BTreeSet<_>>();
        if traces.len() < 2 || experiences.len() != traces.len() {
            return Err(RelationExpressionCohortArchiveError::InvalidPayload);
        }
        Ok(Self {
            mechanism: decode_hash(mechanism)?,
            traces,
            replayed_groups: decode_u64(groups)?,
            expressions_without: decode_u64(without)?,
            expressions_with: decode_u64(with)?,
        })
    }
}

impl CanonicalEncode for RelationExpressionCohortArtifact {
    fn encode_canonical(&self) -> Result<Vec<u8>, cairn_core::CanonicalError> {
        CanonicalValue::Array(vec![
            CanonicalValue::Text("cairn.memory.relation-expression-cohort.v1".to_owned()),
            CanonicalValue::Bytes(self.mechanism.as_bytes().to_vec()),
            CanonicalValue::Array(self.traces.iter().map(encode_trace).collect()),
            CanonicalValue::Unsigned(self.replayed_groups),
            CanonicalValue::Unsigned(self.expressions_without),
            CanonicalValue::Unsigned(self.expressions_with),
        ])
        .encode_canonical()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReplayedRelationExpressionCohort {
    pub learned: LearnedRelationExpressionGrouping,
    pub receipt: VerifiedRelationExpressionReuse,
}

/// Content-addressed evidence archive with no authority-restoration path.
#[derive(Clone, Copy, Debug, Default)]
pub struct RelationExpressionCohortArchive;

impl RelationExpressionCohortArchive {
    /// Persist one independently verified expression cohort.
    ///
    /// # Errors
    ///
    /// Rejects missing lineage, mismatched evidence, or canonical storage
    /// failure.
    pub fn persist(
        &self,
        cas: &Cas,
        learned: &LearnedRelationExpressionGrouping,
        traces: &[RawRelationObservationTrace],
        receipt: &VerifiedRelationExpressionReuse,
        metadata: ReuseArtifactMetadata,
    ) -> Result<StoredArtifact, RelationExpressionCohortArchiveError> {
        if metadata.parent_ids.is_empty() {
            return Err(RelationExpressionCohortArchiveError::MissingParent);
        }
        let payload = RelationExpressionCohortArtifact::from_verified(learned, traces, receipt)?;
        let scope = payload.cohort_hash()?;
        let envelope = ArtifactEnvelope::new(
            expression_schema()?,
            expression_kind()?,
            expression_semantics(),
            metadata.parent_ids,
            Some(scope),
            metadata.producer,
            metadata.budget,
            metadata.created_at_logical,
            payload,
        )?;
        cas.put_envelope(&envelope)
            .map_err(RelationExpressionCohortArchiveError::Storage)
    }

    /// Load structurally checked but untrusted expression evidence.
    ///
    /// # Errors
    ///
    /// Rejects schema, semantics, lineage, hash, payload, or bound failures.
    pub fn load(
        &self,
        cas: &Cas,
        blob_id: BlobId,
    ) -> Result<RelationExpressionCohortArtifact, RelationExpressionCohortArchiveError> {
        let mut registry = SchemaRegistry::new();
        registry.register(SchemaDescriptor {
            name: RELATION_EXPRESSION_COHORT_SCHEMA.to_owned(),
            current_version: RELATION_EXPRESSION_COHORT_SCHEMA_VERSION,
            additive_read_versions: BTreeSet::new(),
            maximum_payload_bytes: 16_777_216,
        })?;
        let envelope = cas.get_envelope(
            blob_id,
            &registry,
            &BTreeSet::from([expression_semantics()]),
            CanonicalLimits {
                max_input_bytes: 20_971_520,
                max_depth: 32,
                max_collection_items: 524_288,
                max_scalar_bytes: 16_777_216,
            },
        )?;
        if envelope.artifact_kind != expression_kind()? || envelope.parent_ids.is_empty() {
            return Err(RelationExpressionCohortArchiveError::InvalidEnvelope);
        }
        let artifact = RelationExpressionCohortArtifact::from_value(envelope.payload)?;
        if envelope.task_scope != Some(artifact.cohort_hash()?) {
            return Err(RelationExpressionCohortArchiveError::InvalidEnvelope);
        }
        Ok(artifact)
    }

    /// Re-invent and independently replay loaded evidence from blank authority.
    ///
    /// # Errors
    ///
    /// Rejects invention, identity, accounting, or verifier mismatch.
    #[allow(clippy::too_many_arguments)]
    pub fn replay(
        &self,
        artifact: &RelationExpressionCohortArtifact,
        expression_limits: &RelationExpressionLimits,
        entity_limits: EntityObservationLimits,
        observation_limits: ObservationPredicateLimits,
        predicate_limits: PredicateGroupingLimits,
        invention_limits: InventionLimits,
        execution_limits: ExecLimits,
    ) -> Result<ReplayedRelationExpressionCohort, RelationExpressionCohortArchiveError> {
        let learned = RelationExpressionInventor
            .invent(
                artifact.traces(),
                expression_limits,
                entity_limits,
                observation_limits,
                predicate_limits,
                invention_limits,
            )
            .map_err(RelationExpressionCohortArchiveError::Invention)?;
        if learned
            .mechanism_hash()
            .map_err(|_| RelationExpressionCohortArchiveError::Canonicalization)?
            != artifact.mechanism
        {
            return Err(RelationExpressionCohortArchiveError::IdentityMismatch);
        }
        let receipt = RelationExpressionVerifier
            .verify_reuse(
                &learned,
                artifact.traces(),
                expression_limits,
                entity_limits,
                observation_limits,
                predicate_limits,
                execution_limits,
            )
            .map_err(RelationExpressionCohortArchiveError::Verification)?;
        if receipt.mechanism() != artifact.mechanism
            || receipt.replayed_groups() != artifact.replayed_groups
            || receipt.expressions_without() != artifact.expressions_without
            || receipt.expressions_with() != artifact.expressions_with
        {
            return Err(RelationExpressionCohortArchiveError::ReplayMismatch);
        }
        Ok(ReplayedRelationExpressionCohort { learned, receipt })
    }
}

#[derive(Debug)]
pub enum RelationExpressionCohortArchiveError {
    MissingParent,
    InvalidEnvelope,
    InvalidPayload,
    Canonicalization,
    IdentityMismatch,
    ReplayMismatch,
    Invention(RelationExpressionError),
    Verification(RelationExpressionVerifyError),
    Artifact(ArtifactError),
    Storage(CasError),
}

impl fmt::Display for RelationExpressionCohortArchiveError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for RelationExpressionCohortArchiveError {}

impl From<ArtifactError> for RelationExpressionCohortArchiveError {
    fn from(value: ArtifactError) -> Self {
        Self::Artifact(value)
    }
}

impl From<CasError> for RelationExpressionCohortArchiveError {
    fn from(value: CasError) -> Self {
        Self::Storage(value)
    }
}

fn encode_trace(trace: &RawRelationObservationTrace) -> CanonicalValue {
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

fn decode_trace(
    value: &CanonicalValue,
) -> Result<RawRelationObservationTrace, RelationExpressionCohortArchiveError> {
    let CanonicalValue::Array(fields) = value else {
        return Err(RelationExpressionCohortArchiveError::InvalidPayload);
    };
    let [episode, primitives, outputs] = fields.as_slice() else {
        return Err(RelationExpressionCohortArchiveError::InvalidPayload);
    };
    let CanonicalValue::Array(primitive_values) = primitives else {
        return Err(RelationExpressionCohortArchiveError::InvalidPayload);
    };
    let primitives = primitive_values
        .iter()
        .map(|value| {
            let CanonicalValue::Array(fields) = value else {
                return Err(RelationExpressionCohortArchiveError::InvalidPayload);
            };
            let [id, observation, attributes] = fields.as_slice() else {
                return Err(RelationExpressionCohortArchiveError::InvalidPayload);
            };
            let CanonicalValue::Array(attribute_values) = attributes else {
                return Err(RelationExpressionCohortArchiveError::InvalidPayload);
            };
            if attribute_values.is_empty() {
                return Err(RelationExpressionCohortArchiveError::InvalidPayload);
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
        return Err(RelationExpressionCohortArchiveError::InvalidPayload);
    }
    let CanonicalValue::Array(output_values) = outputs else {
        return Err(RelationExpressionCohortArchiveError::InvalidPayload);
    };
    let outputs = output_values
        .iter()
        .map(decode_i64)
        .collect::<Result<BTreeSet<_>, _>>()?;
    if outputs.len() != output_values.len() {
        return Err(RelationExpressionCohortArchiveError::InvalidPayload);
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

fn decode_hash(value: &CanonicalValue) -> Result<Hash32, RelationExpressionCohortArchiveError> {
    let CanonicalValue::Bytes(bytes) = value else {
        return Err(RelationExpressionCohortArchiveError::InvalidPayload);
    };
    let fixed: [u8; 32] = bytes
        .as_slice()
        .try_into()
        .map_err(|_| RelationExpressionCohortArchiveError::InvalidPayload)?;
    Ok(Hash32::from_bytes(fixed))
}

fn decode_i64(value: &CanonicalValue) -> Result<i64, RelationExpressionCohortArchiveError> {
    match value {
        CanonicalValue::Unsigned(value) => {
            i64::try_from(*value).map_err(|_| RelationExpressionCohortArchiveError::InvalidPayload)
        }
        CanonicalValue::Negative(value) if *value < 0 => Ok(*value),
        _ => Err(RelationExpressionCohortArchiveError::InvalidPayload),
    }
}

fn decode_u64(value: &CanonicalValue) -> Result<u64, RelationExpressionCohortArchiveError> {
    let CanonicalValue::Unsigned(value) = value else {
        return Err(RelationExpressionCohortArchiveError::InvalidPayload);
    };
    Ok(*value)
}

fn expression_schema() -> Result<SchemaRef, ArtifactError> {
    SchemaRef::new(
        RELATION_EXPRESSION_COHORT_SCHEMA,
        RELATION_EXPRESSION_COHORT_SCHEMA_VERSION,
    )
}

fn expression_kind() -> Result<ArtifactKind, ArtifactError> {
    ArtifactKind::new("memory.relation-expression-cohort")
}

const fn expression_semantics() -> SemanticsVersion {
    SemanticsVersion::new(1, 0, 0)
}
