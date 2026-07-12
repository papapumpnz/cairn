//! Proof-gated reusable mechanism memory.
//!
//! Retrieval produces rebound RKL proposals only. Every proposal is checked
//! and replayed against the complete current problem by `cairn-invent` before
//! it can influence a prediction.

mod latent_memory;
mod layer_memory;

pub use latent_memory::*;
pub use layer_memory::*;

use cairn_cas::{Cas, CasError, StoredArtifact};
use cairn_consolidator::{
    CompiledInternalMechanism, GraphCompressionProposal, InternalEpisodeBindings,
    InternalGraphCompressionProposal, ProofGraph,
};
use cairn_core::{
    ArtifactEnvelope, ArtifactError, ArtifactId, ArtifactKind, BlobId, BudgetRecord,
    CanonicalEncode, CanonicalLimits, CanonicalValue, Hash32, ProducerRecord, SchemaDescriptor,
    SchemaRef, SchemaRegistry, SemanticsVersion,
};
use cairn_invent::{
    DefinitionInventor, DefinitionProposal, InventionError, InventionOutcome, InventionProblem,
};
use cairn_mechanism::SupportGeometry as MechanismSupportGeometry;
use cairn_rkl::CheckedTheory;
use cairn_rkl::{BoundChecker, Definition, Expr, Type, TypeChecker, VarName};
use cairn_verifier::{
    CounterfactualReuseMetrics, InternalCompressionVerifyError, InternalMechanismVerifier,
    InternalPromotionConsolidator, InternalReuseMetrics, PromotionVerifier, VerifiedCompression,
    VerifiedInternalMechanism, VerifiedInternalPromotion, VerifiedInternalReuse,
    VerifiedProceduralPromotion, VerifiedProceduralReuse, VerifiedPromotion,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

pub const MEMORY_SEMANTICS_VERSION: &str = "1.7.0";
pub const REUSE_EVIDENCE_SCHEMA: &str = "cairn.memory.reuse-evidence";
pub const REUSE_EVIDENCE_SCHEMA_VERSION: u32 = 1;
pub const INTERNAL_REUSE_EVIDENCE_SCHEMA: &str = "cairn.memory.internal-reuse-evidence";
pub const INTERNAL_REUSE_EVIDENCE_SCHEMA_VERSION: u32 = 1;
const MAXIMUM_REBOUND_PROPOSALS: usize = 1_024;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum VariationDimension {
    ParameterNames,
    NumericRanges,
    CategoricalValues,
    Counts,
    Positions,
    Layouts,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SupportGeometry {
    pub independent_tasks: u32,
    pub held_out_rebindings: u32,
    pub varied_dimensions: BTreeSet<VariationDimension>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModuleStatus {
    Provisional,
    Persistent,
    Compiled,
    Quarantined,
    Deprecated,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MechanismModule {
    id: Hash32,
    definition: Definition,
    support: SupportGeometry,
    lineage: Vec<Hash32>,
    status: ModuleStatus,
    structural_cost: u16,
    reuse: BTreeMap<Hash32, ReuseEvidence>,
    compression: Option<CompressionEvidence>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CompressionEvidence {
    pub proposal: Hash32,
    pub uncompressed_nodes: u64,
    pub compressed_nodes: u64,
    pub verified_replays: u64,
    pub compiled_definition: Hash32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CompiledMechanism {
    pub proposal: DefinitionProposal,
    pub compression: Hash32,
    pub exact_source_replays: u64,
}

/// One source-independent internal proof mechanism and its independently
/// verified runtime-reuse ledger.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InternalMechanismModule {
    mechanism: CompiledInternalMechanism,
    compilations: BTreeMap<Hash32, VerifiedInternalMechanism>,
    reuse: BTreeMap<Hash32, VerifiedInternalReuse>,
    status: ModuleStatus,
}

impl InternalMechanismModule {
    #[must_use]
    pub const fn id(&self) -> Hash32 {
        self.mechanism.semantic_hash()
    }

    #[must_use]
    pub const fn mechanism(&self) -> &CompiledInternalMechanism {
        &self.mechanism
    }

    #[must_use]
    pub const fn status(&self) -> ModuleStatus {
        self.status
    }

    #[must_use]
    pub fn verified_reuses(&self) -> usize {
        self.reuse.len()
    }

    #[must_use]
    pub fn verified_compilations(&self) -> usize {
        self.compilations.len()
    }
}

/// Result of recording runtime reuse. The store automatically asks the
/// separate consolidator for a certificate once enough evidence exists.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum InternalPromotionProgress {
    Pending { verified_reuses: usize },
    Promoted(VerifiedInternalPromotion),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InternalStoreError {
    CompilationMismatch,
    DuplicateMechanism,
    UnknownMechanism,
    DuplicateExperience,
    Unavailable,
    Consolidation(InternalCompressionVerifyError),
}

/// Memory boundary for compiled internal mechanisms. Runtime code may submit
/// only opaque verifier receipts; it cannot construct promotion certificates
/// or set persistent status directly.
#[derive(Clone, Debug, Default)]
pub struct InternalMechanismStore {
    modules: BTreeMap<Hash32, InternalMechanismModule>,
}

impl InternalMechanismStore {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            modules: BTreeMap::new(),
        }
    }

    /// Insert an independently replayed compiled mechanism as provisional.
    ///
    /// # Errors
    ///
    /// Rejects changed mechanism/proposal identity, empty source replay, and
    /// duplicate semantic mechanisms.
    pub fn insert_provisional(
        &mut self,
        mechanism: CompiledInternalMechanism,
        compilation: VerifiedInternalMechanism,
    ) -> Result<Hash32, InternalStoreError> {
        if compilation.mechanism() != mechanism.semantic_hash()
            || compilation.source_proposal() != mechanism.source_proposal()
            || compilation.replayed_sources() == 0
        {
            return Err(InternalStoreError::CompilationMismatch);
        }
        let id = mechanism.semantic_hash();
        if self.modules.contains_key(&id) {
            return Err(InternalStoreError::DuplicateMechanism);
        }
        self.modules.insert(
            id,
            InternalMechanismModule {
                mechanism,
                compilations: BTreeMap::from([(compilation.source_proposal(), compilation)]),
                reuse: BTreeMap::new(),
                status: ModuleStatus::Provisional,
            },
        );
        Ok(id)
    }

    /// Attach another independently verified evidence proposal that compiled
    /// to the same semantic mechanism. This permits later receipts from
    /// disjoint cohorts to consolidate without weakening lineage checks.
    ///
    /// # Errors
    ///
    /// Rejects unknown mechanisms, changed mechanism identity, empty replay,
    /// and duplicate proposal lineage.
    pub fn register_compilation(
        &mut self,
        id: Hash32,
        mechanism: &CompiledInternalMechanism,
        compilation: VerifiedInternalMechanism,
    ) -> Result<(), InternalStoreError> {
        let module = self
            .modules
            .get_mut(&id)
            .ok_or(InternalStoreError::UnknownMechanism)?;
        if id != mechanism.semantic_hash()
            || id != compilation.mechanism()
            || mechanism.semantic_hash() != module.mechanism.semantic_hash()
            || compilation.source_proposal() != mechanism.source_proposal()
            || compilation.replayed_sources() == 0
        {
            return Err(InternalStoreError::CompilationMismatch);
        }
        if module
            .compilations
            .contains_key(&compilation.source_proposal())
        {
            return Err(InternalStoreError::DuplicateMechanism);
        }
        module
            .compilations
            .insert(compilation.source_proposal(), compilation);
        Ok(())
    }

    /// Record one verifier-issued runtime receipt and automatically request a
    /// consolidation certificate. The first independent receipt remains
    /// pending; the second qualifying receipt promotes the module.
    ///
    /// # Errors
    ///
    /// Rejects unknown/unavailable mechanisms, mixed identities, duplicate
    /// experiences, and consolidator refusal.
    pub fn record_runtime_reuse(
        &mut self,
        id: Hash32,
        receipt: VerifiedInternalReuse,
    ) -> Result<InternalPromotionProgress, InternalStoreError> {
        let module = self
            .modules
            .get_mut(&id)
            .ok_or(InternalStoreError::UnknownMechanism)?;
        if !matches!(
            module.status,
            ModuleStatus::Provisional | ModuleStatus::Persistent | ModuleStatus::Compiled
        ) {
            return Err(InternalStoreError::Unavailable);
        }
        if receipt.mechanism() != id
            || !module.compilations.contains_key(&receipt.source_proposal())
        {
            return Err(InternalStoreError::CompilationMismatch);
        }
        if module.reuse.contains_key(&receipt.experience()) {
            return Err(InternalStoreError::DuplicateExperience);
        }
        module.reuse.insert(receipt.experience(), receipt);
        if module.reuse.len() < 2 {
            return Ok(InternalPromotionProgress::Pending {
                verified_reuses: module.reuse.len(),
            });
        }
        let receipts = module.reuse.values().cloned().collect::<Vec<_>>();
        let compilations = module.compilations.values().cloned().collect::<Vec<_>>();
        let certificate = InternalPromotionConsolidator
            .consolidate(&module.mechanism, &compilations, &receipts)
            .map_err(InternalStoreError::Consolidation)?;
        module.status = ModuleStatus::Persistent;
        Ok(InternalPromotionProgress::Promoted(certificate))
    }

    #[must_use]
    pub fn get(&self, id: Hash32) -> Option<&InternalMechanismModule> {
        self.modules.get(&id)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.modules.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.modules.is_empty()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CompilationError {
    EmptySources,
    CompressionLineageMismatch,
    IncompatibleDefinitions,
    UnsupportedRecursion,
    TypeCheck,
    SourceReplayMismatch,
    Canonicalization,
    Capacity,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct MechanismCompiler;

impl MechanismCompiler {
    /// Compile a replayed proof cohort into a newly named, alpha-normalized RKL
    /// definition whose type bounds cover every observed source definition.
    ///
    /// # Errors
    ///
    /// Rejects mismatched proof lineage, incompatible source structure,
    /// recursion in this initial compiler, failed RKL checking, or any source
    /// definition that cannot be reconstructed by binder rebinding.
    pub fn compile(
        &self,
        sources: &[DefinitionProposal],
        compression: &GraphCompressionProposal,
    ) -> Result<CompiledMechanism, CompilationError> {
        if sources.is_empty() {
            return Err(CompilationError::EmptySources);
        }
        let source_ids = sources
            .iter()
            .map(|source| source.semantic_hash)
            .collect::<BTreeSet<_>>();
        let compressed_ids = compression
            .source_episodes()
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        if source_ids != compressed_ids || source_ids.len() != sources.len() {
            return Err(CompilationError::CompressionLineageMismatch);
        }
        let representative = &sources[0].definition;
        if sources
            .iter()
            .any(|source| !equivalent_definition_shape(representative, &source.definition))
        {
            return Err(CompilationError::IncompatibleDefinitions);
        }
        if sources
            .iter()
            .any(|source| source.definition.recursion.is_some())
        {
            return Err(CompilationError::UnsupportedRecursion);
        }
        let parameters = (0..representative.parameters.len())
            .map(|index| {
                let observed = sources
                    .iter()
                    .map(|source| &source.definition.parameters[index].ty)
                    .collect::<Vec<_>>();
                Ok(cairn_rkl::Binder {
                    name: VarName(format!("parameter_{index}")),
                    ty: generalize_types(&observed)?,
                })
            })
            .collect::<Result<Vec<_>, CompilationError>>()?;
        let return_types = sources
            .iter()
            .map(|source| &source.definition.return_type)
            .collect::<Vec<_>>();
        let definition = Definition {
            name: format!(
                "invented_mechanism_{}",
                &compression.semantic_hash().to_hex()[..16]
            ),
            parameters,
            return_type: generalize_types(&return_types)?,
            body: normalized_body(representative),
            recursion: None,
            proof_annotations: compression
                .source_episodes()
                .iter()
                .copied()
                .chain(std::iter::once(compression.semantic_hash()))
                .collect(),
        };
        TypeChecker::new(100_000)
            .check_definition(&definition)
            .map_err(|_| CompilationError::TypeCheck)?;
        let source_replays = sources.iter().try_fold(0_u64, |count, source| {
            if normalized_body(&source.definition) != definition.body {
                return Err(CompilationError::SourceReplayMismatch);
            }
            count.checked_add(1).ok_or(CompilationError::Capacity)
        })?;
        let bytes = definition
            .encode_canonical()
            .map_err(|_| CompilationError::Canonicalization)?;
        let structural_cost = sources
            .iter()
            .map(|source| source.structural_cost)
            .min()
            .ok_or(CompilationError::EmptySources)?;
        Ok(CompiledMechanism {
            proposal: DefinitionProposal {
                definition,
                structural_cost,
                semantic_hash: Hash32::digest(&bytes),
                candidates_evaluated: 0,
                observational_classes: 0,
            },
            compression: compression.semantic_hash(),
            exact_source_replays: source_replays,
        })
    }
}

impl MechanismModule {
    #[must_use]
    pub const fn id(&self) -> Hash32 {
        self.id
    }

    #[must_use]
    pub const fn definition(&self) -> &Definition {
        &self.definition
    }

    #[must_use]
    pub const fn support(&self) -> &SupportGeometry {
        &self.support
    }

    #[must_use]
    pub fn lineage(&self) -> &[Hash32] {
        &self.lineage
    }

    #[must_use]
    pub const fn status(&self) -> ModuleStatus {
        self.status
    }

    #[must_use]
    pub fn reuse_metrics(&self) -> ReuseMetrics {
        reuse_metrics(self.reuse.values())
    }

    #[must_use]
    pub const fn compression_evidence(&self) -> Option<CompressionEvidence> {
        self.compression
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReuseEvidence {
    pub experience: Hash32,
    pub independently_verified: bool,
    pub necessary_under_ablation: bool,
    pub candidate_executions_without: u64,
    pub candidate_executions_with: u64,
    pub examples_without: u32,
    pub examples_with: u32,
    pub proof_nodes_without: u64,
    pub proof_nodes_with: u64,
}

/// Canonical, content-addressable evidence that an independently verified
/// procedural mechanism changed one task's learning cost.
///
/// Loading this value does not make it trusted. A caller must supply the
/// referenced experience again to [`ReuseArtifactArchive::replay`] before the
/// receipt may enter a reuse ledger or promotion decision.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReuseEvidenceArtifact {
    theory_hash: Hash32,
    experience: Hash32,
    verified_cases: u64,
    metrics: CounterfactualReuseMetrics,
}

impl ReuseEvidenceArtifact {
    fn from_receipt(receipt: &VerifiedProceduralReuse) -> Self {
        Self {
            theory_hash: receipt.theory_hash(),
            experience: receipt.experience(),
            verified_cases: receipt.verified_cases(),
            metrics: receipt.metrics(),
        }
    }

    #[must_use]
    pub const fn theory_hash(self) -> Hash32 {
        self.theory_hash
    }

    #[must_use]
    pub const fn experience(self) -> Hash32 {
        self.experience
    }

    #[must_use]
    pub const fn verified_cases(self) -> u64 {
        self.verified_cases
    }

    #[must_use]
    pub const fn metrics(self) -> CounterfactualReuseMetrics {
        self.metrics
    }

    fn from_value(value: CanonicalValue) -> Result<Self, ReuseArchiveError> {
        let CanonicalValue::Array(values) = value else {
            return Err(ReuseArchiveError::InvalidPayload("payload is not an array"));
        };
        let [marker, theory, experience, verified_cases, blank_solved, candidates_without, candidates_with, examples_without, examples_with, proof_without, proof_with] =
            values.as_slice()
        else {
            return Err(ReuseArchiveError::InvalidPayload(
                "payload field count is not eleven",
            ));
        };
        if marker != &CanonicalValue::Text("cairn.memory.reuse-evidence.v1".to_owned()) {
            return Err(ReuseArchiveError::InvalidPayload("payload marker mismatch"));
        }
        Ok(Self {
            theory_hash: artifact_hash(theory)?,
            experience: artifact_hash(experience)?,
            verified_cases: artifact_u64(verified_cases)?,
            metrics: CounterfactualReuseMetrics {
                blank_solved: artifact_bool(blank_solved)?,
                candidate_executions_without: artifact_u64(candidates_without)?,
                candidate_executions_with: artifact_u64(candidates_with)?,
                examples_without: artifact_u32(examples_without)?,
                examples_with: artifact_u32(examples_with)?,
                proof_nodes_without: artifact_u64(proof_without)?,
                proof_nodes_with: artifact_u64(proof_with)?,
            },
        })
    }
}

impl CanonicalEncode for ReuseEvidenceArtifact {
    fn encode_canonical(&self) -> Result<Vec<u8>, cairn_core::CanonicalError> {
        CanonicalValue::Array(vec![
            CanonicalValue::Text("cairn.memory.reuse-evidence.v1".to_owned()),
            CanonicalValue::Bytes(self.theory_hash.as_bytes().to_vec()),
            CanonicalValue::Bytes(self.experience.as_bytes().to_vec()),
            CanonicalValue::Unsigned(self.verified_cases),
            CanonicalValue::Bool(self.metrics.blank_solved),
            CanonicalValue::Unsigned(self.metrics.candidate_executions_without),
            CanonicalValue::Unsigned(self.metrics.candidate_executions_with),
            CanonicalValue::Unsigned(u64::from(self.metrics.examples_without)),
            CanonicalValue::Unsigned(u64::from(self.metrics.examples_with)),
            CanonicalValue::Unsigned(self.metrics.proof_nodes_without),
            CanonicalValue::Unsigned(self.metrics.proof_nodes_with),
        ])
        .encode_canonical()
    }
}

/// Canonical commitment to one verifier-issued internal mechanism reuse.
/// Loaded values are untrusted until [`InternalReuseArtifactArchive::replay`]
/// reconstructs the mechanism, source cohort, current host, and fixed-work
/// metrics.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InternalReuseEvidenceArtifact {
    mechanism: Hash32,
    source_proposal: Hash32,
    experience: Hash32,
    verified_cases: u64,
    metrics: InternalReuseMetrics,
}

impl InternalReuseEvidenceArtifact {
    fn from_receipt(receipt: &VerifiedInternalReuse) -> Self {
        Self {
            mechanism: receipt.mechanism(),
            source_proposal: receipt.source_proposal(),
            experience: receipt.experience(),
            verified_cases: receipt.verified_cases(),
            metrics: receipt.metrics(),
        }
    }

    #[must_use]
    pub const fn mechanism(self) -> Hash32 {
        self.mechanism
    }

    #[must_use]
    pub const fn source_proposal(self) -> Hash32 {
        self.source_proposal
    }

    #[must_use]
    pub const fn experience(self) -> Hash32 {
        self.experience
    }

    #[must_use]
    pub const fn verified_cases(self) -> u64 {
        self.verified_cases
    }

    #[must_use]
    pub const fn metrics(self) -> InternalReuseMetrics {
        self.metrics
    }

    fn from_value(value: CanonicalValue) -> Result<Self, InternalReuseArchiveError> {
        let CanonicalValue::Array(values) = value else {
            return Err(InternalReuseArchiveError::InvalidPayload(
                "payload is not an array",
            ));
        };
        let [marker, mechanism, proposal, experience, verified_cases, candidates_without, candidates_with, examples_without, examples_with, proof_without, proof_with] =
            values.as_slice()
        else {
            return Err(InternalReuseArchiveError::InvalidPayload(
                "payload field count is not eleven",
            ));
        };
        if marker != &CanonicalValue::Text("cairn.memory.internal-reuse-evidence.v1".to_owned()) {
            return Err(InternalReuseArchiveError::InvalidPayload(
                "payload marker mismatch",
            ));
        }
        Ok(Self {
            mechanism: internal_artifact_hash(mechanism)?,
            source_proposal: internal_artifact_hash(proposal)?,
            experience: internal_artifact_hash(experience)?,
            verified_cases: internal_artifact_u64(verified_cases)?,
            metrics: InternalReuseMetrics {
                candidate_windows_without: internal_artifact_u64(candidates_without)?,
                candidate_windows_with: internal_artifact_u64(candidates_with)?,
                examples_without: internal_artifact_u32(examples_without)?,
                examples_with: internal_artifact_u32(examples_with)?,
                proof_units_without: internal_artifact_u64(proof_without)?,
                proof_units_with: internal_artifact_u64(proof_with)?,
            },
        })
    }
}

impl CanonicalEncode for InternalReuseEvidenceArtifact {
    fn encode_canonical(&self) -> Result<Vec<u8>, cairn_core::CanonicalError> {
        CanonicalValue::Array(vec![
            CanonicalValue::Text("cairn.memory.internal-reuse-evidence.v1".to_owned()),
            CanonicalValue::Bytes(self.mechanism.as_bytes().to_vec()),
            CanonicalValue::Bytes(self.source_proposal.as_bytes().to_vec()),
            CanonicalValue::Bytes(self.experience.as_bytes().to_vec()),
            CanonicalValue::Unsigned(self.verified_cases),
            CanonicalValue::Unsigned(self.metrics.candidate_windows_without),
            CanonicalValue::Unsigned(self.metrics.candidate_windows_with),
            CanonicalValue::Unsigned(u64::from(self.metrics.examples_without)),
            CanonicalValue::Unsigned(u64::from(self.metrics.examples_with)),
            CanonicalValue::Unsigned(self.metrics.proof_units_without),
            CanonicalValue::Unsigned(self.metrics.proof_units_with),
        ])
        .encode_canonical()
    }
}

/// Complete evidence required to regain authority for one loaded internal
/// reuse claim. All references are current-process, independently reconstructed
/// values rather than deserialized verifier state.
#[derive(Clone, Copy)]
pub struct InternalReuseReplay<'a> {
    pub proposal: &'a InternalGraphCompressionProposal,
    pub mechanism: &'a CompiledInternalMechanism,
    pub compilation: &'a VerifiedInternalMechanism,
    pub sources: &'a [ProofGraph],
    pub held_out: &'a ProofGraph,
    pub binding: &'a InternalEpisodeBindings,
    pub maximum_match_windows: usize,
}

/// File-backed archive for untrusted internal reuse commitments.
#[derive(Clone, Copy, Debug, Default)]
pub struct InternalReuseArtifactArchive;

impl InternalReuseArtifactArchive {
    /// Persist one opaque verifier receipt with explicit parent lineage.
    ///
    /// # Errors
    ///
    /// Rejects missing lineage and artifact, canonicalization, storage, or
    /// resource failures.
    pub fn persist(
        &self,
        cas: &Cas,
        receipt: &VerifiedInternalReuse,
        metadata: ReuseArtifactMetadata,
    ) -> Result<StoredArtifact, InternalReuseArchiveError> {
        if metadata.parent_ids.is_empty() {
            return Err(InternalReuseArchiveError::MissingParent);
        }
        let envelope = ArtifactEnvelope::new(
            internal_reuse_schema()?,
            internal_reuse_kind()?,
            internal_reuse_semantics(),
            metadata.parent_ids,
            Some(receipt.experience()),
            metadata.producer,
            metadata.budget,
            metadata.created_at_logical,
            InternalReuseEvidenceArtifact::from_receipt(receipt),
        )?;
        cas.put_envelope(&envelope)
            .map_err(InternalReuseArchiveError::Storage)
    }

    /// Load and structurally validate an untrusted internal reuse commitment.
    ///
    /// # Errors
    ///
    /// Rejects corrupt storage, unsupported schema/semantics, missing lineage,
    /// malformed bounds, and experience-scope disagreement.
    pub fn load(
        &self,
        cas: &Cas,
        blob_id: BlobId,
    ) -> Result<InternalReuseEvidenceArtifact, InternalReuseArchiveError> {
        let mut registry = SchemaRegistry::new();
        registry.register(SchemaDescriptor {
            name: INTERNAL_REUSE_EVIDENCE_SCHEMA.to_owned(),
            current_version: INTERNAL_REUSE_EVIDENCE_SCHEMA_VERSION,
            additive_read_versions: BTreeSet::new(),
            maximum_payload_bytes: 2_048,
        })?;
        let envelope = cas.get_envelope(
            blob_id,
            &registry,
            &BTreeSet::from([internal_reuse_semantics()]),
            CanonicalLimits {
                max_input_bytes: 16_384,
                max_depth: 16,
                max_collection_items: 64,
                max_scalar_bytes: 4_096,
            },
        )?;
        if envelope.artifact_kind != internal_reuse_kind()? || envelope.parent_ids.is_empty() {
            return Err(InternalReuseArchiveError::InvalidEnvelope);
        }
        let artifact = InternalReuseEvidenceArtifact::from_value(envelope.payload)?;
        if envelope.task_scope != Some(artifact.experience) {
            return Err(InternalReuseArchiveError::InvalidEnvelope);
        }
        Ok(artifact)
    }

    /// Reconstruct an opaque verifier receipt from current proof evidence and
    /// require every freshly derived identity and metric to equal the loaded
    /// commitment.
    ///
    /// # Errors
    ///
    /// Rejects changed mechanism/proposal/experience, failed exact replay, or
    /// any changed candidate, sample, proof, or verified-case measurement.
    pub fn replay(
        &self,
        artifact: InternalReuseEvidenceArtifact,
        evidence: InternalReuseReplay<'_>,
    ) -> Result<VerifiedInternalReuse, InternalReuseArchiveError> {
        if artifact.mechanism != evidence.mechanism.semantic_hash()
            || artifact.source_proposal != evidence.proposal.semantic_hash()
            || artifact.experience != evidence.held_out.episode
        {
            return Err(InternalReuseArchiveError::IdentityMismatch);
        }
        let receipt = InternalMechanismVerifier
            .verify_reuse(
                evidence.proposal,
                evidence.mechanism,
                evidence.compilation,
                evidence.sources,
                evidence.held_out,
                evidence.binding,
                evidence.maximum_match_windows,
            )
            .map_err(InternalReuseArchiveError::VerificationFailure)?;
        if receipt.mechanism() != artifact.mechanism
            || receipt.source_proposal() != artifact.source_proposal
            || receipt.experience() != artifact.experience
            || receipt.verified_cases() != artifact.verified_cases
            || receipt.metrics() != artifact.metrics
        {
            return Err(InternalReuseArchiveError::ReplayMismatch);
        }
        Ok(receipt)
    }
}

#[derive(Debug)]
pub enum InternalReuseArchiveError {
    MissingParent,
    InvalidEnvelope,
    InvalidPayload(&'static str),
    IdentityMismatch,
    ReplayMismatch,
    VerificationFailure(InternalCompressionVerifyError),
    Artifact(ArtifactError),
    Storage(CasError),
}

impl fmt::Display for InternalReuseArchiveError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPayload(message) => {
                write!(formatter, "invalid internal reuse artifact: {message}")
            }
            Self::Artifact(error) => error.fmt(formatter),
            Self::Storage(error) => error.fmt(formatter),
            other => write!(formatter, "{other:?}"),
        }
    }
}

impl std::error::Error for InternalReuseArchiveError {}

impl From<ArtifactError> for InternalReuseArchiveError {
    fn from(value: ArtifactError) -> Self {
        Self::Artifact(value)
    }
}

impl From<CasError> for InternalReuseArchiveError {
    fn from(value: CasError) -> Self {
        Self::Storage(value)
    }
}

/// Non-semantic producer and resource observations attached to a persisted
/// reuse receipt. Parent IDs must identify the evidence needed for replay.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReuseArtifactMetadata {
    pub parent_ids: Vec<ArtifactId>,
    pub producer: ProducerRecord,
    pub budget: BudgetRecord,
    pub created_at_logical: u64,
}

/// File-backed archive boundary for verifier-issued reuse receipts.
#[derive(Clone, Copy, Debug, Default)]
pub struct ReuseArtifactArchive;

impl ReuseArtifactArchive {
    /// Persist one verifier-issued receipt in the canonical append-only CAS.
    ///
    /// # Errors
    ///
    /// Rejects missing lineage, invalid artifact metadata, capacity, storage,
    /// hash, and canonicalization failures before returning an identity.
    pub fn persist(
        &self,
        cas: &Cas,
        receipt: &VerifiedProceduralReuse,
        metadata: ReuseArtifactMetadata,
    ) -> Result<StoredArtifact, ReuseArchiveError> {
        if metadata.parent_ids.is_empty() {
            return Err(ReuseArchiveError::MissingParent);
        }
        let payload = ReuseEvidenceArtifact::from_receipt(receipt);
        let envelope = ArtifactEnvelope::new(
            reuse_schema()?,
            reuse_kind()?,
            reuse_semantics(),
            metadata.parent_ids,
            Some(receipt.experience()),
            metadata.producer,
            metadata.budget,
            metadata.created_at_logical,
            payload,
        )?;
        cas.put_envelope(&envelope)
            .map_err(ReuseArchiveError::Storage)
    }

    /// Load and structurally validate an untrusted reuse-evidence artifact.
    ///
    /// # Errors
    ///
    /// Rejects storage corruption, unsupported schema or semantics, malformed
    /// payloads, missing lineage, and mismatched task scope.
    pub fn load(
        &self,
        cas: &Cas,
        blob_id: BlobId,
    ) -> Result<ReuseEvidenceArtifact, ReuseArchiveError> {
        let mut registry = SchemaRegistry::new();
        registry.register(SchemaDescriptor {
            name: REUSE_EVIDENCE_SCHEMA.to_owned(),
            current_version: REUSE_EVIDENCE_SCHEMA_VERSION,
            additive_read_versions: BTreeSet::new(),
            maximum_payload_bytes: 2_048,
        })?;
        let envelope = cas.get_envelope(
            blob_id,
            &registry,
            &BTreeSet::from([reuse_semantics()]),
            CanonicalLimits {
                max_input_bytes: 16_384,
                max_depth: 16,
                max_collection_items: 64,
                max_scalar_bytes: 4_096,
            },
        )?;
        if envelope.artifact_kind != reuse_kind()? || envelope.parent_ids.is_empty() {
            return Err(ReuseArchiveError::InvalidEnvelope);
        }
        let artifact = ReuseEvidenceArtifact::from_value(envelope.payload)?;
        if envelope.task_scope != Some(artifact.experience) {
            return Err(ReuseArchiveError::InvalidEnvelope);
        }
        Ok(artifact)
    }

    /// Independently reconstruct a verifier receipt from a loaded artifact and
    /// the referenced evidence. The artifact's measured fields are proposals;
    /// the verifier re-executes the theory using freshly measured
    /// counterfactual metrics before returning an opaque receipt.
    ///
    /// # Errors
    ///
    /// Rejects theory identity mismatch, failed exact replay, changed metrics,
    /// or changed verified-case count.
    pub fn replay(
        &self,
        artifact: ReuseEvidenceArtifact,
        theory: &CheckedTheory,
        problem: &cairn_core::CollectionProcedureProblem,
        expected_prediction: &BTreeSet<i64>,
        replayed_metrics: CounterfactualReuseMetrics,
    ) -> Result<VerifiedProceduralReuse, ReuseArchiveError> {
        let theory_hash = Hash32::digest(
            &theory
                .encode_canonical()
                .map_err(|_| ReuseArchiveError::Canonicalization)?,
        );
        if theory_hash != artifact.theory_hash {
            return Err(ReuseArchiveError::TheoryMismatch);
        }
        let receipt = PromotionVerifier
            .verify_collection_reuse(
                theory,
                problem,
                expected_prediction,
                artifact.experience,
                replayed_metrics,
            )
            .map_err(|_| ReuseArchiveError::VerificationFailure)?;
        if receipt.theory_hash() != artifact.theory_hash
            || receipt.experience() != artifact.experience
            || receipt.verified_cases() != artifact.verified_cases
            || receipt.metrics() != artifact.metrics
        {
            return Err(ReuseArchiveError::ReplayMismatch);
        }
        Ok(receipt)
    }
}

#[derive(Debug)]
pub enum ReuseArchiveError {
    MissingParent,
    InvalidEnvelope,
    InvalidPayload(&'static str),
    Canonicalization,
    TheoryMismatch,
    VerificationFailure,
    ReplayMismatch,
    Artifact(ArtifactError),
    Storage(CasError),
}

impl fmt::Display for ReuseArchiveError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPayload(message) => write!(formatter, "invalid reuse artifact: {message}"),
            Self::Artifact(error) => error.fmt(formatter),
            Self::Storage(error) => error.fmt(formatter),
            other => write!(formatter, "{other:?}"),
        }
    }
}

