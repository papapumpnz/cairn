//! Proposal-only memory for independently verified scalar expressions.
//!
//! Stored expressions never carry source-channel bindings and cannot emit an
//! accepted value. Runtime users must rebind them to current observations and
//! pass any resulting proposal through the current task's verifier boundary.

use crate::{ModuleStatus, ReuseArtifactMetadata};
use cairn_cas::{Cas, CasError, StoredArtifact};
use cairn_core::{
    ArtifactEnvelope, ArtifactError, ArtifactKind, BlobId, CanonicalEncode, CanonicalLimits,
    CanonicalValue, Hash32, SchemaDescriptor, SchemaRef, SchemaRegistry, SemanticsVersion,
};
use cairn_theory::{
    LearnedScalarExpression, RelationExpressionLimits, ScalarExpressionError,
    ScalarExpressionExample, ScalarExpressionInventor, ScalarExpressionMechanism,
    ScalarExpressionTrace,
};
use cairn_verifier::{
    ScalarExpressionVerifier, ScalarExpressionVerifyError, VerifiedScalarExpressionPromotion,
    VerifiedScalarExpressionReuse,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

pub const SCALAR_EXPRESSION_COHORT_SCHEMA: &str = "cairn.memory.scalar-expression-cohort";
pub const SCALAR_EXPRESSION_COHORT_SCHEMA_VERSION: u32 = 1;

/// One source-independent scalar expression and its verifier-issued evidence.
#[derive(Clone, Debug, PartialEq)]
pub struct ScalarExpressionModule {
    id: Hash32,
    mechanism: ScalarExpressionMechanism,
    receipts: BTreeMap<Hash32, VerifiedScalarExpressionReuse>,
    experiences: BTreeSet<Hash32>,
    status: ModuleStatus,
}

impl ScalarExpressionModule {
    #[must_use]
    pub const fn id(&self) -> Hash32 {
        self.id
    }

    #[must_use]
    pub const fn mechanism(&self) -> &ScalarExpressionMechanism {
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

/// Fail-closed scalar-expression memory terminal.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScalarExpressionStoreError {
    Canonicalization,
    IdentityMismatch,
    UnknownModule,
    EmptyReplay,
    DuplicateReceipt,
    OverlappingExperience,
}

/// Exact, deterministic proposal memory keyed by canonical mechanism identity.
#[derive(Clone, Debug, Default)]
pub struct ScalarExpressionStore {
    modules: BTreeMap<Hash32, ScalarExpressionModule>,
}

impl ScalarExpressionStore {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            modules: BTreeMap::new(),
        }
    }

    /// Insert a source-independent expression without granting persistence.
    ///
    /// # Errors
    ///
    /// Rejects malformed expression/definition shape or canonical identity
    /// failure.
    pub fn insert_provisional(
        &mut self,
        mechanism: ScalarExpressionMechanism,
    ) -> Result<Hash32, ScalarExpressionStoreError> {
        let arity = usize::try_from(mechanism.expression.parameter_count())
            .map_err(|_| ScalarExpressionStoreError::IdentityMismatch)?;
        if arity < 2
            || mechanism.expression.binary_node_count() == 0
            || mechanism.definition.parameters.len() != arity
        {
            return Err(ScalarExpressionStoreError::IdentityMismatch);
        }
        let id = mechanism
            .semantic_hash()
            .map_err(|_| ScalarExpressionStoreError::Canonicalization)?;
        if let Some(existing) = self.modules.get(&id) {
            if existing.mechanism.semantic_hash().ok() != Some(id) {
                return Err(ScalarExpressionStoreError::IdentityMismatch);
            }
            return Ok(id);
        }
        self.modules.insert(
            id,
            ScalarExpressionModule {
                id,
                mechanism,
                receipts: BTreeMap::new(),
                experiences: BTreeSet::new(),
                status: ModuleStatus::Provisional,
            },
        );
        Ok(id)
    }

    /// Record one independently replayed, disjoint reuse cohort.
    ///
    /// # Errors
    ///
    /// Rejects mismatched identity, empty evidence, duplicate receipts, and
    /// overlapping experiences.
    pub fn record_verified_reuse(
        &mut self,
        id: Hash32,
        receipt: VerifiedScalarExpressionReuse,
    ) -> Result<(), ScalarExpressionStoreError> {
        let module = self
            .modules
            .get_mut(&id)
            .ok_or(ScalarExpressionStoreError::UnknownModule)?;
        if receipt.mechanism() != id {
            return Err(ScalarExpressionStoreError::IdentityMismatch);
        }
        if receipt.replayed_examples() == 0 || receipt.experiences().is_empty() {
            return Err(ScalarExpressionStoreError::EmptyReplay);
        }
        let receipt_id = Hash32::digest(
            &receipt
                .experiences()
                .iter()
                .flat_map(|experience| experience.as_bytes().iter().copied())
                .collect::<Vec<_>>(),
        );
        if module.receipts.contains_key(&receipt_id) {
            return Err(ScalarExpressionStoreError::DuplicateReceipt);
        }
        if !module.experiences.is_disjoint(receipt.experiences()) {
            return Err(ScalarExpressionStoreError::OverlappingExperience);
        }
        module.experiences.extend(receipt.experiences());
        module.receipts.insert(receipt_id, receipt);
        Ok(())
    }

    /// Apply a consolidator certificate only when it covers the complete local
    /// ledger and at least two independently scoped reuse cohorts.
    #[must_use]
    pub fn apply_verified_promotion(
        &mut self,
        id: Hash32,
        promotion: &VerifiedScalarExpressionPromotion,
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

    /// Iterate only mechanisms admitted by the independent promotion path.
    pub fn persistent_modules(&self) -> impl Iterator<Item = &ScalarExpressionModule> {
        self.modules
            .values()
            .filter(|module| module.status == ModuleStatus::Persistent)
    }

    #[must_use]
    pub fn module(&self, id: Hash32) -> Option<&ScalarExpressionModule> {
        self.modules.get(&id)
    }
}

/// Canonical scalar evidence from which a mechanism must be freshly learned.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScalarExpressionCohortArtifact {
    mechanism: Hash32,
    traces: Vec<ScalarExpressionTrace>,
    replayed_examples: u64,
    expressions_without: u64,
    expressions_with: u64,
}

