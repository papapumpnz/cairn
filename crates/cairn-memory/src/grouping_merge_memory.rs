use crate::{ModuleStatus, ReuseArtifactMetadata};
use cairn_cas::{Cas, CasError, StoredArtifact};
use cairn_core::{
    ArtifactEnvelope, ArtifactError, ArtifactKind, BlobId, CanonicalEncode, CanonicalLimits,
    CanonicalValue, EntityId, Hash32, RelationRecord, RelationRole, SchemaDescriptor, SchemaRef,
    SchemaRegistry, SemanticsVersion,
};
use cairn_invent::InventionLimits;
use cairn_rkl::ExecLimits;
use cairn_theory::{
    LearnedGroupingMerge, RawGroupingError, RawGroupingLimits, RawGroupingTrace,
    RawRelationGroupingInventor,
};
use cairn_verifier::{
    GroupingMergeVerifier, GroupingPromotionError, VerifiedGroupingPromotion, VerifiedGroupingReuse,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

pub const GROUPING_COHORT_SCHEMA: &str = "cairn.memory.raw-grouping-cohort";
pub const GROUPING_COHORT_SCHEMA_VERSION: u32 = 1;

/// One proposal-only relation-bound mechanism and its verifier evidence.
#[derive(Clone, Debug, PartialEq)]
pub struct GroupingMergeModule {
    id: Hash32,
    mechanism: LearnedGroupingMerge,
    receipts: BTreeMap<Hash32, VerifiedGroupingReuse>,
    experiences: BTreeSet<Hash32>,
    status: ModuleStatus,
}

impl GroupingMergeModule {
    #[must_use]
    pub const fn id(&self) -> Hash32 {
        self.id
    }
    #[must_use]
    pub const fn mechanism(&self) -> &LearnedGroupingMerge {
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
pub enum GroupingMergeStoreError {
    Canonicalization,
    IdentityMismatch,
    UnknownModule,
    EmptyReplay,
    DuplicateReceipt,
    OverlappingExperience,
}

/// Proof-gated memory for a selected grouping plus executable merge rule.
#[derive(Clone, Debug, Default)]
pub struct GroupingMergeStore {
    modules: BTreeMap<Hash32, GroupingMergeModule>,
}

impl GroupingMergeStore {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            modules: BTreeMap::new(),
        }
    }

    /// Insert one task-local learned composition as a provisional proposal.
    ///
    /// # Errors
    ///
    /// Rejects canonicalization failure or an identity collision.
    pub fn insert_provisional(
        &mut self,
        mechanism: LearnedGroupingMerge,
    ) -> Result<Hash32, GroupingMergeStoreError> {
        let id = mechanism
            .semantic_hash()
            .map_err(|_| GroupingMergeStoreError::Canonicalization)?;
        if let Some(existing) = self.modules.get(&id) {
            if existing.mechanism.semantic_hash().ok() != Some(id) {
                return Err(GroupingMergeStoreError::IdentityMismatch);
            }
            return Ok(id);
        }
        self.modules.insert(
            id,
            GroupingMergeModule {
                id,
                mechanism,
                receipts: BTreeMap::new(),
                experiences: BTreeSet::new(),
                status: ModuleStatus::Provisional,
            },
        );
        Ok(id)
    }

    /// Record an opaque independent-verifier reuse receipt.
    ///
    /// # Errors
    ///
    /// Rejects unknown identity, empty replay, duplicate receipt, or reused
    /// experience evidence.
    pub fn record_verified_reuse(
        &mut self,
        id: Hash32,
        receipt: VerifiedGroupingReuse,
    ) -> Result<(), GroupingMergeStoreError> {
        let module = self
            .modules
            .get_mut(&id)
            .ok_or(GroupingMergeStoreError::UnknownModule)?;
        if receipt.mechanism() != id {
            return Err(GroupingMergeStoreError::IdentityMismatch);
        }
        if receipt.replayed_groups() == 0 || receipt.experiences().is_empty() {
            return Err(GroupingMergeStoreError::EmptyReplay);
        }
        let receipt_id = Hash32::digest(
            &receipt
                .experiences()
                .iter()
                .flat_map(|experience| experience.as_bytes().iter().copied())
                .collect::<Vec<_>>(),
        );
        if module.receipts.contains_key(&receipt_id) {
            return Err(GroupingMergeStoreError::DuplicateReceipt);
        }
        if !module.experiences.is_disjoint(receipt.experiences()) {
            return Err(GroupingMergeStoreError::OverlappingExperience);
        }
        module.experiences.extend(receipt.experiences());
        module.receipts.insert(receipt_id, receipt);
        Ok(())
    }

    /// Apply only a consolidator-issued certificate covering stored evidence.
    #[must_use]
    pub fn apply_verified_promotion(
        &mut self,
        id: Hash32,
        promotion: &VerifiedGroupingPromotion,
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

    pub fn persistent_mechanisms(&self) -> impl Iterator<Item = &LearnedGroupingMerge> {
        self.modules
            .values()
            .filter(|module| module.status == ModuleStatus::Persistent)
            .map(|module| &module.mechanism)
    }

    #[must_use]
    pub fn module(&self, id: Hash32) -> Option<&GroupingMergeModule> {
        self.modules.get(&id)
    }
}

/// Canonical but untrusted raw cohort sufficient to reconstruct learning after
/// restart. No verifier receipt, promotion certificate, or module status is
/// serialized.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RawGroupingCohortArtifact {
    mechanism: Hash32,
    traces: Vec<RawGroupingTrace>,
    replayed_groups: u64,
    candidate_executions_without: u64,
    candidate_executions_with: u64,
}

impl RawGroupingCohortArtifact {
    #[must_use]
    pub const fn mechanism(&self) -> Hash32 {
        self.mechanism
    }

    #[must_use]
    pub fn traces(&self) -> &[RawGroupingTrace] {
        &self.traces
    }

    fn from_verified(
        learned: &LearnedGroupingMerge,
        traces: &[RawGroupingTrace],
        receipt: &VerifiedGroupingReuse,
    ) -> Result<Self, GroupingCohortArchiveError> {
        let mechanism = learned
            .semantic_hash()
            .map_err(|_| GroupingCohortArchiveError::Canonicalization)?;
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
            return Err(GroupingCohortArchiveError::IdentityMismatch);
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

    fn cohort_hash(&self) -> Result<Hash32, GroupingCohortArchiveError> {
        self.encode_canonical()
            .map(|bytes| Hash32::digest(&bytes))
            .map_err(|_| GroupingCohortArchiveError::Canonicalization)
    }

    fn from_value(value: CanonicalValue) -> Result<Self, GroupingCohortArchiveError> {
        let CanonicalValue::Array(fields) = value else {
            return Err(GroupingCohortArchiveError::InvalidPayload);
        };
        let [marker, mechanism, traces, replayed_groups, candidates_without, candidates_with] =
            fields.as_slice()
        else {
            return Err(GroupingCohortArchiveError::InvalidPayload);
        };
        if marker != &CanonicalValue::Text("cairn.memory.raw-grouping-cohort.v1".to_owned()) {
            return Err(GroupingCohortArchiveError::InvalidPayload);
        }
        let CanonicalValue::Array(trace_values) = traces else {
            return Err(GroupingCohortArchiveError::InvalidPayload);
        };
        let mut decoded = trace_values
            .iter()
            .map(decode_trace)
            .collect::<Result<Vec<_>, _>>()?;
        decoded.sort_by_key(|trace| trace.episode);
        let experiences = decoded
            .iter()
            .map(|trace| trace.episode)
            .collect::<BTreeSet<_>>();
        if decoded.len() < 2 || experiences.len() != decoded.len() {
            return Err(GroupingCohortArchiveError::InvalidPayload);
        }
        Ok(Self {
            mechanism: decode_hash(mechanism)?,
            traces: decoded,
            replayed_groups: decode_u64(replayed_groups)?,
            candidate_executions_without: decode_u64(candidates_without)?,
            candidate_executions_with: decode_u64(candidates_with)?,
        })
    }
}

impl CanonicalEncode for RawGroupingCohortArtifact {
    fn encode_canonical(&self) -> Result<Vec<u8>, cairn_core::CanonicalError> {
        CanonicalValue::Array(vec![
            CanonicalValue::Text("cairn.memory.raw-grouping-cohort.v1".to_owned()),
            CanonicalValue::Bytes(self.mechanism.as_bytes().to_vec()),
            CanonicalValue::Array(self.traces.iter().map(encode_trace).collect()),
            CanonicalValue::Unsigned(self.replayed_groups),
            CanonicalValue::Unsigned(self.candidate_executions_without),
            CanonicalValue::Unsigned(self.candidate_executions_with),
        ])
        .encode_canonical()
    }
}

/// Authority reconstructed from one loaded raw cohort.
#[derive(Clone, Debug, PartialEq)]
pub struct ReplayedGroupingCohort {
    pub learned: LearnedGroupingMerge,
    pub receipt: VerifiedGroupingReuse,
}

/// File-backed archive for raw grouping cohorts and their measured identity.
#[derive(Clone, Copy, Debug, Default)]
pub struct RawGroupingCohortArchive;

impl RawGroupingCohortArchive {
    /// Persist one independently learned and replayed raw cohort.
    ///
    /// # Errors
    ///
    /// Rejects missing lineage, identity disagreement, malformed evidence, or
    /// canonical CAS failures.
    pub fn persist(
        &self,
        cas: &Cas,
        learned: &LearnedGroupingMerge,
        traces: &[RawGroupingTrace],
        receipt: &VerifiedGroupingReuse,
        metadata: ReuseArtifactMetadata,
    ) -> Result<StoredArtifact, GroupingCohortArchiveError> {
        if metadata.parent_ids.is_empty() {
            return Err(GroupingCohortArchiveError::MissingParent);
        }
        let payload = RawGroupingCohortArtifact::from_verified(learned, traces, receipt)?;
        let scope = payload.cohort_hash()?;
        let envelope = ArtifactEnvelope::new(
            grouping_cohort_schema()?,
            grouping_cohort_kind()?,
            grouping_cohort_semantics(),
            metadata.parent_ids,
            Some(scope),
            metadata.producer,
            metadata.budget,
            metadata.created_at_logical,
            payload,
        )?;
        cas.put_envelope(&envelope)
            .map_err(GroupingCohortArchiveError::Storage)
    }

    /// Load a structurally validated but still untrusted raw cohort.
    ///
    /// # Errors
    ///
    /// Rejects storage corruption, schema/semantics mismatch, absent lineage,
    /// malformed payload, bounds, or scope mismatch.
    pub fn load(
        &self,
        cas: &Cas,
        blob_id: BlobId,
    ) -> Result<RawGroupingCohortArtifact, GroupingCohortArchiveError> {
        let mut registry = SchemaRegistry::new();
        registry.register(SchemaDescriptor {
            name: GROUPING_COHORT_SCHEMA.to_owned(),
            current_version: GROUPING_COHORT_SCHEMA_VERSION,
            additive_read_versions: BTreeSet::new(),
            maximum_payload_bytes: 8_388_608,
        })?;
        let envelope = cas.get_envelope(
            blob_id,
            &registry,
            &BTreeSet::from([grouping_cohort_semantics()]),
            CanonicalLimits {
                max_input_bytes: 10_485_760,
                max_depth: 32,
                max_collection_items: 262_144,
                max_scalar_bytes: 8_388_608,
            },
        )?;
        if envelope.artifact_kind != grouping_cohort_kind()? || envelope.parent_ids.is_empty() {
            return Err(GroupingCohortArchiveError::InvalidEnvelope);
        }
        let artifact = RawGroupingCohortArtifact::from_value(envelope.payload)?;
        if envelope.task_scope != Some(artifact.cohort_hash()?) {
            return Err(GroupingCohortArchiveError::InvalidEnvelope);
        }
        Ok(artifact)
    }

    /// Re-invent and independently replay one loaded cohort from scratch.
    ///
    /// # Errors
    ///
    /// Rejects changed identity, failed invention, failed independent replay,
    /// or any changed work/replay measurement.
    pub fn replay(
        &self,
        artifact: &RawGroupingCohortArtifact,
        raw_limits: RawGroupingLimits,
        invention_limits: InventionLimits,
        execution_limits: ExecLimits,
    ) -> Result<ReplayedGroupingCohort, GroupingCohortArchiveError> {
        let learned = RawRelationGroupingInventor
            .invent(artifact.traces(), raw_limits, invention_limits)
            .map_err(GroupingCohortArchiveError::Invention)?;
        if learned
            .semantic_hash()
            .map_err(|_| GroupingCohortArchiveError::Canonicalization)?
            != artifact.mechanism
        {
            return Err(GroupingCohortArchiveError::IdentityMismatch);
        }
        let receipt = GroupingMergeVerifier
            .verify_raw_reuse(
                &learned,
                artifact.traces(),
                artifact.candidate_executions_without,
                artifact.candidate_executions_with,
                raw_limits,
                execution_limits,
            )
            .map_err(GroupingCohortArchiveError::Verification)?;
        if receipt.mechanism() != artifact.mechanism
            || receipt.replayed_groups() != artifact.replayed_groups
            || receipt.candidate_executions_without() != artifact.candidate_executions_without
            || receipt.candidate_executions_with() != artifact.candidate_executions_with
        {
            return Err(GroupingCohortArchiveError::ReplayMismatch);
        }
        Ok(ReplayedGroupingCohort { learned, receipt })
    }
}

#[derive(Debug)]
pub enum GroupingCohortArchiveError {
    MissingParent,
    InvalidEnvelope,
    InvalidPayload,
    Canonicalization,
    IdentityMismatch,
    ReplayMismatch,
    Invention(RawGroupingError),
    Verification(GroupingPromotionError),
    Artifact(ArtifactError),
    Storage(CasError),
}

impl fmt::Display for GroupingCohortArchiveError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for GroupingCohortArchiveError {}

impl From<ArtifactError> for GroupingCohortArchiveError {
    fn from(value: ArtifactError) -> Self {
        Self::Artifact(value)
    }
}

impl From<CasError> for GroupingCohortArchiveError {
    fn from(value: CasError) -> Self {
        Self::Storage(value)
    }
}

fn encode_trace(trace: &RawGroupingTrace) -> CanonicalValue {
    let entities = trace
        .evidence
        .entities
        .iter()
        .map(|(entity, value)| {
            CanonicalValue::Array(vec![
                CanonicalValue::Bytes(entity.0.as_bytes().to_vec()),
                signed(*value),
            ])
        })
        .collect();
    let mut relations = trace.evidence.relations.clone();
    relations.sort_by(|left, right| {
        left.role
            .cmp(&right.role)
            .then_with(|| left.subject.cmp(&right.subject))
            .then_with(|| left.object.cmp(&right.object))
    });
    let relations = relations
        .iter()
        .map(|relation| {
            CanonicalValue::Array(vec![
                CanonicalValue::Text(relation.role.0.clone()),
                CanonicalValue::Bytes(relation.subject.0.as_bytes().to_vec()),
                CanonicalValue::Bytes(relation.object.0.as_bytes().to_vec()),
            ])
        })
        .collect();
    CanonicalValue::Array(vec![
        CanonicalValue::Bytes(trace.episode.as_bytes().to_vec()),
        CanonicalValue::Array(entities),
        CanonicalValue::Array(relations),
        CanonicalValue::Array(trace.outputs.iter().copied().map(signed).collect()),
    ])
}

fn decode_trace(value: &CanonicalValue) -> Result<RawGroupingTrace, GroupingCohortArchiveError> {
    let CanonicalValue::Array(fields) = value else {
        return Err(GroupingCohortArchiveError::InvalidPayload);
    };
    let [episode, entities, relations, outputs] = fields.as_slice() else {
        return Err(GroupingCohortArchiveError::InvalidPayload);
    };
    let CanonicalValue::Array(entity_values) = entities else {
        return Err(GroupingCohortArchiveError::InvalidPayload);
    };
    let entities = entity_values
        .iter()
        .map(|value| {
            let CanonicalValue::Array(fields) = value else {
                return Err(GroupingCohortArchiveError::InvalidPayload);
            };
            let [entity, value] = fields.as_slice() else {
                return Err(GroupingCohortArchiveError::InvalidPayload);
            };
            Ok((EntityId(decode_hash(entity)?), decode_i64(value)?))
        })
        .collect::<Result<BTreeMap<_, _>, _>>()?;
    if entities.len() != entity_values.len() {
        return Err(GroupingCohortArchiveError::InvalidPayload);
    }
    let CanonicalValue::Array(relation_values) = relations else {
        return Err(GroupingCohortArchiveError::InvalidPayload);
    };
    let relations = relation_values
        .iter()
        .map(|value| {
            let CanonicalValue::Array(fields) = value else {
                return Err(GroupingCohortArchiveError::InvalidPayload);
            };
            let [role, subject, object] = fields.as_slice() else {
                return Err(GroupingCohortArchiveError::InvalidPayload);
            };
            let CanonicalValue::Text(role) = role else {
                return Err(GroupingCohortArchiveError::InvalidPayload);
            };
            Ok(RelationRecord {
                role: RelationRole(role.clone()),
                subject: EntityId(decode_hash(subject)?),
                object: EntityId(decode_hash(object)?),
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let CanonicalValue::Array(output_values) = outputs else {
        return Err(GroupingCohortArchiveError::InvalidPayload);
    };
    let outputs = output_values
        .iter()
        .map(decode_i64)
        .collect::<Result<BTreeSet<_>, _>>()?;
    if outputs.len() != output_values.len() {
        return Err(GroupingCohortArchiveError::InvalidPayload);
    }
    Ok(RawGroupingTrace {
        episode: decode_hash(episode)?,
        evidence: cairn_theory::RawGroupingEvidence {
            entities,
            relations,
        },
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

fn decode_hash(value: &CanonicalValue) -> Result<Hash32, GroupingCohortArchiveError> {
    let CanonicalValue::Bytes(bytes) = value else {
        return Err(GroupingCohortArchiveError::InvalidPayload);
    };
    let fixed: [u8; 32] = bytes
        .as_slice()
        .try_into()
        .map_err(|_| GroupingCohortArchiveError::InvalidPayload)?;
    Ok(Hash32::from_bytes(fixed))
}

fn decode_i64(value: &CanonicalValue) -> Result<i64, GroupingCohortArchiveError> {
    match value {
        CanonicalValue::Unsigned(value) => {
            i64::try_from(*value).map_err(|_| GroupingCohortArchiveError::InvalidPayload)
        }
        CanonicalValue::Negative(value) if *value < 0 => Ok(*value),
        _ => Err(GroupingCohortArchiveError::InvalidPayload),
    }
}

fn decode_u64(value: &CanonicalValue) -> Result<u64, GroupingCohortArchiveError> {
    let CanonicalValue::Unsigned(value) = value else {
        return Err(GroupingCohortArchiveError::InvalidPayload);
    };
    Ok(*value)
}

fn grouping_cohort_schema() -> Result<SchemaRef, ArtifactError> {
    SchemaRef::new(GROUPING_COHORT_SCHEMA, GROUPING_COHORT_SCHEMA_VERSION)
}

fn grouping_cohort_kind() -> Result<ArtifactKind, ArtifactError> {
    ArtifactKind::new("memory.raw-grouping-cohort")
}

const fn grouping_cohort_semantics() -> SemanticsVersion {
    SemanticsVersion::new(1, 0, 0)
}