impl std::error::Error for ReuseArchiveError {}

impl From<ArtifactError> for ReuseArchiveError {
    fn from(value: ArtifactError) -> Self {
        Self::Artifact(value)
    }
}

impl From<CasError> for ReuseArchiveError {
    fn from(value: CasError) -> Self {
        Self::Storage(value)
    }
}

fn reuse_schema() -> Result<SchemaRef, ArtifactError> {
    SchemaRef::new(REUSE_EVIDENCE_SCHEMA, REUSE_EVIDENCE_SCHEMA_VERSION)
}

fn reuse_kind() -> Result<ArtifactKind, ArtifactError> {
    ArtifactKind::new("memory.reuse-evidence")
}

const fn reuse_semantics() -> SemanticsVersion {
    SemanticsVersion::new(1, 0, 0)
}

fn internal_reuse_schema() -> Result<SchemaRef, ArtifactError> {
    SchemaRef::new(
        INTERNAL_REUSE_EVIDENCE_SCHEMA,
        INTERNAL_REUSE_EVIDENCE_SCHEMA_VERSION,
    )
}

fn internal_reuse_kind() -> Result<ArtifactKind, ArtifactError> {
    ArtifactKind::new("memory.internal-reuse-evidence")
}

const fn internal_reuse_semantics() -> SemanticsVersion {
    SemanticsVersion::new(1, 0, 0)
}