impl ScalarExpressionCohortArtifact {
    #[must_use]
    pub const fn mechanism(&self) -> Hash32 {
        self.mechanism
    }

    #[must_use]
    pub fn traces(&self) -> &[ScalarExpressionTrace] {
        &self.traces
    }

    fn from_verified(
        learned: &LearnedScalarExpression,
        traces: &[ScalarExpressionTrace],
        receipt: &VerifiedScalarExpressionReuse,
    ) -> Result<Self, ScalarExpressionCohortArchiveError> {
        let mechanism = learned
            .mechanism_hash()
            .map_err(|_| ScalarExpressionCohortArchiveError::Canonicalization)?;
        let experiences = traces
            .iter()
            .map(|trace| trace.experience)
            .collect::<BTreeSet<_>>();
        if traces.len() < 2
            || experiences.len() != traces.len()
            || mechanism != receipt.mechanism()
            || &experiences != receipt.experiences()
            || receipt.replayed_examples() == 0
            || receipt.expressions_without() <= receipt.expressions_with()
        {
            return Err(ScalarExpressionCohortArchiveError::IdentityMismatch);
        }
        let mut traces = traces.to_vec();
        traces.sort_by_key(|trace| trace.experience);
        Ok(Self {
            mechanism,
            traces,
            replayed_examples: receipt.replayed_examples(),
            expressions_without: receipt.expressions_without(),
            expressions_with: receipt.expressions_with(),
        })
    }

    fn cohort_hash(&self) -> Result<Hash32, ScalarExpressionCohortArchiveError> {
        self.encode_canonical()
            .map(|bytes| Hash32::digest(&bytes))
            .map_err(|_| ScalarExpressionCohortArchiveError::Canonicalization)
    }