fn internal_artifact_hash(value: &CanonicalValue) -> Result<Hash32, InternalReuseArchiveError> {
    let CanonicalValue::Bytes(bytes) = value else {
        return Err(InternalReuseArchiveError::InvalidPayload(
            "hash is not bytes",
        ));
    };
    let fixed: [u8; 32] = bytes
        .as_slice()
        .try_into()
        .map_err(|_| InternalReuseArchiveError::InvalidPayload("hash is not 32 bytes"))?;
    Ok(Hash32::from_bytes(fixed))
}

fn internal_artifact_u64(value: &CanonicalValue) -> Result<u64, InternalReuseArchiveError> {
    let CanonicalValue::Unsigned(value) = value else {
        return Err(InternalReuseArchiveError::InvalidPayload(
            "metric is not unsigned",
        ));
    };
    Ok(*value)
}

fn internal_artifact_u32(value: &CanonicalValue) -> Result<u32, InternalReuseArchiveError> {
    u32::try_from(internal_artifact_u64(value)?)
        .map_err(|_| InternalReuseArchiveError::InvalidPayload("metric exceeds u32"))
}

fn artifact_hash(value: &CanonicalValue) -> Result<Hash32, ReuseArchiveError> {
    let CanonicalValue::Bytes(bytes) = value else {
        return Err(ReuseArchiveError::InvalidPayload("hash is not bytes"));
    };
    let fixed: [u8; 32] = bytes
        .as_slice()
        .try_into()
        .map_err(|_| ReuseArchiveError::InvalidPayload("hash is not 32 bytes"))?;
    Ok(Hash32::from_bytes(fixed))
}

fn artifact_u64(value: &CanonicalValue) -> Result<u64, ReuseArchiveError> {
    let CanonicalValue::Unsigned(value) = value else {
        return Err(ReuseArchiveError::InvalidPayload("metric is not unsigned"));
    };
    Ok(*value)
}

fn artifact_u32(value: &CanonicalValue) -> Result<u32, ReuseArchiveError> {
    u32::try_from(artifact_u64(value)?)
        .map_err(|_| ReuseArchiveError::InvalidPayload("metric exceeds u32"))
}

fn artifact_bool(value: &CanonicalValue) -> Result<bool, ReuseArchiveError> {
    let CanonicalValue::Bool(value) = value else {
        return Err(ReuseArchiveError::InvalidPayload("flag is not Boolean"));
    };
    Ok(*value)
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ReuseMetrics {
    pub verified_invocations: u64,
    pub necessary_invocations: u64,
    pub candidate_executions_without: u64,
    pub candidate_executions_with: u64,
    pub examples_without: u64,
    pub examples_with: u64,
    pub proof_nodes_without: u64,
    pub proof_nodes_with: u64,
    pub negative_transfers: u64,
}

fn reuse_metrics<'a>(evidence: impl Iterator<Item = &'a ReuseEvidence>) -> ReuseMetrics {
    let mut metrics = ReuseMetrics::default();
    for evidence in evidence {
        metrics.verified_invocations += 1;
        metrics.necessary_invocations += u64::from(evidence.necessary_under_ablation);
        metrics.candidate_executions_without = metrics
            .candidate_executions_without
            .saturating_add(evidence.candidate_executions_without);
        metrics.candidate_executions_with = metrics
            .candidate_executions_with
            .saturating_add(evidence.candidate_executions_with);
        metrics.examples_without = metrics
            .examples_without
            .saturating_add(u64::from(evidence.examples_without));
        metrics.examples_with = metrics
            .examples_with
            .saturating_add(u64::from(evidence.examples_with));
        metrics.proof_nodes_without = metrics
            .proof_nodes_without
            .saturating_add(evidence.proof_nodes_without);
        metrics.proof_nodes_with = metrics
            .proof_nodes_with
            .saturating_add(evidence.proof_nodes_with);
        metrics.negative_transfers += u64::from(
            evidence.candidate_executions_with > evidence.candidate_executions_without
                || evidence.examples_with > evidence.examples_without,
        );
    }
    metrics
}