    fn from_value(value: CanonicalValue) -> Result<Self, ScalarExpressionCohortArchiveError> {
        let CanonicalValue::Array(fields) = value else {
            return Err(ScalarExpressionCohortArchiveError::InvalidPayload);
        };
        let [marker, mechanism, traces, examples, without, with] = fields.as_slice() else {
            return Err(ScalarExpressionCohortArchiveError::InvalidPayload);
        };
        if marker != &CanonicalValue::Text("cairn.memory.scalar-expression-cohort.v1".to_owned()) {
            return Err(ScalarExpressionCohortArchiveError::InvalidPayload);
        }
        let CanonicalValue::Array(trace_values) = traces else {
            return Err(ScalarExpressionCohortArchiveError::InvalidPayload);
        };
        let mut traces = trace_values
            .iter()
            .map(decode_scalar_trace)
            .collect::<Result<Vec<_>, _>>()?;
        traces.sort_by_key(|trace| trace.experience);
        let experiences = traces
            .iter()
            .map(|trace| trace.experience)
            .collect::<BTreeSet<_>>();
        if traces.len() < 2 || experiences.len() != traces.len() {
            return Err(ScalarExpressionCohortArchiveError::InvalidPayload);
        }
        Ok(Self {
            mechanism: decode_scalar_hash(mechanism)?,
            traces,
            replayed_examples: decode_scalar_u64(examples)?,
            expressions_without: decode_scalar_u64(without)?,
            expressions_with: decode_scalar_u64(with)?,
        })
    }
}

impl CanonicalEncode for ScalarExpressionCohortArtifact {
    fn encode_canonical(&self) -> Result<Vec<u8>, cairn_core::CanonicalError> {
        CanonicalValue::Array(vec![
            CanonicalValue::Text("cairn.memory.scalar-expression-cohort.v1".to_owned()),
            CanonicalValue::Bytes(self.mechanism.as_bytes().to_vec()),
            CanonicalValue::Array(self.traces.iter().map(encode_scalar_trace).collect()),
            CanonicalValue::Unsigned(self.replayed_examples),
            CanonicalValue::Unsigned(self.expressions_without),
            CanonicalValue::Unsigned(self.expressions_with),
        ])
        .encode_canonical()
    }
}

/// Fresh task learning and independent replay reconstructed from one artifact.
#[derive(Clone, Debug, PartialEq)]
pub struct ReplayedScalarExpressionCohort {
    pub learned: LearnedScalarExpression,
    pub receipt: VerifiedScalarExpressionReuse,
}

/// Content-addressed scalar evidence archive with no authority restoration.
#[derive(Clone, Copy, Debug, Default)]
pub struct ScalarExpressionCohortArchive;

impl ScalarExpressionCohortArchive {
    /// Persist one independently verified cohort as evidence only.
    ///
    /// # Errors
    ///
    /// Rejects missing lineage, mismatched evidence, or canonical storage
    /// failure.
    pub fn persist(
        &self,
        cas: &Cas,
        learned: &LearnedScalarExpression,
        traces: &[ScalarExpressionTrace],
        receipt: &VerifiedScalarExpressionReuse,
        metadata: ReuseArtifactMetadata,
    ) -> Result<StoredArtifact, ScalarExpressionCohortArchiveError> {
        if metadata.parent_ids.is_empty() {
            return Err(ScalarExpressionCohortArchiveError::MissingParent);
        }
        let payload = ScalarExpressionCohortArtifact::from_verified(learned, traces, receipt)?;
        let scope = payload.cohort_hash()?;
        let envelope = ArtifactEnvelope::new(
            scalar_expression_schema()?,
            scalar_expression_kind()?,
            scalar_expression_semantics(),
            metadata.parent_ids,
            Some(scope),
            metadata.producer,
            metadata.budget,
            metadata.created_at_logical,
            payload,
        )?;
        cas.put_envelope(&envelope)
            .map_err(ScalarExpressionCohortArchiveError::Storage)
    }

    /// Load bounded, structurally checked but untrusted scalar evidence.
    ///
    /// # Errors
    ///
    /// Rejects schema, semantics, lineage, hash, payload, or bound failures.
    pub fn load(
        &self,
        cas: &Cas,
        blob_id: BlobId,
    ) -> Result<ScalarExpressionCohortArtifact, ScalarExpressionCohortArchiveError> {
        let mut registry = SchemaRegistry::new();
        registry.register(SchemaDescriptor {
            name: SCALAR_EXPRESSION_COHORT_SCHEMA.to_owned(),
            current_version: SCALAR_EXPRESSION_COHORT_SCHEMA_VERSION,
            additive_read_versions: BTreeSet::new(),
            maximum_payload_bytes: 4_194_304,
        })?;
        let envelope = cas.get_envelope(
            blob_id,
            &registry,
            &BTreeSet::from([scalar_expression_semantics()]),
            CanonicalLimits {
                max_input_bytes: 5_242_880,
                max_depth: 24,
                max_collection_items: 131_072,
                max_scalar_bytes: 4_194_304,
            },
        )?;
        if envelope.artifact_kind != scalar_expression_kind()? || envelope.parent_ids.is_empty() {
            return Err(ScalarExpressionCohortArchiveError::InvalidEnvelope);
        }
        let artifact = ScalarExpressionCohortArtifact::from_value(envelope.payload)?;
        if envelope.task_scope != Some(artifact.cohort_hash()?) {
            return Err(ScalarExpressionCohortArchiveError::InvalidEnvelope);
        }
        Ok(artifact)
    }

    /// Re-invent and independently replay evidence into fresh process state.
    ///
    /// # Errors
    ///
    /// Rejects invention, identity, accounting, or verifier mismatch.
    pub fn replay(
        &self,
        artifact: &ScalarExpressionCohortArtifact,
        limits: &RelationExpressionLimits,
    ) -> Result<ReplayedScalarExpressionCohort, ScalarExpressionCohortArchiveError> {
        let learned = ScalarExpressionInventor
            .invent(artifact.traces(), limits)
            .map_err(ScalarExpressionCohortArchiveError::Invention)?;
        if learned
            .mechanism_hash()
            .map_err(|_| ScalarExpressionCohortArchiveError::Canonicalization)?
            != artifact.mechanism
        {
            return Err(ScalarExpressionCohortArchiveError::IdentityMismatch);
        }
        let receipt = ScalarExpressionVerifier
            .verify_reuse(&learned, artifact.traces(), limits)
            .map_err(ScalarExpressionCohortArchiveError::Verification)?;
        if receipt.mechanism() != artifact.mechanism
            || receipt.replayed_examples() != artifact.replayed_examples
            || receipt.expressions_without() != artifact.expressions_without
            || receipt.expressions_with() != artifact.expressions_with
        {
            return Err(ScalarExpressionCohortArchiveError::ReplayMismatch);
        }
        Ok(ReplayedScalarExpressionCohort { learned, receipt })
    }
}

#[derive(Debug)]
pub enum ScalarExpressionCohortArchiveError {
    MissingParent,
    InvalidEnvelope,
    InvalidPayload,
    Canonicalization,
    IdentityMismatch,
    ReplayMismatch,
    Invention(ScalarExpressionError),
    Verification(ScalarExpressionVerifyError),
    Artifact(ArtifactError),
    Storage(CasError),
}

impl fmt::Display for ScalarExpressionCohortArchiveError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for ScalarExpressionCohortArchiveError {}

impl From<ArtifactError> for ScalarExpressionCohortArchiveError {
    fn from(value: ArtifactError) -> Self {
        Self::Artifact(value)
    }
}

impl From<CasError> for ScalarExpressionCohortArchiveError {
    fn from(value: CasError) -> Self {
        Self::Storage(value)
    }
}