fn validate_reuse(
    existing: &BTreeMap<Hash32, ReuseEvidence>,
    evidence: &ReuseEvidence,
) -> Result<(), ReuseError> {
    if !evidence.independently_verified {
        return Err(ReuseError::Unverified);
    }
    if evidence.candidate_executions_without == 0 || evidence.examples_without == 0 {
        return Err(ReuseError::EmptyCounterfactual);
    }
    if existing.contains_key(&evidence.experience) {
        return Err(ReuseError::DuplicateExperience);
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReuseError {
    UnknownModule,
    NotPersistent,
    Unverified,
    DuplicateExperience,
    EmptyCounterfactual,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PromotionError {
    InsufficientTasks { actual: u32, required: u32 },
    MissingHeldOutRebinding,
    MissingVariation,
    EmptyEvidence,
    IncompatibleDefinitions,
    Canonicalization,
    CompressionLineageMismatch,
    UnverifiedCompression,
    Compilation(CompilationError),
}

impl fmt::Display for PromotionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for PromotionError {}

#[derive(Clone, Copy, Debug, Default)]
pub struct PromotionRunner;

impl PromotionRunner {
    /// Promote only independently admitted, structurally equivalent proposals
    /// with cross-task and held-out support.
    ///
    /// # Errors
    ///
    /// Returns an exact missing-evidence or incompatibility reason.
    pub fn promote(
        &self,
        proposals: &[DefinitionProposal],
        support: SupportGeometry,
    ) -> Result<MechanismModule, PromotionError> {
        if proposals.is_empty() {
            return Err(PromotionError::EmptyEvidence);
        }
        if support.independent_tasks < 2 {
            return Err(PromotionError::InsufficientTasks {
                actual: support.independent_tasks,
                required: 2,
            });
        }
        if support.held_out_rebindings == 0 {
            return Err(PromotionError::MissingHeldOutRebinding);
        }
        if support.varied_dimensions.is_empty() {
            return Err(PromotionError::MissingVariation);
        }
        let representative = &proposals[0];
        if proposals.iter().skip(1).any(|proposal| {
            !equivalent_definition_shape(&representative.definition, &proposal.definition)
        }) {
            return Err(PromotionError::IncompatibleDefinitions);
        }
        let mut lineage = proposals
            .iter()
            .map(|proposal| proposal.semantic_hash)
            .collect::<Vec<_>>();
        lineage.sort_unstable();
        lineage.dedup();
        let id = module_identity(&representative.definition, &lineage, &support)?;
        Ok(MechanismModule {
            id,
            definition: representative.definition.clone(),
            support,
            lineage,
            status: ModuleStatus::Provisional,
            structural_cost: representative.structural_cost,
            reuse: BTreeMap::new(),
            compression: None,
        })
    }

    /// Create a provisional module only when its independently invented
    /// definitions are also explained by an exactly replayed proof-subgraph
    /// compression proposal.
    ///
    /// # Errors
    ///
    /// In addition to ordinary promotion errors, rejects compression over a
    /// different source lineage or without positive, replayed compression.
    pub fn promote_compressed(
        &self,
        proposals: &[DefinitionProposal],
        support: SupportGeometry,
        compression: &GraphCompressionProposal,
        verified: &VerifiedCompression,
    ) -> Result<MechanismModule, PromotionError> {
        let proposal_sources = proposals
            .iter()
            .map(|proposal| proposal.semantic_hash)
            .collect::<BTreeSet<_>>();
        let compression_sources = compression
            .source_episodes()
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        if proposal_sources != compression_sources || proposal_sources.len() != proposals.len() {
            return Err(PromotionError::CompressionLineageMismatch);
        }
        if compression.verified_replays()
            != u64::try_from(proposals.len()).map_err(|_| PromotionError::UnverifiedCompression)?
            || compression.compressed_nodes() >= compression.uncompressed_nodes()
            || verified.proposal() != compression.semantic_hash()
            || verified.source_episodes() != &compression_sources
            || verified.uncompressed_nodes() != compression.uncompressed_nodes()
            || verified.compressed_nodes() != compression.compressed_nodes()
            || verified.replayed_sources() != compression.verified_replays()
        {
            return Err(PromotionError::UnverifiedCompression);
        }
        let compiled = MechanismCompiler
            .compile(proposals, compression)
            .map_err(PromotionError::Compilation)?;
        let mut module = self.promote(proposals, support)?;
        module.definition = compiled.proposal.definition;
        module.structural_cost = compiled.proposal.structural_cost;
        module.id = module_identity(&module.definition, &module.lineage, &module.support)?;
        module.compression = Some(CompressionEvidence {
            proposal: compression.semantic_hash(),
            uncompressed_nodes: compression.uncompressed_nodes(),
            compressed_nodes: compression.compressed_nodes(),
            verified_replays: compression.verified_replays(),
            compiled_definition: compiled.proposal.semantic_hash,
        });
        Ok(module)
    }
}

#[derive(Clone, Debug, Default)]
pub struct MechanismStore {
    modules: BTreeMap<Hash32, MechanismModule>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ProceduralMechanismModule {
    id: Hash32,
    theory: CheckedTheory,
    support: MechanismSupportGeometry,
    source_proofs: BTreeSet<Hash32>,
    verified_cases: u64,
    actual_reuse_count: u32,
    reuse: BTreeMap<Hash32, ReuseEvidence>,
    status: ModuleStatus,
}

impl ProceduralMechanismModule {
    #[must_use]
    pub const fn id(&self) -> Hash32 {
        self.id
    }

    #[must_use]
    pub const fn theory(&self) -> &CheckedTheory {
        &self.theory
    }

    #[must_use]
    pub const fn support(&self) -> &MechanismSupportGeometry {
        &self.support
    }

    #[must_use]
    pub const fn status(&self) -> ModuleStatus {
        self.status
    }

    #[must_use]
    pub const fn source_proofs(&self) -> &BTreeSet<Hash32> {
        &self.source_proofs
    }

    #[must_use]
    pub const fn verified_cases(&self) -> u64 {
        self.verified_cases
    }

    #[must_use]
    pub const fn actual_reuse_count(&self) -> u32 {
        self.actual_reuse_count
    }

    #[must_use]
    pub fn reuse_metrics(&self) -> ReuseMetrics {
        reuse_metrics(self.reuse.values())
    }
}

#[derive(Clone, Debug, Default)]
pub struct ProceduralMechanismStore {
    modules: BTreeMap<Hash32, ProceduralMechanismModule>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProceduralStoreError {
    TheoryHashMismatch,
    ProofMismatch,
    EmptyVerification,
    Duplicate,
    Canonicalization,
}

impl ProceduralMechanismStore {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            modules: BTreeMap::new(),
        }
    }

    /// Admit a checked whole theory as a provisional proposal source. This
    /// records source verification but deliberately cannot claim downstream
    /// reuse or persistence.
    ///
    /// # Errors
    ///
    /// Rejects empty source evidence, empty verification, duplicate theories,
    /// and canonicalization failures.
    pub fn insert_provisional(
        &mut self,
        theory: CheckedTheory,
        support: MechanismSupportGeometry,
        source_proofs: BTreeSet<Hash32>,
        verified_cases: u64,
    ) -> Result<Hash32, ProceduralStoreError> {
        if source_proofs.is_empty() {
            return Err(ProceduralStoreError::ProofMismatch);
        }
        if verified_cases == 0 {
            return Err(ProceduralStoreError::EmptyVerification);
        }
        let bytes = theory
            .encode_canonical()
            .map_err(|_| ProceduralStoreError::Canonicalization)?;
        let id = Hash32::digest(&bytes);
        if self.modules.contains_key(&id) {
            return Err(ProceduralStoreError::Duplicate);
        }
        self.modules.insert(
            id,
            ProceduralMechanismModule {
                id,
                theory,
                support,
                source_proofs,
                verified_cases,
                actual_reuse_count: 0,
                reuse: BTreeMap::new(),
                status: ModuleStatus::Provisional,
            },
        );
        Ok(id)
    }

    /// Persist a complete checked theory only under a token bound to its
    /// canonical bytes and exact source-proof set.
    ///
    /// # Errors
    ///
    /// Rejects altered theories, proof mismatch, empty verification/reuse,
    /// duplicate semantic theories, and canonicalization failures.
    pub fn insert_verified(
        &mut self,
        theory: CheckedTheory,
        support: MechanismSupportGeometry,
        token: &VerifiedProceduralPromotion,
    ) -> Result<Hash32, ProceduralStoreError> {
        let bytes = theory
            .encode_canonical()
            .map_err(|_| ProceduralStoreError::Canonicalization)?;
        let id = Hash32::digest(&bytes);
        if id != token.theory_hash() {
            return Err(ProceduralStoreError::TheoryHashMismatch);
        }
        if token.source_proofs().is_empty() {
            return Err(ProceduralStoreError::ProofMismatch);
        }
        if token.verified_cases() == 0 || token.actual_reuse_count() == 0 {
            return Err(ProceduralStoreError::EmptyVerification);
        }
        if self.modules.contains_key(&id) {
            return Err(ProceduralStoreError::Duplicate);
        }
        self.modules.insert(
            id,
            ProceduralMechanismModule {
                id,
                theory,
                support,
                source_proofs: token.source_proofs().clone(),
                verified_cases: token.verified_cases(),
                actual_reuse_count: token.actual_reuse_count(),
                reuse: BTreeMap::new(),
                status: ModuleStatus::Persistent,
            },
        );
        Ok(id)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.modules.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.modules.is_empty()
    }

    pub fn modules(&self) -> impl Iterator<Item = &ProceduralMechanismModule> {
        self.modules.values()
    }

    pub fn reusable_modules(&self) -> impl Iterator<Item = &ProceduralMechanismModule> {
        self.modules.values().filter(|module| {
            matches!(
                module.status,
                ModuleStatus::Provisional | ModuleStatus::Persistent | ModuleStatus::Compiled
            )
        })
    }

    /// Record independently checked use of a whole theory, including the
    /// frozen no-memory counterfactual needed to distinguish reuse from luck.
    ///
    /// # Errors
    ///
    /// Rejects unknown or unavailable modules, unverified/duplicate evidence,
    /// and receipts without a non-empty counterfactual.
    pub fn record_verified_reuse(
        &mut self,
        id: Hash32,
        evidence: ReuseEvidence,
    ) -> Result<(), ReuseError> {
        let module = self.modules.get_mut(&id).ok_or(ReuseError::UnknownModule)?;
        if !matches!(
            module.status,
            ModuleStatus::Provisional | ModuleStatus::Persistent | ModuleStatus::Compiled
        ) {
            return Err(ReuseError::NotPersistent);
        }
        validate_reuse(&module.reuse, &evidence)?;
        module.reuse.insert(evidence.experience, evidence);
        module.actual_reuse_count = u32::try_from(module.reuse.len()).unwrap_or(u32::MAX);
        Ok(())
    }

    /// Import an opaque independent-verifier receipt into the procedural reuse
    /// ledger without allowing callers to rewrite its measured fields.
    ///
    /// # Errors
    ///
    /// Rejects theory identity mismatch and ordinary reuse-ledger failures.
    pub fn record_verifier_receipt(
        &mut self,
        id: Hash32,
        receipt: &VerifiedProceduralReuse,
    ) -> Result<(), ReuseError> {
        if receipt.theory_hash() != id {
            return Err(ReuseError::UnknownModule);
        }
        let metrics = receipt.metrics();
        self.record_verified_reuse(
            id,
            ReuseEvidence {
                experience: receipt.experience(),
                independently_verified: true,
                necessary_under_ablation: !metrics.blank_solved,
                candidate_executions_without: metrics.candidate_executions_without,
                candidate_executions_with: metrics.candidate_executions_with,
                examples_without: metrics.examples_without,
                examples_with: metrics.examples_with,
                proof_nodes_without: metrics.proof_nodes_without,
                proof_nodes_with: metrics.proof_nodes_with,
            },
        )
    }

    /// Promote an existing provisional theory only when a verifier token is
    /// bound to the same canonical theory, complete lineage, and measured use.
    #[must_use]
    pub fn apply_verified_promotion(
        &mut self,
        id: Hash32,
        promotion: &VerifiedProceduralPromotion,
    ) -> bool {
        let Some(module) = self.modules.get_mut(&id) else {
            return false;
        };
        if module.status != ModuleStatus::Provisional
            || id != promotion.theory_hash()
            || module.source_proofs != *promotion.source_proofs()
            || module.verified_cases == 0
            || promotion.verified_cases() < module.verified_cases
            || promotion.actual_reuse_count() == 0
            || promotion.actual_reuse_count() > module.actual_reuse_count
        {
            return false;
        }
        module.status = ModuleStatus::Persistent;
        true
    }
}

impl MechanismStore {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            modules: BTreeMap::new(),
        }
    }

    pub fn insert(&mut self, module: MechanismModule) -> Option<MechanismModule> {
        self.modules.insert(module.id, module)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.modules.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.modules.is_empty()
    }

    pub fn reusable_modules(&self) -> impl Iterator<Item = &MechanismModule> {
        self.modules.values().filter(|module| {
            matches!(
                module.status,
                ModuleStatus::Provisional | ModuleStatus::Persistent | ModuleStatus::Compiled
            )
        })
    }

    pub fn set_status(&mut self, id: Hash32, status: ModuleStatus) -> bool {
        if matches!(status, ModuleStatus::Persistent | ModuleStatus::Compiled) {
            return false;
        }
        let Some(module) = self.modules.get_mut(&id) else {
            return false;
        };
        module.status = status;
        true
    }

    /// Persist a provisional module only when an independent verifier token
    /// covers its complete source-proof lineage and measured reuse.
    #[must_use]
    pub fn apply_verified_promotion(&mut self, id: Hash32, promotion: &VerifiedPromotion) -> bool {
        let Some(module) = self.modules.get_mut(&id) else {
            return false;
        };
        let lineage = module.lineage.iter().copied().collect::<BTreeSet<_>>();
        if module.status != ModuleStatus::Provisional
            || lineage != *promotion.source_proofs()
            || promotion.verified_cases() == 0
            || promotion.actual_reuse_count() == 0
        {
            return false;
        }
        module.status = ModuleStatus::Persistent;
        true
    }

    /// Record one independently verified invocation and its frozen
    /// without-memory counterfactual. Negative transfer is retained in the
    /// ledger rather than silently discarded.
    ///
    /// # Errors
    ///
    /// Rejects unknown/nonpersistent modules, unverified claims, duplicates,
    /// and evidence without counterfactual work or examples.
    pub fn record_verified_reuse(
        &mut self,
        id: Hash32,
        evidence: ReuseEvidence,
    ) -> Result<(), ReuseError> {
        let module = self.modules.get_mut(&id).ok_or(ReuseError::UnknownModule)?;
        if !matches!(
            module.status,
            ModuleStatus::Persistent | ModuleStatus::Compiled
        ) {
            return Err(ReuseError::NotPersistent);
        }
        validate_reuse(&module.reuse, &evidence)?;
        module.reuse.insert(evidence.experience, evidence);
        Ok(())
    }

    fn proposals(&self, problem: &InventionProblem) -> Vec<ReboundProposal> {
        let mut proposals = Vec::new();
        let maximum = MAXIMUM_REBOUND_PROPOSALS;
        for module in self.reusable_modules() {
            if proposals.len() >= maximum {
                break;
            }
            if !type_shape_compatible(&module.definition.return_type, &problem.return_type)
                || module.definition.parameters.len() != problem.parameters.len()
            {
                continue;
            }
            let preferred = module
                .definition
                .parameters
                .iter()
                .map(|old| {
                    problem.parameters.iter().position(|new| {
                        old.name == new.name && type_shape_compatible(&old.ty, &new.ty)
                    })
                })
                .collect::<Option<Vec<_>>>();
            if let Some(mapping) = preferred {
                push_rebound(module, problem, &mapping, &mut proposals);
                continue;
            }
            let mut used = vec![false; problem.parameters.len()];
            let mut mapping = Vec::with_capacity(problem.parameters.len());
            bindings(
                module,
                problem,
                0,
                &mut used,
                &mut mapping,
                &mut proposals,
                maximum,
            );
        }
        proposals
    }

    fn compositions(&self, problem: &InventionProblem) -> Vec<ComposedProposal> {
        let modules = self.reusable_modules().collect::<Vec<_>>();
        let mut proposals = Vec::new();
        let maximum = MAXIMUM_REBOUND_PROPOSALS;
        for outer in &modules {
            if proposals.len() >= maximum {
                break;
            }
            if !type_shape_compatible(&outer.definition.return_type, &problem.return_type) {
                continue;
            }
            for inner in &modules {
                if proposals.len() >= maximum {
                    break;
                }
                if outer.id == inner.id {
                    continue;
                }
                let combined_arity = outer
                    .definition
                    .parameters
                    .len()
                    .saturating_add(inner.definition.parameters.len())
                    .saturating_sub(1);
                if combined_arity != problem.parameters.len() {
                    continue;
                }
                for substituted_parameter in 0..outer.definition.parameters.len() {
                    if proposals.len() >= maximum {
                        break;
                    }
                    if !type_shape_compatible(
                        &outer.definition.parameters[substituted_parameter].ty,
                        &inner.definition.return_type,
                    ) {
                        continue;
                    }
                    compose_bindings(
                        outer,
                        inner,
                        substituted_parameter,
                        problem,
                        &mut proposals,
                        maximum,
                    );
                }
            }
        }
        proposals
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SolveOrigin {
    MemoryProposal,
    MemoryComposition,
    FreshSearch,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompositionRecord {
    pub outer_module: Hash32,
    pub inner_module: Hash32,
    pub substituted_parameter: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MemorySolveMetrics {
    pub memory_proposals_evaluated: u64,
    pub composition_proposals_evaluated: u64,
    pub rejected_memory_proposals: u64,
    pub fresh_candidates_evaluated: u64,
    pub total_search_work: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MemorySolveOutcome {
    pub invention: InventionOutcome,
    pub origin: SolveOrigin,
    pub composition: Option<CompositionRecord>,
    pub metrics: MemorySolveMetrics,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct MemoryAssistedInventor;

impl MemoryAssistedInventor {
    /// Try deterministic rebound modules, verifying each against the current
    /// task. If none passes, run unrestricted fresh invention.
    ///
    /// # Errors
    ///
    /// Returns the fresh inventor's exact terminal error after memory proposals
    /// have failed; retrieval never vetoes fallback search.
    pub fn invent(
        &self,
        problem: &InventionProblem,
        store: &MechanismStore,
    ) -> Result<MemorySolveOutcome, InventionError> {
        let inventor = DefinitionInventor;
        let proposals = store.proposals(problem);
        let mut attempted = 0_u64;
        for proposal in proposals {
            attempted += 1;
            if let Ok(invention) = inventor.admit_definition(
                problem,
                proposal.definition,
                proposal.structural_cost,
                attempted,
            ) {
                return Ok(MemorySolveOutcome {
                    invention,
                    origin: SolveOrigin::MemoryProposal,
                    composition: None,
                    metrics: MemorySolveMetrics {
                        memory_proposals_evaluated: attempted,
                        composition_proposals_evaluated: 0,
                        rejected_memory_proposals: attempted - 1,
                        fresh_candidates_evaluated: 0,
                        total_search_work: attempted,
                    },
                });
            }
        }
        let direct_attempts = attempted;
        let compositions = store.compositions(problem);
        let mut composition_attempts = 0_u64;
        for proposal in compositions {
            composition_attempts += 1;
            attempted += 1;
            if let Ok(invention) = inventor.admit_definition(
                problem,
                proposal.definition,
                proposal.structural_cost,
                attempted,
            ) {
                return Ok(MemorySolveOutcome {
                    invention,
                    origin: SolveOrigin::MemoryComposition,
                    composition: Some(proposal.record),
                    metrics: MemorySolveMetrics {
                        memory_proposals_evaluated: direct_attempts,
                        composition_proposals_evaluated: composition_attempts,
                        rejected_memory_proposals: attempted - 1,
                        fresh_candidates_evaluated: 0,
                        total_search_work: attempted,
                    },
                });
            }
        }
        let invention = inventor.invent(problem)?;
        let fresh = invention.proposal.candidates_evaluated;
        Ok(MemorySolveOutcome {
            invention,
            origin: SolveOrigin::FreshSearch,
            composition: None,
            metrics: MemorySolveMetrics {
                memory_proposals_evaluated: direct_attempts,
                composition_proposals_evaluated: composition_attempts,
                rejected_memory_proposals: attempted,
                fresh_candidates_evaluated: fresh,
                total_search_work: attempted.saturating_add(fresh),
            },
        })
    }
}

struct ReboundProposal {
    definition: Definition,
    structural_cost: u16,
}

struct ComposedProposal {
    definition: Definition,
    structural_cost: u16,
    record: CompositionRecord,
}

fn compose_bindings(
    outer: &MechanismModule,
    inner: &MechanismModule,
    substituted_parameter: usize,
    problem: &InventionProblem,
    proposals: &mut Vec<ComposedProposal>,
    maximum: usize,
) {
    let inner_mappings = compatible_bindings(
        &inner.definition.parameters,
        problem,
        &BTreeSet::new(),
        maximum.saturating_sub(proposals.len()),
    );
    for inner_mapping in inner_mappings {
        if proposals.len() >= maximum {
            return;
        }
        let used = inner_mapping.iter().copied().collect::<BTreeSet<_>>();
        let remaining_outer = outer
            .definition
            .parameters
            .iter()
            .enumerate()
            .filter(|(index, _)| *index != substituted_parameter)
            .map(|(_, parameter)| parameter.clone())
            .collect::<Vec<_>>();
        for outer_mapping in compatible_bindings(
            &remaining_outer,
            problem,
            &used,
            maximum.saturating_sub(proposals.len()),
        ) {
            let inner_substitutions = inner
                .definition
                .parameters
                .iter()
                .zip(&inner_mapping)
                .map(|(parameter, current)| {
                    (
                        parameter.name.clone(),
                        Expr::Var(problem.parameters[*current].name.clone()),
                    )
                })
                .collect::<BTreeMap<_, _>>();
            let inner_body = substitute(
                &inner.definition.body,
                &inner_substitutions,
                &mut BTreeSet::new(),
            );
            let mut outer_substitutions = BTreeMap::new();
            outer_substitutions.insert(
                outer.definition.parameters[substituted_parameter]
                    .name
                    .clone(),
                inner_body,
            );
            for (parameter, current) in remaining_outer.iter().zip(&outer_mapping) {
                outer_substitutions.insert(
                    parameter.name.clone(),
                    Expr::Var(problem.parameters[*current].name.clone()),
                );
            }
            let Some(structural_cost) = outer.structural_cost.checked_add(inner.structural_cost)
            else {
                continue;
            };
            proposals.push(ComposedProposal {
                definition: Definition {
                    name: problem.definition_name.clone(),
                    parameters: problem.parameters.clone(),
                    return_type: problem.return_type.clone(),
                    body: substitute(
                        &outer.definition.body,
                        &outer_substitutions,
                        &mut BTreeSet::new(),
                    ),
                    recursion: None,
                    proof_annotations: Vec::new(),
                },
                structural_cost,
                record: CompositionRecord {
                    outer_module: outer.id,
                    inner_module: inner.id,
                    substituted_parameter,
                },
            });
        }
    }
}

fn compatible_bindings(
    parameters: &[cairn_rkl::Binder],
    problem: &InventionProblem,
    initially_used: &BTreeSet<usize>,
    maximum: usize,
) -> Vec<Vec<usize>> {
    fn visit(
        parameters: &[cairn_rkl::Binder],
        problem: &InventionProblem,
        index: usize,
        used: &mut BTreeSet<usize>,
        selected: &mut Vec<usize>,
        results: &mut Vec<Vec<usize>>,
        maximum: usize,
    ) {
        if results.len() >= maximum {
            return;
        }
        if index == parameters.len() {
            results.push(selected.clone());
            return;
        }
        for (current, candidate) in problem.parameters.iter().enumerate() {
            if used.contains(&current)
                || !type_shape_compatible(&parameters[index].ty, &candidate.ty)
            {
                continue;
            }
            used.insert(current);
            selected.push(current);
            visit(
                parameters,
                problem,
                index + 1,
                used,
                selected,
                results,
                maximum,
            );
            selected.pop();
            used.remove(&current);
            if results.len() >= maximum {
                return;
            }
        }
    }

    let mut results = Vec::new();
    visit(
        parameters,
        problem,
        0,
        &mut initially_used.clone(),
        &mut Vec::new(),
        &mut results,
        maximum,
    );
    results
}

fn bindings(
    module: &MechanismModule,
    problem: &InventionProblem,
    old_index: usize,
    used: &mut [bool],
    mapping: &mut Vec<usize>,
    proposals: &mut Vec<ReboundProposal>,
    maximum: usize,
) {
    if proposals.len() >= maximum {
        return;
    }
    if old_index == module.definition.parameters.len() {
        push_rebound(module, problem, mapping, proposals);
        return;
    }
    let old = &module.definition.parameters[old_index];
    for (new_index, new) in problem.parameters.iter().enumerate() {
        if used[new_index] || !type_shape_compatible(&old.ty, &new.ty) {
            continue;
        }
        used[new_index] = true;
        mapping.push(new_index);
        bindings(
            module,
            problem,
            old_index + 1,
            used,
            mapping,
            proposals,
            maximum,
        );
        mapping.pop();
        used[new_index] = false;
        if proposals.len() >= maximum {
            return;
        }
    }
}

fn push_rebound(
    module: &MechanismModule,
    problem: &InventionProblem,
    mapping: &[usize],
    proposals: &mut Vec<ReboundProposal>,
) {
    let substitutions = module
        .definition
        .parameters
        .iter()
        .zip(mapping)
        .map(|(old, new)| {
            (
                old.name.clone(),
                Expr::Var(problem.parameters[*new].name.clone()),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let definition = Definition {
        name: problem.definition_name.clone(),
        parameters: problem.parameters.clone(),
        return_type: problem.return_type.clone(),
        body: substitute(
            &module.definition.body,
            &substitutions,
            &mut BTreeSet::new(),
        ),
        recursion: None,
        proof_annotations: Vec::new(),
    };
    if let Expr::Var(parameter) = &definition.body {
        let contradicted = problem.demonstrations.iter().any(|example| {
            example
                .inputs
                .get(parameter)
                .is_none_or(|value| value != &example.output)
        });
        if contradicted {
            return;
        }
    }
    if proposals
        .last()
        .is_some_and(|proposal| proposal.definition == definition)
    {
        return;
    }
    proposals.push(ReboundProposal {
        definition,
        structural_cost: module.structural_cost,
    });
}

fn substitute(
    expression: &Expr,
    substitutions: &BTreeMap<VarName, Expr>,
    bound: &mut BTreeSet<VarName>,
) -> Expr {
    match expression {
        Expr::Bool(value) => Expr::Bool(*value),
        Expr::Int(value) => Expr::Int(*value),
        Expr::Atom(value) => Expr::Atom(*value),
        Expr::Coord2 { row, column } => Expr::Coord2 {
            row: *row,
            column: *column,
        },
        Expr::Var(name) => {
            if bound.contains(name) {
                Expr::Var(name.clone())
            } else {
                substitutions
                    .get(name)
                    .cloned()
                    .unwrap_or_else(|| Expr::Var(name.clone()))
            }
        }
        Expr::Tuple(values) => Expr::Tuple(
            values
                .iter()
                .map(|value| substitute(value, substitutions, bound))
                .collect(),
        ),
        Expr::Let {
            binder,
            value,
            body,
        } => {
            let value = substitute(value, substitutions, bound);
            bound.insert(binder.clone());
            let body = substitute(body, substitutions, bound);
            bound.remove(binder);
            Expr::Let {
                binder: binder.clone(),
                value: Box::new(value),
                body: Box::new(body),
            }
        }
        Expr::If {
            condition,
            then_branch,
            else_branch,
        } => Expr::If {
            condition: Box::new(substitute(condition, substitutions, bound)),
            then_branch: Box::new(substitute(then_branch, substitutions, bound)),
            else_branch: Box::new(substitute(else_branch, substitutions, bound)),
        },
        Expr::Unary { op, argument } => Expr::Unary {
            op: *op,
            argument: Box::new(substitute(argument, substitutions, bound)),
        },
        Expr::Binary { op, left, right } => Expr::Binary {
            op: *op,
            left: Box::new(substitute(left, substitutions, bound)),
            right: Box::new(substitute(right, substitutions, bound)),
        },
        Expr::EmptySet { element, maximum } => Expr::EmptySet {
            element: element.clone(),
            maximum: *maximum,
        },
        Expr::SingletonSet { value, maximum } => Expr::SingletonSet {
            value: Box::new(substitute(value, substitutions, bound)),
            maximum: *maximum,
        },
        Expr::Count(value) => Expr::Count(Box::new(substitute(value, substitutions, bound))),
        Expr::Call {
            function,
            arguments,
        } => Expr::Call {
            function: function.clone(),
            arguments: arguments
                .iter()
                .map(|argument| substitute(argument, substitutions, bound))
                .collect(),
        },
        Expr::BoundedFixpoint {
            binder,
            initial,
            step,
            maximum_steps,
            progress_measure,
        } => {
            let initial = substitute(initial, substitutions, bound);
            bound.insert(binder.clone());
            let step = substitute(step, substitutions, bound);
            let progress_measure = substitute(progress_measure, substitutions, bound);
            bound.remove(binder);
            Expr::BoundedFixpoint {
                binder: binder.clone(),
                initial: Box::new(initial),
                step: Box::new(step),
                maximum_steps: *maximum_steps,
                progress_measure: Box::new(progress_measure),
            }
        }
    }
}

fn equivalent_definition_shape(left: &Definition, right: &Definition) -> bool {
    if left.parameters.len() != right.parameters.len()
        || !type_shape_compatible(&left.return_type, &right.return_type)
        || left
            .parameters
            .iter()
            .zip(&right.parameters)
            .any(|(left, right)| !type_shape_compatible(&left.ty, &right.ty))
    {
        return false;
    }
    normalized_body(left) == normalized_body(right)
}

fn module_identity(
    definition: &Definition,
    lineage: &[Hash32],
    support: &SupportGeometry,
) -> Result<Hash32, PromotionError> {
    let mut identity = definition
        .encode_canonical()
        .map_err(|_| PromotionError::Canonicalization)?;
    identity.extend_from_slice(MEMORY_SEMANTICS_VERSION.as_bytes());
    for hash in lineage {
        identity.extend_from_slice(hash.as_bytes());
    }
    identity.extend_from_slice(&support.independent_tasks.to_be_bytes());
    identity.extend_from_slice(&support.held_out_rebindings.to_be_bytes());
    for dimension in &support.varied_dimensions {
        identity.push(variation_code(*dimension));
    }
    Ok(Hash32::digest(&identity))
}

fn generalize_types(types: &[&Type]) -> Result<Type, CompilationError> {
    let Some(first) = types.first() else {
        return Err(CompilationError::IncompatibleDefinitions);
    };
    match first {
        Type::Int(_) => {
            let ranges = types
                .iter()
                .map(|ty| match ty {
                    Type::Int(range) => Ok(*range),
                    _ => Err(CompilationError::IncompatibleDefinitions),
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(Type::Int(cairn_rkl::IntRange {
                lower: ranges
                    .iter()
                    .map(|range| range.lower)
                    .min()
                    .ok_or(CompilationError::IncompatibleDefinitions)?,
                upper: ranges
                    .iter()
                    .map(|range| range.upper)
                    .max()
                    .ok_or(CompilationError::IncompatibleDefinitions)?,
            }))
        }
        Type::CellSet { .. } => generalize_maximum(types, |maximum| Type::CellSet { maximum }),
        Type::EntitySet { .. } => generalize_maximum(types, |maximum| Type::EntitySet { maximum }),
        Type::List { .. } => {
            let elements = types
                .iter()
                .map(|ty| match ty {
                    Type::List { element, .. } => Ok(element.as_ref()),
                    _ => Err(CompilationError::IncompatibleDefinitions),
                })
                .collect::<Result<Vec<_>, _>>()?;
            let maximum = collection_maximum(types)?;
            Ok(Type::List {
                element: Box::new(generalize_types(&elements)?),
                maximum,
            })
        }
        Type::Tuple(fields) => {
            if types
                .iter()
                .any(|ty| !matches!(ty, Type::Tuple(other) if other.len() == fields.len()))
            {
                return Err(CompilationError::IncompatibleDefinitions);
            }
            let generalized = (0..fields.len())
                .map(|index| {
                    let column = types
                        .iter()
                        .map(|ty| match ty {
                            Type::Tuple(other) => &other[index],
                            _ => unreachable!("shape checked"),
                        })
                        .collect::<Vec<_>>();
                    generalize_types(&column)
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(Type::Tuple(generalized))
        }
        _ if types.iter().all(|ty| *ty == *first) => Ok((*first).clone()),
        _ => Err(CompilationError::IncompatibleDefinitions),
    }
}

fn generalize_maximum(
    types: &[&Type],
    constructor: impl FnOnce(u32) -> Type,
) -> Result<Type, CompilationError> {
    collection_maximum(types).map(constructor)
}

fn collection_maximum(types: &[&Type]) -> Result<u32, CompilationError> {
    types
        .iter()
        .map(|ty| {
            ty.collection_maximum()
                .ok_or(CompilationError::IncompatibleDefinitions)
        })
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .max()
        .ok_or(CompilationError::IncompatibleDefinitions)
}

fn normalized_body(definition: &Definition) -> Expr {
    let substitutions = definition
        .parameters
        .iter()
        .enumerate()
        .map(|(index, parameter)| {
            (
                parameter.name.clone(),
                Expr::Var(VarName(format!("parameter_{index}"))),
            )
        })
        .collect::<BTreeMap<_, _>>();
    substitute(&definition.body, &substitutions, &mut BTreeSet::new())
}

fn type_shape_compatible(left: &Type, right: &Type) -> bool {
    match (left, right) {
        (Type::Int(_), Type::Int(_))
        | (Type::CellSet { .. }, Type::CellSet { .. })
        | (Type::EntitySet { .. }, Type::EntitySet { .. }) => true,
        (Type::List { element: left, .. }, Type::List { element: right, .. }) => {
            type_shape_compatible(left, right)
        }
        (Type::Tuple(left), Type::Tuple(right)) => {
            left.len() == right.len()
                && left
                    .iter()
                    .zip(right)
                    .all(|(left, right)| type_shape_compatible(left, right))
        }
        _ => left == right,
    }
}

const fn variation_code(dimension: VariationDimension) -> u8 {
    match dimension {
        VariationDimension::ParameterNames => 0,
        VariationDimension::NumericRanges => 1,
        VariationDimension::CategoricalValues => 2,
        VariationDimension::Counts => 3,
        VariationDimension::Positions => 4,
        VariationDimension::Layouts => 5,
    }
}