fn encode_scalar_trace(trace: &ScalarExpressionTrace) -> CanonicalValue {
    CanonicalValue::Array(vec![
        CanonicalValue::Bytes(trace.experience.as_bytes().to_vec()),
        CanonicalValue::Array(
            trace
                .examples
                .iter()
                .map(|example| {
                    CanonicalValue::Array(vec![
                        CanonicalValue::Array(
                            example.inputs.iter().copied().map(scalar_signed).collect(),
                        ),
                        scalar_signed(example.output),
                    ])
                })
                .collect(),
        ),
    ])
}

fn decode_scalar_trace(
    value: &CanonicalValue,
) -> Result<ScalarExpressionTrace, ScalarExpressionCohortArchiveError> {
    let CanonicalValue::Array(fields) = value else {
        return Err(ScalarExpressionCohortArchiveError::InvalidPayload);
    };
    let [experience, examples] = fields.as_slice() else {
        return Err(ScalarExpressionCohortArchiveError::InvalidPayload);
    };
    let CanonicalValue::Array(example_values) = examples else {
        return Err(ScalarExpressionCohortArchiveError::InvalidPayload);
    };
    if example_values.len() < 3 {
        return Err(ScalarExpressionCohortArchiveError::InvalidPayload);
    }
    let examples = example_values
        .iter()
        .map(|value| {
            let CanonicalValue::Array(fields) = value else {
                return Err(ScalarExpressionCohortArchiveError::InvalidPayload);
            };
            let [inputs, output] = fields.as_slice() else {
                return Err(ScalarExpressionCohortArchiveError::InvalidPayload);
            };
            let CanonicalValue::Array(input_values) = inputs else {
                return Err(ScalarExpressionCohortArchiveError::InvalidPayload);
            };
            if input_values.is_empty() {
                return Err(ScalarExpressionCohortArchiveError::InvalidPayload);
            }
            Ok(ScalarExpressionExample {
                inputs: input_values
                    .iter()
                    .map(decode_scalar_i64)
                    .collect::<Result<Vec<_>, _>>()?,
                output: decode_scalar_i64(output)?,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(ScalarExpressionTrace {
        experience: decode_scalar_hash(experience)?,
        examples,
    })
}

const fn scalar_signed(value: i64) -> CanonicalValue {
    if value >= 0 {
        CanonicalValue::Unsigned(value.unsigned_abs())
    } else {
        CanonicalValue::Negative(value)
    }
}

fn decode_scalar_hash(
    value: &CanonicalValue,
) -> Result<Hash32, ScalarExpressionCohortArchiveError> {
    let CanonicalValue::Bytes(bytes) = value else {
        return Err(ScalarExpressionCohortArchiveError::InvalidPayload);
    };
    let fixed: [u8; 32] = bytes
        .as_slice()
        .try_into()
        .map_err(|_| ScalarExpressionCohortArchiveError::InvalidPayload)?;
    Ok(Hash32::from_bytes(fixed))
}

fn decode_scalar_i64(value: &CanonicalValue) -> Result<i64, ScalarExpressionCohortArchiveError> {
    match value {
        CanonicalValue::Unsigned(value) => {
            i64::try_from(*value).map_err(|_| ScalarExpressionCohortArchiveError::InvalidPayload)
        }
        CanonicalValue::Negative(value) if *value < 0 => Ok(*value),
        _ => Err(ScalarExpressionCohortArchiveError::InvalidPayload),
    }
}

fn decode_scalar_u64(value: &CanonicalValue) -> Result<u64, ScalarExpressionCohortArchiveError> {
    let CanonicalValue::Unsigned(value) = value else {
        return Err(ScalarExpressionCohortArchiveError::InvalidPayload);
    };
    Ok(*value)
}

fn scalar_expression_schema() -> Result<SchemaRef, ArtifactError> {
    SchemaRef::new(
        SCALAR_EXPRESSION_COHORT_SCHEMA,
        SCALAR_EXPRESSION_COHORT_SCHEMA_VERSION,
    )
}

fn scalar_expression_kind() -> Result<ArtifactKind, ArtifactError> {
    ArtifactKind::new("memory.scalar-expression-cohort")
}

const fn scalar_expression_semantics() -> SemanticsVersion {
    SemanticsVersion::new(1, 0, 0)
}
