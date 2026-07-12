//! Independent promotion boundary combining replay provenance and learning gates.

mod layer;

pub use layer::*;

use cairn_consolidator::{
    CompiledInternalMechanism, ConstructedInternalComposition, EpisodeBindings,
    FrontierNodeSelector, GraphCompressionProposal, GraphPatternArgument, GraphPatternNode,
    InternalEpisodeBindings, InternalGraphCompressionProposal, InternalGraphPatternNode,
    InternalPatternInput, LoweredObservationFrontier, ObservationFrontierLimits,
    ObservationFrontierProvenance, ProofDatum, ProofFrontier, ProofGraph, ProofNode,
    ProofValueType, RelationalConstructionPolicy, RelationalValueSelector,
};
use cairn_core::{
    CanonicalEncode, CanonicalValue, CollectionProcedureProblem, EvidenceElementId,
    EvidenceValueType, Hash32, RelationalObservation,
};
use cairn_mechanism::{
    admit_persistent, AdmissionError, InventionEvidence, Mechanism, MechanismStatus,
};
pub use cairn_proof::{IndependentVerifier, ReplayVerifier, VerifyError};
use cairn_rkl::{
    CheckedTheory, ReferenceTheoryExecutor, RuntimeEnvironment, TheoryExecLimits,
    TheoryExecutionRequest, Type, TypeEnvironment, Value,
};
use cairn_world::{
    ActionId, LatentActionRole, LatentEpisode, LatentMechanismSchema, LatentSchemaBinding,
    LatentSchemaRule, LatentTransitionTheory, StateSpace, WorldState,
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedCompression {
    proposal: Hash32,
    source_episodes: BTreeSet<Hash32>,
    uncompressed_nodes: u64,
    compressed_nodes: u64,
    replayed_sources: u64,
}

impl VerifiedCompression {
    #[must_use]
    pub const fn proposal(&self) -> Hash32 {
        self.proposal
    }

    #[must_use]
    pub const fn source_episodes(&self) -> &BTreeSet<Hash32> {
        &self.source_episodes
    }

    #[must_use]
    pub const fn uncompressed_nodes(&self) -> u64 {
        self.uncompressed_nodes
    }

    #[must_use]
    pub const fn compressed_nodes(&self) -> u64 {
        self.compressed_nodes
    }

    #[must_use]
    pub const fn replayed_sources(&self) -> u64 {
        self.replayed_sources
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CompressionVerifyError {
    EmptyEvidence,
    DuplicateEpisode,
    SourceMismatch,
    InvalidOutput,
    InvalidTopology,
    MissingBinding,
    ReplayMismatch,
    CostMismatch,
    NoCompression,
    Capacity,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct CompressionVerifier;

impl CompressionVerifier {
    /// Independently reconstruct every source proof from a compression pattern
    /// and recalculate both representation costs.
    ///
    /// # Errors
    ///
    /// Rejects source/binding disagreement, malformed graphs, reconstruction
    /// mismatch, cost disagreement, or non-positive compression.
    pub fn verify(
        &self,
        proposal: &GraphCompressionProposal,
        sources: &[ProofGraph],
    ) -> Result<VerifiedCompression, CompressionVerifyError> {
        if sources.is_empty() {
            return Err(CompressionVerifyError::EmptyEvidence);
        }
        let source_ids = sources
            .iter()
            .map(|source| source.episode)
            .collect::<BTreeSet<_>>();
        if source_ids.len() != sources.len() {
            return Err(CompressionVerifyError::DuplicateEpisode);
        }
        let proposal_ids = proposal
            .source_episodes()
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        if source_ids != proposal_ids || proposal.bindings().len() != sources.len() {
            return Err(CompressionVerifyError::SourceMismatch);
        }
        let mut raw_units = 0_u64;
        for source in sources {
            let normalized = verifier_ancestral_subgraph(source)?;
            let bindings = proposal
                .bindings()
                .iter()
                .find(|bindings| bindings.episode == source.episode)
                .ok_or(CompressionVerifyError::MissingBinding)?;
            let reconstructed = verifier_instantiate(proposal.pattern(), bindings)?;
            if normalized != reconstructed
                || proposal.output()
                    != u32::try_from(reconstructed.len().saturating_sub(1))
                        .map_err(|_| CompressionVerifyError::Capacity)?
            {
                return Err(CompressionVerifyError::ReplayMismatch);
            }
            raw_units = raw_units
                .checked_add(graph_units(&normalized)?)
                .ok_or(CompressionVerifyError::Capacity)?;
        }
        let compressed_units = compressed_units(proposal.pattern(), proposal.bindings())?;
        if raw_units != proposal.uncompressed_nodes()
            || compressed_units != proposal.compressed_nodes()
        {
            return Err(CompressionVerifyError::CostMismatch);
        }
        if compressed_units >= raw_units {
            return Err(CompressionVerifyError::NoCompression);
        }
        Ok(VerifiedCompression {
            proposal: proposal.semantic_hash(),
            source_episodes: source_ids,
            uncompressed_nodes: raw_units,
            compressed_nodes: compressed_units,
            replayed_sources: u64::try_from(sources.len())
                .map_err(|_| CompressionVerifyError::Capacity)?,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedInternalCompression {
    proposal: Hash32,
    source_episodes: BTreeSet<Hash32>,
    uncompressed_nodes: u64,
    compressed_nodes: u64,
    replayed_sources: u64,
}

impl VerifiedInternalCompression {
    #[must_use]
    pub const fn proposal(&self) -> Hash32 {
        self.proposal
    }

    #[must_use]
    pub const fn source_episodes(&self) -> &BTreeSet<Hash32> {
        &self.source_episodes
    }

    #[must_use]
    pub const fn uncompressed_nodes(&self) -> u64 {
        self.uncompressed_nodes
    }

    #[must_use]
    pub const fn compressed_nodes(&self) -> u64 {
        self.compressed_nodes
    }

    #[must_use]
    pub const fn replayed_sources(&self) -> u64 {
        self.replayed_sources
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InternalCompressionVerifyError {
    EmptyEvidence,
    DuplicateEpisode,
    SourceMismatch,
    InvalidOutput,
    InvalidTopology,
    InvalidSelection,
    BoundaryMismatch,
    MissingBinding,
    ReplayMismatch,
    CostMismatch,
    IdentityMismatch,
    CompiledLineageMismatch,
    CompiledSchemaMismatch,
    DuplicateMechanism,
    OverlappingInvocation,
    IrrelevantInvocation,
    FrontierMismatch,
    ConstructedProofMismatch,
    InvalidJointResult,
    InvalidObservation,
    ObservationFrontierMismatch,
    RelationalPolicyMismatch,
    InsufficientReuseEvidence,
    NoLearningReduction,
    NoCompression,
    Canonicalization,
    Capacity,
}

/// Opaque evidence that a source-independent internal mechanism was derived
/// from an independently replayed compression proposal.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedInternalMechanism {
    mechanism: Hash32,
    source_proposal: Hash32,
    replayed_sources: u64,
}

impl VerifiedInternalMechanism {
    #[must_use]
    pub const fn mechanism(&self) -> Hash32 {
        self.mechanism
    }

    #[must_use]
    pub const fn source_proposal(&self) -> Hash32 {
        self.source_proposal
    }

    #[must_use]
    pub const fn replayed_sources(&self) -> u64 {
        self.replayed_sources
    }
}

/// Opaque evidence that one compiled mechanism invocation exactly expands to
/// the selected occurrence in a current host proof.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedInternalInvocation {
    mechanism: Hash32,
    episode: Hash32,
    selected_nodes: Vec<u32>,
}

impl VerifiedInternalInvocation {
    #[must_use]
    pub const fn mechanism(&self) -> Hash32 {
        self.mechanism
    }

    #[must_use]
    pub const fn episode(&self) -> Hash32 {
        self.episode
    }

    #[must_use]
    pub fn selected_nodes(&self) -> &[u32] {
        &self.selected_nodes
    }
}

/// Deterministic fixed-work comparison between fresh internal-mechanism
/// invention and reuse of an already compiled mechanism.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InternalReuseMetrics {
    pub candidate_windows_without: u64,
    pub candidate_windows_with: u64,
    pub examples_without: u32,
    pub examples_with: u32,
    pub proof_units_without: u64,
    pub proof_units_with: u64,
}

/// Opaque verifier receipt for one necessary invocation on a new experience.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedInternalReuse {
    mechanism: Hash32,
    source_proposal: Hash32,
    experience: Hash32,
    verified_cases: u64,
    metrics: InternalReuseMetrics,
}

/// Complete current-process evidence for one prerequisite mechanism in a
/// proposed composition. No serialized verifier authority is accepted.
#[derive(Clone, Copy)]
pub struct InternalCompositionEvidence<'a> {
    pub proposal: &'a InternalGraphCompressionProposal,
    pub mechanism: &'a CompiledInternalMechanism,
    pub compilation: &'a VerifiedInternalMechanism,
    pub sources: &'a [ProofGraph],
    pub binding: &'a InternalEpisodeBindings,
}

/// Complete current-process input for independent relational construction
/// verification. Expected truth remains confined to this verifier request.
#[derive(Clone, Copy)]
pub struct RelationalConstructionEvidence<'a> {
    pub observation: &'a RelationalObservation,
    pub lowering: &'a LoweredObservationFrontier,
    pub policy: &'a RelationalConstructionPolicy,
    pub expected: &'a ProofGraph,
    pub candidate: &'a ConstructedInternalComposition,
    pub mechanisms: &'a [InternalCompositionEvidence<'a>],
    pub policy_verification: &'a VerifiedRelationalConstructionPolicy,
    pub limits: ObservationFrontierLimits,
}

/// Inputs proving that a structural correspondence policy was derived from a
/// previously verified exact construction rather than supplied independently.
#[derive(Clone, Copy)]
pub struct RelationalPolicyInductionEvidence<'a> {
    pub observation: &'a RelationalObservation,
    pub lowering: &'a LoweredObservationFrontier,
    pub policy: &'a RelationalConstructionPolicy,
    pub candidate: &'a ConstructedInternalComposition,
    pub verified_construction: &'a VerifiedInternalConstruction,
    pub mechanisms: &'a [InternalCompositionEvidence<'a>],
    pub limits: ObservationFrontierLimits,
}

/// Opaque verifier evidence binding a value-free relational policy to the
/// exact training construction from which it was induced.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedRelationalConstructionPolicy {
    policy: Hash32,
    training: Hash32,
    certificate: Hash32,
}

impl VerifiedRelationalConstructionPolicy {
    #[must_use]
    pub const fn policy(&self) -> Hash32 {
        self.policy
    }

    #[must_use]
    pub const fn training(&self) -> Hash32 {
        self.training
    }

    #[must_use]
    pub const fn certificate(&self) -> Hash32 {
        self.certificate
    }
}

/// Opaque verifier result proving that distinct learned mechanisms jointly
/// contribute to one current proof and reduce relearning work.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedInternalComposition {
    experience: Hash32,
    invocations: Vec<VerifiedInternalInvocation>,
    metrics: InternalReuseMetrics,
    certificate: Hash32,
}

/// Independent result proving that learned mechanisms constructed an exact
/// held-out proof from an evidence-only frontier.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedInternalConstruction {
    frontier: Hash32,
    composition: VerifiedInternalComposition,
    certificate: Hash32,
}

/// Opaque evidence that a relational observation was independently lowered to
/// the exact proof frontier and provenance claimed by a producer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedObservationFrontier {
    observation: Hash32,
    frontier: Hash32,
    certificate: Hash32,
}

impl VerifiedObservationFrontier {
    #[must_use]
    pub const fn observation(&self) -> Hash32 {
        self.observation
    }

    #[must_use]
    pub const fn frontier(&self) -> Hash32 {
        self.frontier
    }

    #[must_use]
    pub const fn certificate(&self) -> Hash32 {
        self.certificate
    }
}

impl VerifiedInternalConstruction {
    #[must_use]
    pub const fn frontier(&self) -> Hash32 {
        self.frontier
    }

    #[must_use]
    pub const fn composition(&self) -> &VerifiedInternalComposition {
        &self.composition
    }

    #[must_use]
    pub const fn certificate(&self) -> Hash32 {
        self.certificate
    }
}

impl VerifiedInternalComposition {
    #[must_use]
    pub const fn experience(&self) -> Hash32 {
        self.experience
    }

    #[must_use]
    pub fn invocations(&self) -> &[VerifiedInternalInvocation] {
        &self.invocations
    }

    #[must_use]
    pub const fn metrics(&self) -> InternalReuseMetrics {
        self.metrics
    }

    #[must_use]
    pub const fn certificate(&self) -> Hash32 {
        self.certificate
    }
}

impl VerifiedInternalReuse {
    #[must_use]
    pub const fn mechanism(&self) -> Hash32 {
        self.mechanism
    }

    #[must_use]
    pub const fn source_proposal(&self) -> Hash32 {
        self.source_proposal
    }

    #[must_use]
    pub const fn experience(&self) -> Hash32 {
        self.experience
    }

    #[must_use]
    pub const fn verified_cases(&self) -> u64 {
        self.verified_cases
    }

    #[must_use]
    pub const fn metrics(&self) -> InternalReuseMetrics {
        self.metrics
    }
}

/// Consolidator-issued certificate aggregating multiple independent runtime
/// reuse receipts for the same compiled internal mechanism.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedInternalPromotion {
    mechanism: Hash32,
    source_proposals: BTreeSet<Hash32>,
    experiences: BTreeSet<Hash32>,
    verified_cases: u64,
}

impl VerifiedInternalPromotion {
    #[must_use]
    pub const fn mechanism(&self) -> Hash32 {
        self.mechanism
    }

    #[must_use]
    pub const fn source_proposals(&self) -> &BTreeSet<Hash32> {
        &self.source_proposals
    }

    #[must_use]
    pub const fn experiences(&self) -> &BTreeSet<Hash32> {
        &self.experiences
    }

    #[must_use]
    pub const fn verified_cases(&self) -> u64 {
        self.verified_cases
    }
}

/// Independent compiler and invocation trust boundary for learned internal
/// proof mechanisms.
#[derive(Clone, Copy, Debug, Default)]
pub struct InternalMechanismVerifier;

impl InternalMechanismVerifier {
    /// Independently reconstruct one relational observation frontier. Producer
    /// lowering code and learned construction are outside this reference path.
    ///
    /// # Errors
    ///
    /// Rejects invalid observation evidence, capacity excess, changed nodes or
    /// provenance, or a mismatched canonical commitment.
    pub fn verify_observation_frontier(
        &self,
        observation: &RelationalObservation,
        candidate: &LoweredObservationFrontier,
        limits: ObservationFrontierLimits,
    ) -> Result<VerifiedObservationFrontier, InternalCompressionVerifyError> {
        observation
            .verify(limits.observation)
            .map_err(|_| InternalCompressionVerifyError::InvalidObservation)?;
        let (frontier, provenance) = verifier_lower_observation(observation, limits)?;
        let frontier_hash = verifier_observation_frontier_hash(&frontier, &provenance)?;
        if candidate.frontier != frontier
            || candidate.provenance != provenance
            || candidate.commitment != frontier_hash
        {
            return Err(InternalCompressionVerifyError::ObservationFrontierMismatch);
        }
        let certificate = CanonicalValue::Array(vec![
            CanonicalValue::Text("cairn.verified-observation-frontier.v1".to_owned()),
            CanonicalValue::Bytes(observation.observation.id.0.as_bytes().to_vec()),
            CanonicalValue::Bytes(frontier_hash.as_bytes().to_vec()),
        ])
        .encode_canonical()
        .map(|bytes| Hash32::digest(&bytes))
        .map_err(|_| InternalCompressionVerifyError::Canonicalization)?;
        Ok(VerifiedObservationFrontier {
            observation: observation.observation.id.0,
            frontier: frontier_hash,
            certificate,
        })
    }

    /// Verify that compilation removed evidence identity without changing the
    /// typed executable schema, then replay all mining sources independently.
    ///
    /// # Errors
    ///
    /// Rejects invalid source compression, changed lineage/schema/identity,
    /// inconsistent parameter types, or any failed source occurrence replay.
    pub fn verify_compilation(
        &self,
        proposal: &InternalGraphCompressionProposal,
        mechanism: &CompiledInternalMechanism,
        sources: &[ProofGraph],
    ) -> Result<VerifiedInternalMechanism, InternalCompressionVerifyError> {
        let compression = InternalCompressionVerifier.verify(proposal, sources)?;
        if mechanism.source_proposal() != proposal.semantic_hash() {
            return Err(InternalCompressionVerifyError::CompiledLineageMismatch);
        }
        let parameter_types = proposal
            .bindings()
            .first()
            .ok_or(InternalCompressionVerifyError::MissingBinding)?
            .values
            .iter()
            .map(|value| value.ty)
            .collect::<Vec<_>>();
        if proposal.bindings().iter().any(|binding| {
            binding.values.len() != parameter_types.len()
                || binding
                    .values
                    .iter()
                    .zip(&parameter_types)
                    .any(|(value, ty)| value.ty != *ty)
        }) || mechanism.pattern() != proposal.pattern()
            || mechanism.boundary_types() != proposal.boundary_types()
            || mechanism.parameter_types() != parameter_types
            || mechanism.output() != proposal.output()
        {
            return Err(InternalCompressionVerifyError::CompiledSchemaMismatch);
        }
        let identity = verifier_internal_mechanism_identity(
            mechanism.pattern(),
            mechanism.boundary_types(),
            mechanism.parameter_types(),
            mechanism.output(),
        )?;
        if identity != mechanism.semantic_hash() {
            return Err(InternalCompressionVerifyError::IdentityMismatch);
        }
        for source in sources {
            let binding = proposal
                .bindings()
                .iter()
                .find(|binding| binding.episode == source.episode)
                .ok_or(InternalCompressionVerifyError::MissingBinding)?;
            self.verify_invocation(mechanism, source, binding)?;
        }
        Ok(VerifiedInternalMechanism {
            mechanism: identity,
            source_proposal: compression.proposal(),
            replayed_sources: compression.replayed_sources(),
        })
    }

    /// Independently verify one current-host binding and exact pattern
    /// expansion without invoking producer matching or expansion code.
    ///
    /// # Errors
    ///
    /// Rejects changed mechanism identity, host, selection, boundary, type,
    /// fixed value, or shared parameter binding.
    pub fn verify_invocation(
        &self,
        mechanism: &CompiledInternalMechanism,
        host: &ProofGraph,
        binding: &InternalEpisodeBindings,
    ) -> Result<VerifiedInternalInvocation, InternalCompressionVerifyError> {
        verifier_validate_graph(host)?;
        if binding.episode != host.episode {
            return Err(InternalCompressionVerifyError::SourceMismatch);
        }
        let identity = verifier_internal_mechanism_identity(
            mechanism.pattern(),
            mechanism.boundary_types(),
            mechanism.parameter_types(),
            mechanism.output(),
        )?;
        if identity != mechanism.semantic_hash() {
            return Err(InternalCompressionVerifyError::IdentityMismatch);
        }
        if binding.values.len() != mechanism.parameter_types().len()
            || binding
                .values
                .iter()
                .zip(mechanism.parameter_types())
                .any(|(value, ty)| value.ty != *ty)
        {
            return Err(InternalCompressionVerifyError::MissingBinding);
        }
        let occurrence = verifier_internal_occurrence(host, binding, mechanism.pattern())?;
        if occurrence.boundary_types != mechanism.boundary_types() {
            return Err(InternalCompressionVerifyError::BoundaryMismatch);
        }
        let expanded = verifier_instantiate_internal(mechanism.pattern(), binding)?;
        if occurrence.nodes != expanded {
            return Err(InternalCompressionVerifyError::ReplayMismatch);
        }
        Ok(VerifiedInternalInvocation {
            mechanism: identity,
            episode: host.episode,
            selected_nodes: binding.selected_nodes.clone(),
        })
    }

    /// Derive a fixed-work reuse receipt by independently replaying the source
    /// compilation and current invocation, then counting the exact windows and
    /// evidence required by fresh mining versus compiled matching.
    ///
    /// # Errors
    ///
    /// Rejects changed compilation lineage, repeated source/held-out episodes,
    /// failed invocation replay, or absent candidate, sample,
    /// and proof-size reductions.
    #[allow(clippy::too_many_arguments)] // Keep each independently checked evidence boundary explicit.
    pub fn verify_reuse(
        &self,
        proposal: &InternalGraphCompressionProposal,
        mechanism: &CompiledInternalMechanism,
        compilation: &VerifiedInternalMechanism,
        sources: &[ProofGraph],
        held_out: &ProofGraph,
        binding: &InternalEpisodeBindings,
        maximum_match_windows: usize,
    ) -> Result<VerifiedInternalReuse, InternalCompressionVerifyError> {
        let replayed = self.verify_compilation(proposal, mechanism, sources)?;
        if &replayed != compilation
            || compilation.mechanism != mechanism.semantic_hash()
            || compilation.source_proposal != proposal.semantic_hash()
            || sources
                .iter()
                .any(|source| source.episode == held_out.episode)
            || sources.len() < 2
            || maximum_match_windows == 0
        {
            return Err(InternalCompressionVerifyError::CompiledLineageMismatch);
        }
        self.verify_invocation(mechanism, held_out, binding)?;
        let width = mechanism.pattern().len();
        let candidate_windows_without = verifier_fresh_window_work(sources, width)?;
        if width > held_out.nodes.len() {
            return Err(InternalCompressionVerifyError::ReplayMismatch);
        }
        let candidate_windows_with = u64::try_from(held_out.nodes.len() - width + 1)
            .map_err(|_| InternalCompressionVerifyError::Capacity)?;
        if candidate_windows_with
            > u64::try_from(maximum_match_windows)
                .map_err(|_| InternalCompressionVerifyError::Capacity)?
        {
            return Err(InternalCompressionVerifyError::Capacity);
        }
        let examples_without =
            u32::try_from(sources.len()).map_err(|_| InternalCompressionVerifyError::Capacity)?;
        let proof_units_with = 1_u64
            .checked_add(
                u64::try_from(mechanism.parameter_types().len())
                    .map_err(|_| InternalCompressionVerifyError::Capacity)?,
            )
            .and_then(|value| {
                value.checked_add(u64::try_from(mechanism.boundary_types().len()).ok()?)
            })
            .ok_or(InternalCompressionVerifyError::Capacity)?;
        let metrics = InternalReuseMetrics {
            candidate_windows_without,
            candidate_windows_with,
            examples_without,
            examples_with: 1,
            proof_units_without: proposal.uncompressed_nodes(),
            proof_units_with,
        };
        if metrics.candidate_windows_with >= metrics.candidate_windows_without
            || metrics.examples_with >= metrics.examples_without
            || metrics.proof_units_with >= metrics.proof_units_without
        {
            return Err(InternalCompressionVerifyError::NoLearningReduction);
        }
        Ok(VerifiedInternalReuse {
            mechanism: mechanism.semantic_hash(),
            source_proposal: proposal.semantic_hash(),
            experience: held_out.episode,
            verified_cases: compilation
                .replayed_sources
                .checked_add(1)
                .ok_or(InternalCompressionVerifyError::Capacity)?,
            metrics,
        })
    }

    /// Independently replay every learned invocation, require pairwise-disjoint
    /// occurrences whose outputs are ancestors of the host result, and derive
    /// aggregate memory-off/on learning work.
    ///
    /// # Errors
    ///
    /// Rejects fewer than two or duplicate mechanisms, overlap, irrelevant
    /// invocations, changed compilation evidence, replay mismatch, capacity,
    /// and absent candidate/sample/proof reduction.
    pub fn verify_composition(
        &self,
        host: &ProofGraph,
        evidence: &[InternalCompositionEvidence<'_>],
        maximum_match_windows: usize,
    ) -> Result<VerifiedInternalComposition, InternalCompressionVerifyError> {
        verifier_validate_graph(host)?;
        if evidence.len() < 2 || maximum_match_windows == 0 {
            return Err(InternalCompressionVerifyError::EmptyEvidence);
        }
        let mut ordered = evidence.iter().collect::<Vec<_>>();
        ordered.sort_unstable_by_key(|item| item.mechanism.semantic_hash());
        if ordered
            .windows(2)
            .any(|pair| pair[0].mechanism.semantic_hash() == pair[1].mechanism.semantic_hash())
        {
            return Err(InternalCompressionVerifyError::DuplicateMechanism);
        }
        let ancestors = verifier_proof_ancestors(host)?;
        let mut selected = BTreeSet::new();
        let mut invocations = Vec::with_capacity(ordered.len());
        let mut source_proposals = Vec::with_capacity(ordered.len());
        let mut metrics = InternalReuseMetrics {
            candidate_windows_without: 0,
            candidate_windows_with: 0,
            examples_without: 0,
            examples_with: 1,
            proof_units_without: 0,
            proof_units_with: 0,
        };
        for item in ordered {
            let output = usize::try_from(item.mechanism.output())
                .ok()
                .and_then(|index| item.binding.selected_nodes.get(index))
                .ok_or(InternalCompressionVerifyError::ReplayMismatch)?;
            if !ancestors.contains(output) {
                return Err(InternalCompressionVerifyError::IrrelevantInvocation);
            }
            if item
                .binding
                .selected_nodes
                .iter()
                .any(|node| !selected.insert(*node))
            {
                return Err(InternalCompressionVerifyError::OverlappingInvocation);
            }
            invocations.push(self.verify_invocation(item.mechanism, host, item.binding)?);
            source_proposals.push(item.proposal.semantic_hash());
            let receipt = self.verify_reuse(
                item.proposal,
                item.mechanism,
                item.compilation,
                item.sources,
                host,
                item.binding,
                maximum_match_windows,
            )?;
            let measured = receipt.metrics();
            metrics.candidate_windows_without = metrics
                .candidate_windows_without
                .checked_add(measured.candidate_windows_without)
                .ok_or(InternalCompressionVerifyError::Capacity)?;
            metrics.candidate_windows_with = metrics
                .candidate_windows_with
                .checked_add(measured.candidate_windows_with)
                .ok_or(InternalCompressionVerifyError::Capacity)?;
            metrics.examples_without = metrics
                .examples_without
                .checked_add(measured.examples_without)
                .ok_or(InternalCompressionVerifyError::Capacity)?;
            metrics.proof_units_without = metrics
                .proof_units_without
                .checked_add(measured.proof_units_without)
                .ok_or(InternalCompressionVerifyError::Capacity)?;
            metrics.proof_units_with = metrics
                .proof_units_with
                .checked_add(measured.proof_units_with)
                .ok_or(InternalCompressionVerifyError::Capacity)?;
        }
        if metrics.candidate_windows_with >= metrics.candidate_windows_without
            || metrics.examples_with >= metrics.examples_without
            || metrics.proof_units_with >= metrics.proof_units_without
        {
            return Err(InternalCompressionVerifyError::NoLearningReduction);
        }
        let certificate = verifier_internal_composition_certificate(
            host.episode,
            &source_proposals,
            &invocations,
            metrics,
        )?;
        Ok(VerifiedInternalComposition {
            experience: host.episode,
            invocations,
            metrics,
            certificate,
        })
    }

    /// Independently require that one runtime candidate exactly constructs a
    /// held-out proof from the supplied evidence prefix and verified learned
    /// invocations. The expected proof is verifier-only input.
    ///
    /// # Errors
    ///
    /// Rejects malformed or changed frontiers, a candidate that differs from
    /// held-out truth, invocation disagreement, invalid joint-result structure,
    /// or any underlying composition/reuse verification failure.
    pub fn verify_constructed_composition(
        &self,
        frontier: &ProofFrontier,
        expected: &ProofGraph,
        candidate: &ConstructedInternalComposition,
        evidence: &[InternalCompositionEvidence<'_>],
        maximum_match_windows: usize,
    ) -> Result<VerifiedInternalConstruction, InternalCompressionVerifyError> {
        verifier_validate_frontier(frontier)?;
        verifier_validate_graph(expected)?;
        verifier_validate_graph(&candidate.graph)?;
        if expected != &candidate.graph
            || candidate.graph.episode != frontier.episode
            || expected.episode != frontier.episode
        {
            return Err(InternalCompressionVerifyError::ConstructedProofMismatch);
        }
        if candidate.graph.nodes.get(..frontier.nodes.len()) != Some(frontier.nodes.as_slice()) {
            return Err(InternalCompressionVerifyError::FrontierMismatch);
        }
        let mut candidate_invocations = candidate.invocations.iter().collect::<Vec<_>>();
        candidate_invocations.sort_unstable_by_key(|invocation| invocation.mechanism);
        let mut ordered_evidence = evidence.iter().collect::<Vec<_>>();
        ordered_evidence.sort_unstable_by_key(|item| item.mechanism.semantic_hash());
        if candidate_invocations.len() != ordered_evidence.len()
            || candidate_invocations
                .iter()
                .zip(&ordered_evidence)
                .any(|(invocation, item)| {
                    invocation.mechanism != item.mechanism.semantic_hash()
                        || &invocation.binding != item.binding
                })
        {
            return Err(InternalCompressionVerifyError::CompiledLineageMismatch);
        }
        let joint = candidate
            .graph
            .nodes
            .last()
            .ok_or(InternalCompressionVerifyError::InvalidJointResult)?;
        let expected_inputs = ordered_evidence
            .iter()
            .map(|item| {
                item.binding
                    .selected_nodes
                    .get(
                        usize::try_from(item.mechanism.output())
                            .map_err(|_| InternalCompressionVerifyError::Capacity)?,
                    )
                    .copied()
                    .ok_or(InternalCompressionVerifyError::InvalidJointResult)
            })
            .collect::<Result<Vec<_>, _>>()?;
        if joint.operation != "join-learned-results"
            || joint.inputs != expected_inputs
            || candidate.graph.output
                != u32::try_from(candidate.graph.nodes.len() - 1)
                    .map_err(|_| InternalCompressionVerifyError::Capacity)?
        {
            return Err(InternalCompressionVerifyError::InvalidJointResult);
        }
        let composition =
            self.verify_composition(&candidate.graph, evidence, maximum_match_windows)?;
        let frontier_hash = verifier_frontier_hash(frontier)?;
        let certificate = CanonicalValue::Array(vec![
            CanonicalValue::Text("cairn.verified-internal-construction.v1".to_owned()),
            CanonicalValue::Bytes(frontier_hash.as_bytes().to_vec()),
            CanonicalValue::Bytes(composition.certificate().as_bytes().to_vec()),
        ])
        .encode_canonical()
        .map(|bytes| Hash32::digest(&bytes))
        .map_err(|_| InternalCompressionVerifyError::Canonicalization)?;
        Ok(VerifiedInternalConstruction {
            frontier: frontier_hash,
            composition,
            certificate,
        })
    }

    /// Independently reconstruct a value-free relational policy and bind it to
    /// the opaque exact-construction receipt that established its training case.
    ///
    /// # Errors
    ///
    /// Rejects changed observation/frontier lineage, mechanism or invocation
    /// disagreement, ambiguous correspondence, or any selector differing from
    /// independent structural reconstruction.
    pub fn verify_relational_policy_induction(
        &self,
        request: RelationalPolicyInductionEvidence<'_>,
    ) -> Result<VerifiedRelationalConstructionPolicy, InternalCompressionVerifyError> {
        let RelationalPolicyInductionEvidence {
            observation,
            lowering,
            policy,
            candidate,
            verified_construction,
            mechanisms,
            limits,
        } = request;
        let lowering_receipt = self.verify_observation_frontier(observation, lowering, limits)?;
        let frontier_hash = verifier_frontier_hash(&lowering.frontier)?;
        if verified_construction.frontier() != frontier_hash
            || verified_construction.composition().experience() != candidate.graph.episode
            || candidate.graph.episode != lowering.frontier.episode
            || candidate.graph.nodes.get(..lowering.frontier.nodes.len())
                != Some(lowering.frontier.nodes.as_slice())
        {
            return Err(InternalCompressionVerifyError::RelationalPolicyMismatch);
        }
        let mut ordered_evidence = mechanisms.iter().collect::<Vec<_>>();
        ordered_evidence.sort_unstable_by_key(|item| item.mechanism.semantic_hash());
        let mut invocations = candidate.invocations.iter().collect::<Vec<_>>();
        invocations.sort_unstable_by_key(|invocation| invocation.mechanism);
        let mut verified_invocations = verified_construction
            .composition()
            .invocations()
            .iter()
            .collect::<Vec<_>>();
        verified_invocations.sort_unstable_by_key(|invocation| invocation.mechanism());
        if ordered_evidence.len() != invocations.len()
            || invocations.len() != verified_invocations.len()
            || invocations
                .iter()
                .zip(&ordered_evidence)
                .zip(&verified_invocations)
                .any(|((invocation, item), verified)| {
                    invocation.mechanism != item.mechanism.semantic_hash()
                        || &invocation.binding != item.binding
                        || verified.mechanism() != invocation.mechanism
                        || verified.selected_nodes() != invocation.binding.selected_nodes
                })
        {
            return Err(InternalCompressionVerifyError::CompiledLineageMismatch);
        }
        let fingerprints = verifier_relational_fingerprints(observation)?;
        let mut boundary_selectors = Vec::with_capacity(invocations.len());
        let mut value_selectors = Vec::with_capacity(invocations.len());
        for invocation in invocations {
            let boundaries = invocation
                .binding
                .boundary_nodes
                .iter()
                .map(|node| {
                    let selector = verifier_frontier_node_selector(&lowering.frontier, *node)?;
                    verifier_select_frontier_node(&lowering.frontier, &selector)?;
                    Ok(selector)
                })
                .collect::<Result<Vec<_>, InternalCompressionVerifyError>>()?;
            let values = invocation
                .binding
                .values
                .iter()
                .map(|value| verifier_relational_value_selector(observation, &fingerprints, *value))
                .collect::<Result<Vec<_>, _>>()?;
            boundary_selectors.push(boundaries);
            value_selectors.push(values);
        }
        let reconstructed = RelationalConstructionPolicy {
            mechanisms: ordered_evidence
                .iter()
                .map(|item| item.mechanism.semantic_hash())
                .collect(),
            boundary_selectors,
            value_selectors,
        };
        if &reconstructed != policy {
            return Err(InternalCompressionVerifyError::RelationalPolicyMismatch);
        }
        let policy_hash = verifier_relational_policy_hash(policy)?;
        let certificate = CanonicalValue::Array(vec![
            CanonicalValue::Text("cairn.verified-relational-policy.v1".to_owned()),
            CanonicalValue::Bytes(lowering_receipt.certificate().as_bytes().to_vec()),
            CanonicalValue::Bytes(verified_construction.certificate().as_bytes().to_vec()),
            CanonicalValue::Bytes(policy_hash.as_bytes().to_vec()),
        ])
        .encode_canonical()
        .map(|bytes| Hash32::digest(&bytes))
        .map_err(|_| InternalCompressionVerifyError::Canonicalization)?;
        Ok(VerifiedRelationalConstructionPolicy {
            policy: policy_hash,
            training: verified_construction.certificate(),
            certificate,
        })
    }

    /// Independently replay a relational correspondence policy and measure its
    /// direct one-candidate construction path rather than obsolete sliding-host
    /// matching work.
    ///
    /// # Errors
    ///
    /// Rejects observation/lowering mismatch, a policy containing changed or
    /// ambiguous selectors, non-unit construction work, incorrect held-out
    /// proof, invalid mechanism lineage/invocation, or absent learning reduction.
    #[allow(clippy::too_many_lines)] // One explicit independent trust-boundary replay.
    pub fn verify_relational_constructed_composition(
        &self,
        request: RelationalConstructionEvidence<'_>,
    ) -> Result<VerifiedInternalConstruction, InternalCompressionVerifyError> {
        let RelationalConstructionEvidence {
            observation,
            lowering,
            policy,
            expected,
            candidate,
            mechanisms: evidence,
            policy_verification,
            limits,
        } = request;
        let verified_frontier = self.verify_observation_frontier(observation, lowering, limits)?;
        verifier_validate_graph(expected)?;
        verifier_validate_graph(&candidate.graph)?;
        if expected != &candidate.graph
            || candidate.graph.episode != lowering.frontier.episode
            || expected.episode != lowering.frontier.episode
            || candidate.graph.nodes.get(..lowering.frontier.nodes.len())
                != Some(lowering.frontier.nodes.as_slice())
        {
            return Err(InternalCompressionVerifyError::ConstructedProofMismatch);
        }
        if candidate.candidates_considered != 1 || evidence.len() < 2 {
            return Err(InternalCompressionVerifyError::RelationalPolicyMismatch);
        }
        let mut ordered_evidence = evidence.iter().collect::<Vec<_>>();
        ordered_evidence.sort_unstable_by_key(|item| item.mechanism.semantic_hash());
        let mechanism_ids = ordered_evidence
            .iter()
            .map(|item| item.mechanism.semantic_hash())
            .collect::<Vec<_>>();
        if policy.mechanisms != mechanism_ids
            || policy.boundary_selectors.len() != ordered_evidence.len()
            || policy.value_selectors.len() != ordered_evidence.len()
            || mechanism_ids.windows(2).any(|pair| pair[0] == pair[1])
        {
            return Err(InternalCompressionVerifyError::RelationalPolicyMismatch);
        }
        let mut candidate_invocations = candidate.invocations.iter().collect::<Vec<_>>();
        candidate_invocations.sort_unstable_by_key(|invocation| invocation.mechanism);
        if candidate_invocations.len() != ordered_evidence.len()
            || candidate_invocations
                .iter()
                .zip(&ordered_evidence)
                .any(|(invocation, item)| {
                    invocation.mechanism != item.mechanism.semantic_hash()
                        || &invocation.binding != item.binding
                })
        {
            return Err(InternalCompressionVerifyError::CompiledLineageMismatch);
        }
        let fingerprints = verifier_relational_fingerprints(observation)?;
        let policy_hash = verifier_relational_policy_hash(policy)?;
        if policy_verification.policy() != policy_hash {
            return Err(InternalCompressionVerifyError::RelationalPolicyMismatch);
        }
        for (index, (invocation, item)) in candidate_invocations
            .iter()
            .zip(&ordered_evidence)
            .enumerate()
        {
            if policy.boundary_selectors[index].len() != item.mechanism.boundary_types().len()
                || policy.value_selectors[index].len() != item.mechanism.parameter_types().len()
            {
                return Err(InternalCompressionVerifyError::RelationalPolicyMismatch);
            }
            let boundaries = policy.boundary_selectors[index]
                .iter()
                .map(|selector| verifier_select_frontier_node(&lowering.frontier, selector))
                .collect::<Result<Vec<_>, _>>()?;
            let values = policy.value_selectors[index]
                .iter()
                .map(|selector| {
                    verifier_select_relational_value(observation, &fingerprints, *selector)
                })
                .collect::<Result<Vec<_>, _>>()?;
            if invocation.binding.boundary_nodes != boundaries
                || invocation.binding.values != values
                || invocation.binding.episode != lowering.frontier.episode
            {
                return Err(InternalCompressionVerifyError::RelationalPolicyMismatch);
            }
        }

        let ancestors = verifier_proof_ancestors(&candidate.graph)?;
        let mut selected = BTreeSet::new();
        let mut invocations = Vec::with_capacity(ordered_evidence.len());
        let mut source_proposals = Vec::with_capacity(ordered_evidence.len());
        let mut metrics = InternalReuseMetrics {
            candidate_windows_without: 0,
            candidate_windows_with: 1,
            examples_without: 0,
            examples_with: 1,
            proof_units_without: 0,
            proof_units_with: 0,
        };
        for item in &ordered_evidence {
            let replayed = self.verify_compilation(item.proposal, item.mechanism, item.sources)?;
            if &replayed != item.compilation
                || item.sources.len() < 2
                || item
                    .sources
                    .iter()
                    .any(|source| source.episode == candidate.graph.episode)
            {
                return Err(InternalCompressionVerifyError::CompiledLineageMismatch);
            }
            let output = item
                .binding
                .selected_nodes
                .get(
                    usize::try_from(item.mechanism.output())
                        .map_err(|_| InternalCompressionVerifyError::Capacity)?,
                )
                .ok_or(InternalCompressionVerifyError::ReplayMismatch)?;
            if !ancestors.contains(output) {
                return Err(InternalCompressionVerifyError::IrrelevantInvocation);
            }
            if item
                .binding
                .selected_nodes
                .iter()
                .any(|node| !selected.insert(*node))
            {
                return Err(InternalCompressionVerifyError::OverlappingInvocation);
            }
            invocations.push(self.verify_invocation(
                item.mechanism,
                &candidate.graph,
                item.binding,
            )?);
            source_proposals.push(item.proposal.semantic_hash());
            metrics.candidate_windows_without = metrics
                .candidate_windows_without
                .checked_add(verifier_fresh_window_work(
                    item.sources,
                    item.mechanism.pattern().len(),
                )?)
                .ok_or(InternalCompressionVerifyError::Capacity)?;
            metrics.examples_without = metrics
                .examples_without
                .checked_add(
                    u32::try_from(item.sources.len())
                        .map_err(|_| InternalCompressionVerifyError::Capacity)?,
                )
                .ok_or(InternalCompressionVerifyError::Capacity)?;
            metrics.proof_units_without = metrics
                .proof_units_without
                .checked_add(item.proposal.uncompressed_nodes())
                .ok_or(InternalCompressionVerifyError::Capacity)?;
            let replay_units = 1_u64
                .checked_add(
                    u64::try_from(item.mechanism.parameter_types().len())
                        .map_err(|_| InternalCompressionVerifyError::Capacity)?,
                )
                .and_then(|value| {
                    value.checked_add(u64::try_from(item.mechanism.boundary_types().len()).ok()?)
                })
                .ok_or(InternalCompressionVerifyError::Capacity)?;
            metrics.proof_units_with = metrics
                .proof_units_with
                .checked_add(replay_units)
                .ok_or(InternalCompressionVerifyError::Capacity)?;
        }
        if metrics.candidate_windows_with >= metrics.candidate_windows_without
            || metrics.examples_with >= metrics.examples_without
            || metrics.proof_units_with >= metrics.proof_units_without
        {
            return Err(InternalCompressionVerifyError::NoLearningReduction);
        }
        let joint = candidate
            .graph
            .nodes
            .last()
            .ok_or(InternalCompressionVerifyError::InvalidJointResult)?;
        let expected_inputs = ordered_evidence
            .iter()
            .map(|item| {
                item.binding
                    .selected_nodes
                    .get(
                        usize::try_from(item.mechanism.output())
                            .map_err(|_| InternalCompressionVerifyError::Capacity)?,
                    )
                    .copied()
                    .ok_or(InternalCompressionVerifyError::InvalidJointResult)
            })
            .collect::<Result<Vec<_>, _>>()?;
        if joint.operation != "join-learned-results"
            || joint.inputs != expected_inputs
            || candidate.graph.output
                != u32::try_from(candidate.graph.nodes.len() - 1)
                    .map_err(|_| InternalCompressionVerifyError::Capacity)?
        {
            return Err(InternalCompressionVerifyError::InvalidJointResult);
        }
        let composition_certificate = verifier_internal_composition_certificate(
            candidate.graph.episode,
            &source_proposals,
            &invocations,
            metrics,
        )?;
        let composition = VerifiedInternalComposition {
            experience: candidate.graph.episode,
            invocations,
            metrics,
            certificate: composition_certificate,
        };
        let frontier_hash = verifier_frontier_hash(&lowering.frontier)?;
        let certificate = CanonicalValue::Array(vec![
            CanonicalValue::Text("cairn.verified-relational-construction.v1".to_owned()),
            CanonicalValue::Bytes(verified_frontier.certificate().as_bytes().to_vec()),
            CanonicalValue::Bytes(frontier_hash.as_bytes().to_vec()),
            CanonicalValue::Bytes(policy_hash.as_bytes().to_vec()),
            CanonicalValue::Bytes(policy_verification.certificate().as_bytes().to_vec()),
            CanonicalValue::Bytes(composition.certificate().as_bytes().to_vec()),
        ])
        .encode_canonical()
        .map(|bytes| Hash32::digest(&bytes))
        .map_err(|_| InternalCompressionVerifyError::Canonicalization)?;
        Ok(VerifiedInternalConstruction {
            frontier: frontier_hash,
            composition,
            certificate,
        })
    }
}

/// Separate consolidation authority for runtime evidence about compiled
/// internal mechanisms.
#[derive(Clone, Copy, Debug, Default)]
pub struct InternalPromotionConsolidator;

impl InternalPromotionConsolidator {
    /// Aggregate at least two independent verifier receipts into a promotion
    /// certificate bound to mechanism and source-proposal identity.
    ///
    /// # Errors
    ///
    /// Rejects duplicate/mixed experiences, changed compilation lineage, or
    /// any receipt without strict candidate, sample, and proof-size reduction.
    pub fn consolidate(
        &self,
        mechanism: &CompiledInternalMechanism,
        compilations: &[VerifiedInternalMechanism],
        receipts: &[VerifiedInternalReuse],
    ) -> Result<VerifiedInternalPromotion, InternalCompressionVerifyError> {
        let experiences = receipts
            .iter()
            .map(VerifiedInternalReuse::experience)
            .collect::<BTreeSet<_>>();
        if receipts.len() < 2 || experiences.len() != receipts.len() {
            return Err(InternalCompressionVerifyError::InsufficientReuseEvidence);
        }
        let source_proposals = compilations
            .iter()
            .map(VerifiedInternalMechanism::source_proposal)
            .collect::<BTreeSet<_>>();
        if compilations.is_empty()
            || source_proposals.len() != compilations.len()
            || compilations
                .iter()
                .any(|compilation| compilation.mechanism != mechanism.semantic_hash())
            || receipts.iter().any(|receipt| {
                receipt.mechanism != mechanism.semantic_hash()
                    || !source_proposals.contains(&receipt.source_proposal)
            })
        {
            return Err(InternalCompressionVerifyError::CompiledLineageMismatch);
        }
        if receipts.iter().any(|receipt| {
            let metrics = receipt.metrics;
            metrics.candidate_windows_with >= metrics.candidate_windows_without
                || metrics.examples_with >= metrics.examples_without
                || metrics.proof_units_with >= metrics.proof_units_without
        }) {
            return Err(InternalCompressionVerifyError::NoLearningReduction);
        }
        let verified_cases = receipts.iter().try_fold(0_u64, |total, receipt| {
            total
                .checked_add(receipt.verified_cases)
                .ok_or(InternalCompressionVerifyError::Capacity)
        })?;
        Ok(VerifiedInternalPromotion {
            mechanism: mechanism.semantic_hash(),
            source_proposals,
            experiences,
            verified_cases,
        })
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct InternalCompressionVerifier;

impl InternalCompressionVerifier {
    /// Independently reconstruct every mined internal occurrence, including
    /// source-local boundary edges, then recalculate representation costs and
    /// semantic identity without using the producer's mining implementation.
    ///
    /// # Errors
    ///
    /// Rejects source/binding disagreement, malformed hosts, invalid selected
    /// windows or boundary mappings, reconstruction mismatch, cost or identity
    /// disagreement, and non-positive compression.
    pub fn verify(
        &self,
        proposal: &InternalGraphCompressionProposal,
        sources: &[ProofGraph],
    ) -> Result<VerifiedInternalCompression, InternalCompressionVerifyError> {
        if sources.is_empty() || proposal.pattern().is_empty() {
            return Err(InternalCompressionVerifyError::EmptyEvidence);
        }
        let source_ids = sources
            .iter()
            .map(|source| source.episode)
            .collect::<BTreeSet<_>>();
        if source_ids.len() != sources.len() {
            return Err(InternalCompressionVerifyError::DuplicateEpisode);
        }
        let proposal_ids = proposal
            .source_episodes()
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        if source_ids != proposal_ids || proposal.bindings().len() != sources.len() {
            return Err(InternalCompressionVerifyError::SourceMismatch);
        }
        let mut raw_units = 0_u64;
        for source in sources {
            verifier_validate_graph(source)?;
            let binding = proposal
                .bindings()
                .iter()
                .find(|binding| binding.episode == source.episode)
                .ok_or(InternalCompressionVerifyError::MissingBinding)?;
            let occurrence = verifier_internal_occurrence(source, binding, proposal.pattern())?;
            if occurrence.boundary_types != proposal.boundary_types() {
                return Err(InternalCompressionVerifyError::BoundaryMismatch);
            }
            let reconstructed = verifier_instantiate_internal(proposal.pattern(), binding)?;
            if occurrence.nodes != reconstructed
                || proposal.output()
                    != u32::try_from(reconstructed.len().saturating_sub(1))
                        .map_err(|_| InternalCompressionVerifyError::Capacity)?
            {
                return Err(InternalCompressionVerifyError::ReplayMismatch);
            }
            raw_units = raw_units
                .checked_add(verifier_internal_units(&occurrence.nodes)?)
                .ok_or(InternalCompressionVerifyError::Capacity)?;
        }
        let compressed_units = verifier_internal_compressed_units(
            proposal.pattern(),
            proposal.boundary_types(),
            proposal.bindings(),
        )?;
        if raw_units != proposal.uncompressed_nodes()
            || compressed_units != proposal.compressed_nodes()
        {
            return Err(InternalCompressionVerifyError::CostMismatch);
        }
        if compressed_units >= raw_units {
            return Err(InternalCompressionVerifyError::NoCompression);
        }
        let identity = verifier_internal_identity(
            proposal.pattern(),
            proposal.boundary_types(),
            proposal.source_episodes(),
            raw_units,
            compressed_units,
        )?;
        if identity != proposal.semantic_hash() {
            return Err(InternalCompressionVerifyError::IdentityMismatch);
        }
        Ok(VerifiedInternalCompression {
            proposal: identity,
            source_episodes: source_ids,
            uncompressed_nodes: raw_units,
            compressed_nodes: compressed_units,
            replayed_sources: u64::try_from(sources.len())
                .map_err(|_| InternalCompressionVerifyError::Capacity)?,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct VerifierInternalNode {
    operation: String,
    output_type: ProofValueType,
    inputs: Vec<InternalPatternInput>,
    arguments: Vec<ProofDatum>,
}

struct VerifierInternalOccurrence {
    nodes: Vec<VerifierInternalNode>,
    boundary_types: Vec<ProofValueType>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedPromotion {
    source_proofs: BTreeSet<Hash32>,
    verified_cases: u64,
    actual_reuse_count: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedProceduralPromotion {
    theory_hash: Hash32,
    source_proofs: BTreeSet<Hash32>,
    verified_cases: u64,
    actual_reuse_count: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CounterfactualReuseMetrics {
    pub blank_solved: bool,
    pub candidate_executions_without: u64,
    pub candidate_executions_with: u64,
    pub examples_without: u32,
    pub examples_with: u32,
    pub proof_nodes_without: u64,
    pub proof_nodes_with: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedProceduralReuse {
    theory_hash: Hash32,
    experience: Hash32,
    verified_cases: u64,
    metrics: CounterfactualReuseMetrics,
}

impl VerifiedProceduralReuse {
    #[must_use]
    pub const fn theory_hash(&self) -> Hash32 {
        self.theory_hash
    }

    #[must_use]
    pub const fn experience(&self) -> Hash32 {
        self.experience
    }

    #[must_use]
    pub const fn verified_cases(&self) -> u64 {
        self.verified_cases
    }

    #[must_use]
    pub const fn metrics(&self) -> CounterfactualReuseMetrics {
        self.metrics
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProceduralExperimentBaseline {
    pub target_absent_from_named_vocabulary: bool,
    pub source_code_changes: u32,
    pub initial_success_parts_per_million: u32,
    pub uncompressed_cost: u64,
    pub compressed_cost: u64,
    pub false_acceptances_without: u32,
    pub false_acceptances_with: u32,
}

impl VerifiedProceduralPromotion {
    #[must_use]
    pub const fn theory_hash(&self) -> Hash32 {
        self.theory_hash
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
}

impl VerifiedPromotion {
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
}

#[derive(Clone, Copy, Debug, Default)]
pub struct PromotionVerifier;

impl PromotionVerifier {
    /// Independently replay a whole collection theory on current and held-out
    /// evidence and bind the result to its canonical identity.
    ///
    /// # Errors
    ///
    /// Rejects malformed evidence, failed replay, empty counterfactual work, or
    /// a held-out mismatch.
    pub fn verify_collection_reuse(
        &self,
        theory: &CheckedTheory,
        problem: &CollectionProcedureProblem,
        expected_prediction: &BTreeSet<i64>,
        experience: Hash32,
        metrics: CounterfactualReuseMetrics,
    ) -> Result<VerifiedProceduralReuse, AdmissionError> {
        if problem.demonstrations.is_empty()
            || problem.prediction_input.is_empty()
            || metrics.candidate_executions_without == 0
            || metrics.examples_without == 0
            || metrics.blank_solved
        {
            return Err(AdmissionError::Unverified);
        }
        for demonstration in &problem.demonstrations {
            if execute_collection(theory, &demonstration.input)
                != Some(demonstration.output.clone())
            {
                return Err(AdmissionError::Unverified);
            }
        }
        if execute_collection(theory, &problem.prediction_input)
            != Some(expected_prediction.clone())
        {
            return Err(AdmissionError::Unverified);
        }
        let bytes = theory
            .encode_canonical()
            .map_err(|_| AdmissionError::Unverified)?;
        Ok(VerifiedProceduralReuse {
            theory_hash: Hash32::digest(&bytes),
            experience,
            verified_cases: u64::try_from(problem.demonstrations.len())
                .unwrap_or(u64::MAX)
                .saturating_add(1),
            metrics,
        })
    }

    /// Validate a mechanism and the evidence that it changed future learning.
    /// Replay certificates must already have been issued by `cairn-proof` and
    /// are represented in the mechanism derivation certificate.
    ///
    /// # Errors
    ///
    /// Returns an exact representation or learning-evidence failure.
    pub fn verify(
        &self,
        mechanism: &Mechanism,
        evidence: &InventionEvidence,
    ) -> Result<VerifiedPromotion, AdmissionError> {
        if admit_persistent(mechanism, evidence)? != MechanismStatus::Persistent {
            return Err(AdmissionError::Unverified);
        }
        Ok(VerifiedPromotion {
            source_proofs: mechanism.proof.source_proofs.clone(),
            verified_cases: mechanism.proof.verified_cases,
            actual_reuse_count: evidence.actual_reuse_count,
        })
    }

    /// Bind strict learning evidence to the canonical bytes of a complete
    /// checked theory, including definitions, quantified rules, and schedule.
    ///
    /// # Errors
    ///
    /// Rejects ordinary mechanism admission failures, disagreement between
    /// Mechanism IR and the checked theory, or canonical encoding failure.
    pub fn verify_procedural(
        &self,
        mechanism: &Mechanism,
        theory: &CheckedTheory,
        evidence: &InventionEvidence,
    ) -> Result<VerifiedProceduralPromotion, AdmissionError> {
        if admit_persistent(mechanism, evidence)? != MechanismStatus::Persistent {
            return Err(AdmissionError::Unverified);
        }
        if theory.theory().definitions != mechanism.entity_definitions
            || theory.theory().rules != mechanism.transition_rules
            || theory
                .theory()
                .inputs
                .iter()
                .map(|input| input.ty.clone())
                .collect::<Vec<_>>()
                != mechanism.signature.inputs
        {
            return Err(AdmissionError::Unverified);
        }
        let bytes = theory
            .encode_canonical()
            .map_err(|_| AdmissionError::Unverified)?;
        Ok(VerifiedProceduralPromotion {
            theory_hash: Hash32::digest(&bytes),
            source_proofs: mechanism.proof.source_proofs.clone(),
            verified_cases: mechanism.proof.verified_cases,
            actual_reuse_count: evidence.actual_reuse_count,
        })
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ProceduralPromotionConsolidator;

impl ProceduralPromotionConsolidator {
    /// Aggregate independent verifier receipts into the strict persistent
    /// learning gate and return a theory-bound promotion certificate.
    ///
    /// # Errors
    ///
    /// Rejects fewer than two independent experiences, mixed theory identity,
    /// duplicate experiences, or any unmet persistent-learning criterion.
    pub fn consolidate(
        &self,
        mechanism: &Mechanism,
        theory: &CheckedTheory,
        baseline: ProceduralExperimentBaseline,
        receipts: &[VerifiedProceduralReuse],
    ) -> Result<VerifiedProceduralPromotion, AdmissionError> {
        let theory_hash = Hash32::digest(
            &theory
                .encode_canonical()
                .map_err(|_| AdmissionError::Unverified)?,
        );
        let experiences = receipts
            .iter()
            .map(VerifiedProceduralReuse::experience)
            .collect::<BTreeSet<_>>();
        if receipts.len() < 2
            || experiences.len() != receipts.len()
            || receipts
                .iter()
                .any(|receipt| receipt.theory_hash != theory_hash)
        {
            return Err(AdmissionError::InsufficientIndependentExperience);
        }
        let sum_u64 = |select: fn(CounterfactualReuseMetrics) -> u64| {
            receipts
                .iter()
                .map(|receipt| select(receipt.metrics))
                .fold(0_u64, u64::saturating_add)
        };
        let sum_u32 = |select: fn(CounterfactualReuseMetrics) -> u32| {
            receipts
                .iter()
                .map(|receipt| select(receipt.metrics))
                .fold(0_u32, u32::saturating_add)
        };
        let evidence = InventionEvidence {
            target_absent_from_named_vocabulary: baseline.target_absent_from_named_vocabulary,
            source_code_changes: baseline.source_code_changes,
            independent_experiences: u32::try_from(experiences.len()).unwrap_or(u32::MAX),
            initial_success_parts_per_million: baseline.initial_success_parts_per_million,
            post_experience_success_parts_per_million: 1_000_000,
            uncompressed_cost: baseline.uncompressed_cost,
            compressed_cost: baseline.compressed_cost,
            held_out_verified_cases: receipts
                .iter()
                .map(VerifiedProceduralReuse::verified_cases)
                .fold(0_u64, u64::saturating_add),
            candidate_executions_without: sum_u64(|metrics| metrics.candidate_executions_without),
            candidate_executions_with: sum_u64(|metrics| metrics.candidate_executions_with),
            examples_without: sum_u32(|metrics| metrics.examples_without),
            examples_with: sum_u32(|metrics| metrics.examples_with),
            false_acceptances_without: baseline.false_acceptances_without,
            false_acceptances_with: baseline.false_acceptances_with,
            actual_reuse_count: u32::try_from(receipts.len()).unwrap_or(u32::MAX),
            necessary_reuse_count: u32::try_from(
                receipts
                    .iter()
                    .filter(|receipt| !receipt.metrics.blank_solved)
                    .count(),
            )
            .unwrap_or(u32::MAX),
        };
        PromotionVerifier.verify_procedural(mechanism, theory, &evidence)
    }
}

fn execute_collection(theory: &CheckedTheory, input: &[i64]) -> Option<BTreeSet<i64>> {
    let binder = theory.theory().inputs.first()?;
    let Type::List { element, maximum } = &binder.ty else {
        return None;
    };
    if input.len() > usize::try_from(*maximum).ok()? {
        return None;
    }
    let mut types = TypeEnvironment::new();
    types.insert(binder.name.clone(), binder.ty.clone());
    let mut values = RuntimeEnvironment::new();
    values.insert(
        binder.name.clone(),
        Value::Set {
            element_type: (**element).clone(),
            maximum: *maximum,
            values: input.iter().copied().map(Value::Int).collect(),
        },
    );
    let trace = ReferenceTheoryExecutor
        .execute(
            theory,
            &TheoryExecutionRequest {
                types,
                values,
                initial_facts: BTreeSet::new(),
                initial_state: BTreeMap::new(),
            },
            TheoryExecLimits {
                expression: cairn_rkl::ExecLimits::default(),
                maximum_bindings: u64::from(*maximum),
                maximum_rule_firings: u64::from(*maximum),
            },
        )
        .ok()?;
    trace
        .rendered
        .facts
        .into_iter()
        .map(|value| match value {
            Value::Int(value) => Some(value),
            _ => None,
        })
        .collect()
}

fn verifier_validate_graph(graph: &ProofGraph) -> Result<(), InternalCompressionVerifyError> {
    let output =
        usize::try_from(graph.output).map_err(|_| InternalCompressionVerifyError::Capacity)?;
    if graph.nodes.is_empty() || output >= graph.nodes.len() {
        return Err(InternalCompressionVerifyError::InvalidOutput);
    }
    for (index, node) in graph.nodes.iter().enumerate() {
        if node.operation.is_empty()
            || node
                .inputs
                .iter()
                .any(|input| usize::try_from(*input).map_or(true, |value| value >= index))
        {
            return Err(InternalCompressionVerifyError::InvalidTopology);
        }
    }
    Ok(())
}

fn verifier_validate_frontier(
    frontier: &ProofFrontier,
) -> Result<(), InternalCompressionVerifyError> {
    if frontier.nodes.is_empty() {
        return Err(InternalCompressionVerifyError::FrontierMismatch);
    }
    for (index, node) in frontier.nodes.iter().enumerate() {
        if node.operation.is_empty()
            || node.operation == "join-learned-results"
            || node
                .inputs
                .iter()
                .any(|input| usize::try_from(*input).map_or(true, |value| value >= index))
        {
            return Err(InternalCompressionVerifyError::FrontierMismatch);
        }
    }
    Ok(())
}

fn verifier_frontier_hash(
    frontier: &ProofFrontier,
) -> Result<Hash32, InternalCompressionVerifyError> {
    CanonicalValue::Array(vec![
        CanonicalValue::Text("cairn.proof-frontier.v1".to_owned()),
        CanonicalValue::Bytes(frontier.episode.as_bytes().to_vec()),
        CanonicalValue::Array(
            frontier
                .nodes
                .iter()
                .map(|node| {
                    CanonicalValue::Array(vec![
                        CanonicalValue::Text(node.operation.clone()),
                        CanonicalValue::Unsigned(verifier_proof_type_code(node.output_type)),
                        CanonicalValue::Array(
                            node.inputs
                                .iter()
                                .map(|input| CanonicalValue::Unsigned(u64::from(*input)))
                                .collect(),
                        ),
                        CanonicalValue::Array(
                            node.arguments
                                .iter()
                                .map(|argument| {
                                    CanonicalValue::Array(vec![
                                        CanonicalValue::Unsigned(verifier_proof_type_code(
                                            argument.ty,
                                        )),
                                        CanonicalValue::Bytes(argument.value.as_bytes().to_vec()),
                                    ])
                                })
                                .collect(),
                        ),
                    ])
                })
                .collect(),
        ),
    ])
    .encode_canonical()
    .map(|bytes| Hash32::digest(&bytes))
    .map_err(|_| InternalCompressionVerifyError::Canonicalization)
}

fn verifier_lower_observation(
    observation: &RelationalObservation,
    limits: ObservationFrontierLimits,
) -> Result<(ProofFrontier, Vec<ObservationFrontierProvenance>), InternalCompressionVerifyError> {
    let node_count = 1_usize
        .checked_add(observation.elements.len())
        .and_then(|count| count.checked_add(observation.relations.len()))
        .ok_or(InternalCompressionVerifyError::Capacity)?;
    if node_count
        > usize::try_from(limits.maximum_frontier_nodes)
            .map_err(|_| InternalCompressionVerifyError::Capacity)?
    {
        return Err(InternalCompressionVerifyError::Capacity);
    }
    let mut nodes = vec![ProofNode {
        operation: "observe-relational-evidence".to_owned(),
        output_type: ProofValueType::EntitySet,
        inputs: Vec::new(),
        arguments: vec![ProofDatum {
            ty: ProofValueType::Atom,
            value: observation.observation.id.0,
        }],
    }];
    let mut provenance = vec![ObservationFrontierProvenance::Observation];
    let mut element_nodes = BTreeMap::new();
    for element in &observation.elements {
        let node =
            u32::try_from(nodes.len()).map_err(|_| InternalCompressionVerifyError::Capacity)?;
        element_nodes.insert(element.id, node);
        let value_type = verifier_evidence_type(element.value_type);
        nodes.push(ProofNode {
            operation: "observe-evidence-element".to_owned(),
            output_type: value_type,
            inputs: vec![0],
            arguments: vec![ProofDatum {
                ty: value_type,
                value: element.value,
            }],
        });
        provenance.push(ObservationFrontierProvenance::Element(element.id));
    }
    for relation in &observation.relations {
        let inputs = relation
            .participants
            .iter()
            .map(|participant| {
                element_nodes
                    .get(participant)
                    .copied()
                    .ok_or(InternalCompressionVerifyError::InvalidObservation)
            })
            .collect::<Result<Vec<_>, _>>()?;
        nodes.push(ProofNode {
            operation: "observe-evidence-relation".to_owned(),
            output_type: ProofValueType::Relation,
            inputs,
            arguments: vec![ProofDatum {
                ty: ProofValueType::Relation,
                value: relation.predicate,
            }],
        });
        provenance.push(ObservationFrontierProvenance::Relation {
            predicate: relation.predicate,
            participants: relation.participants.clone(),
        });
    }
    Ok((
        ProofFrontier {
            episode: observation.observation.id.0,
            nodes,
        },
        provenance,
    ))
}

fn verifier_observation_frontier_hash(
    frontier: &ProofFrontier,
    provenance: &[ObservationFrontierProvenance],
) -> Result<Hash32, InternalCompressionVerifyError> {
    if frontier.nodes.len() != provenance.len() {
        return Err(InternalCompressionVerifyError::ObservationFrontierMismatch);
    }
    let nodes = frontier
        .nodes
        .iter()
        .map(|node| {
            CanonicalValue::Array(vec![
                CanonicalValue::Text(node.operation.clone()),
                CanonicalValue::Unsigned(verifier_proof_type_code(node.output_type)),
                CanonicalValue::Array(
                    node.inputs
                        .iter()
                        .map(|input| CanonicalValue::Unsigned(u64::from(*input)))
                        .collect(),
                ),
                CanonicalValue::Array(
                    node.arguments
                        .iter()
                        .map(|argument| {
                            CanonicalValue::Array(vec![
                                CanonicalValue::Unsigned(verifier_proof_type_code(argument.ty)),
                                CanonicalValue::Bytes(argument.value.as_bytes().to_vec()),
                            ])
                        })
                        .collect(),
                ),
            ])
        })
        .collect();
    let provenance = provenance
        .iter()
        .map(|source| match source {
            ObservationFrontierProvenance::Observation => {
                CanonicalValue::Array(vec![CanonicalValue::Unsigned(0)])
            }
            ObservationFrontierProvenance::Element(id) => CanonicalValue::Array(vec![
                CanonicalValue::Unsigned(1),
                CanonicalValue::Bytes(id.0.as_bytes().to_vec()),
            ]),
            ObservationFrontierProvenance::Relation {
                predicate,
                participants,
            } => CanonicalValue::Array(vec![
                CanonicalValue::Unsigned(2),
                CanonicalValue::Bytes(predicate.as_bytes().to_vec()),
                CanonicalValue::Array(
                    participants
                        .iter()
                        .map(|participant| CanonicalValue::Bytes(participant.0.as_bytes().to_vec()))
                        .collect(),
                ),
            ]),
        })
        .collect();
    CanonicalValue::Array(vec![
        CanonicalValue::Text("cairn.observation-frontier.v1".to_owned()),
        CanonicalValue::Bytes(frontier.episode.as_bytes().to_vec()),
        CanonicalValue::Array(nodes),
        CanonicalValue::Array(provenance),
    ])
    .encode_canonical()
    .map(|bytes| Hash32::digest(&bytes))
    .map_err(|_| InternalCompressionVerifyError::Canonicalization)
}

fn verifier_frontier_node_matches(
    frontier: &ProofFrontier,
    index: usize,
    selector: &FrontierNodeSelector,
) -> Result<bool, InternalCompressionVerifyError> {
    let node = frontier
        .nodes
        .get(index)
        .ok_or(InternalCompressionVerifyError::FrontierMismatch)?;
    let input_types = node
        .inputs
        .iter()
        .map(|input| {
            frontier
                .nodes
                .get(
                    usize::try_from(*input)
                        .map_err(|_| InternalCompressionVerifyError::Capacity)?,
                )
                .map(|input| input.output_type)
                .ok_or(InternalCompressionVerifyError::FrontierMismatch)
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(node.operation == selector.operation
        && node.output_type == selector.output_type
        && input_types == selector.input_types
        && node
            .arguments
            .iter()
            .map(|argument| argument.ty)
            .eq(selector.argument_types.iter().copied()))
}

fn verifier_frontier_node_selector(
    frontier: &ProofFrontier,
    node: u32,
) -> Result<FrontierNodeSelector, InternalCompressionVerifyError> {
    let node = frontier
        .nodes
        .get(usize::try_from(node).map_err(|_| InternalCompressionVerifyError::Capacity)?)
        .ok_or(InternalCompressionVerifyError::FrontierMismatch)?;
    let input_types = node
        .inputs
        .iter()
        .map(|input| {
            frontier
                .nodes
                .get(
                    usize::try_from(*input)
                        .map_err(|_| InternalCompressionVerifyError::Capacity)?,
                )
                .map(|input| input.output_type)
                .ok_or(InternalCompressionVerifyError::FrontierMismatch)
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(FrontierNodeSelector {
        operation: node.operation.clone(),
        output_type: node.output_type,
        input_types,
        argument_types: node.arguments.iter().map(|argument| argument.ty).collect(),
    })
}

fn verifier_select_frontier_node(
    frontier: &ProofFrontier,
    selector: &FrontierNodeSelector,
) -> Result<u32, InternalCompressionVerifyError> {
    let mut matches = Vec::new();
    for index in 0..frontier.nodes.len() {
        if verifier_frontier_node_matches(frontier, index, selector)? {
            matches
                .push(u32::try_from(index).map_err(|_| InternalCompressionVerifyError::Capacity)?);
        }
    }
    match matches.as_slice() {
        [only] => Ok(*only),
        _ => Err(InternalCompressionVerifyError::RelationalPolicyMismatch),
    }
}

fn verifier_relational_fingerprints(
    observation: &RelationalObservation,
) -> Result<BTreeMap<EvidenceElementId, Hash32>, InternalCompressionVerifyError> {
    let mut colors = observation
        .elements
        .iter()
        .map(|element| {
            CanonicalValue::Array(vec![
                CanonicalValue::Text("cairn.relational-fingerprint.seed.v1".to_owned()),
                CanonicalValue::Unsigned(verifier_proof_type_code(verifier_evidence_type(
                    element.value_type,
                ))),
            ])
            .encode_canonical()
            .map(|bytes| (element.id, Hash32::digest(&bytes)))
            .map_err(|_| InternalCompressionVerifyError::Canonicalization)
        })
        .collect::<Result<BTreeMap<_, _>, _>>()?;
    for round in 0_u64..2 {
        let mut next = BTreeMap::new();
        for element in &observation.elements {
            let mut incident = observation
                .relations
                .iter()
                .filter(|relation| relation.participants.contains(&element.id))
                .map(|relation| {
                    let positions = relation
                        .participants
                        .iter()
                        .enumerate()
                        .filter(|(_, participant)| **participant == element.id)
                        .map(|(position, _)| {
                            u64::try_from(position)
                                .map(CanonicalValue::Unsigned)
                                .map_err(|_| InternalCompressionVerifyError::Capacity)
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    let participants = relation
                        .participants
                        .iter()
                        .map(|participant| {
                            colors
                                .get(participant)
                                .map(|color| CanonicalValue::Bytes(color.as_bytes().to_vec()))
                                .ok_or(InternalCompressionVerifyError::InvalidObservation)
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    CanonicalValue::Array(vec![
                        CanonicalValue::Bytes(relation.predicate.as_bytes().to_vec()),
                        CanonicalValue::Array(positions),
                        CanonicalValue::Array(participants),
                    ])
                    .encode_canonical()
                    .map_err(|_| InternalCompressionVerifyError::Canonicalization)
                })
                .collect::<Result<Vec<_>, _>>()?;
            incident.sort_unstable();
            let encoded = CanonicalValue::Array(vec![
                CanonicalValue::Text("cairn.relational-fingerprint.round.v1".to_owned()),
                CanonicalValue::Unsigned(round),
                CanonicalValue::Unsigned(verifier_proof_type_code(verifier_evidence_type(
                    element.value_type,
                ))),
                CanonicalValue::Array(incident.into_iter().map(CanonicalValue::Bytes).collect()),
            ])
            .encode_canonical()
            .map_err(|_| InternalCompressionVerifyError::Canonicalization)?;
            next.insert(element.id, Hash32::digest(&encoded));
        }
        colors = next;
    }
    Ok(colors)
}

fn verifier_select_relational_value(
    observation: &RelationalObservation,
    fingerprints: &BTreeMap<EvidenceElementId, Hash32>,
    selector: RelationalValueSelector,
) -> Result<ProofDatum, InternalCompressionVerifyError> {
    let matches = observation
        .elements
        .iter()
        .filter(|element| verifier_evidence_type(element.value_type) == selector.value_type)
        .filter(|element| fingerprints.get(&element.id) == Some(&selector.fingerprint))
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [element] => Ok(ProofDatum {
            ty: selector.value_type,
            value: element.value,
        }),
        _ => Err(InternalCompressionVerifyError::RelationalPolicyMismatch),
    }
}

fn verifier_relational_value_selector(
    observation: &RelationalObservation,
    fingerprints: &BTreeMap<EvidenceElementId, Hash32>,
    value: ProofDatum,
) -> Result<RelationalValueSelector, InternalCompressionVerifyError> {
    let matches = observation
        .elements
        .iter()
        .filter(|element| {
            verifier_evidence_type(element.value_type) == value.ty && element.value == value.value
        })
        .collect::<Vec<_>>();
    let [element] = matches.as_slice() else {
        return Err(InternalCompressionVerifyError::RelationalPolicyMismatch);
    };
    let fingerprint = *fingerprints
        .get(&element.id)
        .ok_or(InternalCompressionVerifyError::InvalidObservation)?;
    if observation
        .elements
        .iter()
        .filter(|candidate| verifier_evidence_type(candidate.value_type) == value.ty)
        .filter(|candidate| fingerprints.get(&candidate.id) == Some(&fingerprint))
        .count()
        != 1
    {
        return Err(InternalCompressionVerifyError::RelationalPolicyMismatch);
    }
    Ok(RelationalValueSelector {
        value_type: value.ty,
        fingerprint,
    })
}

fn verifier_relational_policy_hash(
    policy: &RelationalConstructionPolicy,
) -> Result<Hash32, InternalCompressionVerifyError> {
    let boundaries = policy
        .boundary_selectors
        .iter()
        .map(|selectors| {
            CanonicalValue::Array(
                selectors
                    .iter()
                    .map(|selector| {
                        CanonicalValue::Array(vec![
                            CanonicalValue::Text(selector.operation.clone()),
                            CanonicalValue::Unsigned(verifier_proof_type_code(
                                selector.output_type,
                            )),
                            CanonicalValue::Array(
                                selector
                                    .input_types
                                    .iter()
                                    .map(|ty| {
                                        CanonicalValue::Unsigned(verifier_proof_type_code(*ty))
                                    })
                                    .collect(),
                            ),
                            CanonicalValue::Array(
                                selector
                                    .argument_types
                                    .iter()
                                    .map(|ty| {
                                        CanonicalValue::Unsigned(verifier_proof_type_code(*ty))
                                    })
                                    .collect(),
                            ),
                        ])
                    })
                    .collect(),
            )
        })
        .collect();
    let values = policy
        .value_selectors
        .iter()
        .map(|selectors| {
            CanonicalValue::Array(
                selectors
                    .iter()
                    .map(|selector| {
                        CanonicalValue::Array(vec![
                            CanonicalValue::Unsigned(verifier_proof_type_code(selector.value_type)),
                            CanonicalValue::Bytes(selector.fingerprint.as_bytes().to_vec()),
                        ])
                    })
                    .collect(),
            )
        })
        .collect();
    CanonicalValue::Array(vec![
        CanonicalValue::Text("cairn.relational-construction-policy.v1".to_owned()),
        CanonicalValue::Array(
            policy
                .mechanisms
                .iter()
                .map(|mechanism| CanonicalValue::Bytes(mechanism.as_bytes().to_vec()))
                .collect(),
        ),
        CanonicalValue::Array(boundaries),
        CanonicalValue::Array(values),
    ])
    .encode_canonical()
    .map(|bytes| Hash32::digest(&bytes))
    .map_err(|_| InternalCompressionVerifyError::Canonicalization)
}

fn verifier_proof_ancestors(
    graph: &ProofGraph,
) -> Result<BTreeSet<u32>, InternalCompressionVerifyError> {
    let mut ancestors = BTreeSet::new();
    let mut frontier = vec![graph.output];
    while let Some(node_id) = frontier.pop() {
        if !ancestors.insert(node_id) {
            continue;
        }
        let node = graph
            .nodes
            .get(usize::try_from(node_id).map_err(|_| InternalCompressionVerifyError::Capacity)?)
            .ok_or(InternalCompressionVerifyError::InvalidTopology)?;
        frontier.extend(node.inputs.iter().copied());
    }
    Ok(ancestors)
}

fn verifier_internal_composition_certificate(
    experience: Hash32,
    source_proposals: &[Hash32],
    invocations: &[VerifiedInternalInvocation],
    metrics: InternalReuseMetrics,
) -> Result<Hash32, InternalCompressionVerifyError> {
    if source_proposals.len() != invocations.len() {
        return Err(InternalCompressionVerifyError::CompiledLineageMismatch);
    }
    CanonicalValue::Array(vec![
        CanonicalValue::Text("cairn.verified-internal-composition.v1".to_owned()),
        CanonicalValue::Bytes(experience.as_bytes().to_vec()),
        CanonicalValue::Array(
            source_proposals
                .iter()
                .zip(invocations)
                .map(|(proposal, invocation)| {
                    CanonicalValue::Array(vec![
                        CanonicalValue::Bytes(proposal.as_bytes().to_vec()),
                        CanonicalValue::Bytes(invocation.mechanism().as_bytes().to_vec()),
                        CanonicalValue::Array(
                            invocation
                                .selected_nodes()
                                .iter()
                                .map(|node| CanonicalValue::Unsigned(u64::from(*node)))
                                .collect(),
                        ),
                    ])
                })
                .collect(),
        ),
        CanonicalValue::Array(vec![
            CanonicalValue::Unsigned(metrics.candidate_windows_without),
            CanonicalValue::Unsigned(metrics.candidate_windows_with),
            CanonicalValue::Unsigned(u64::from(metrics.examples_without)),
            CanonicalValue::Unsigned(u64::from(metrics.examples_with)),
            CanonicalValue::Unsigned(metrics.proof_units_without),
            CanonicalValue::Unsigned(metrics.proof_units_with),
        ]),
    ])
    .encode_canonical()
    .map(|bytes| Hash32::digest(&bytes))
    .map_err(|_| InternalCompressionVerifyError::Canonicalization)
}

fn verifier_fresh_window_work(
    sources: &[ProofGraph],
    width: usize,
) -> Result<u64, InternalCompressionVerifyError> {
    const MAXIMUM_FRESH_WINDOWS: u64 = 10_000;
    let mut total = 0_u64;
    for source in sources {
        verifier_validate_graph(source)?;
        if width == 0 || width > source.nodes.len() {
            return Err(InternalCompressionVerifyError::ReplayMismatch);
        }
        let episode = u64::try_from(source.nodes.len() - width + 1)
            .map_err(|_| InternalCompressionVerifyError::Capacity)?;
        if episode > MAXIMUM_FRESH_WINDOWS {
            return Err(InternalCompressionVerifyError::Capacity);
        }
        total = total
            .checked_add(episode)
            .ok_or(InternalCompressionVerifyError::Capacity)?;
    }
    Ok(total)
}

#[allow(clippy::too_many_lines)] // Keep independent boundary reconstruction explicit.
fn verifier_internal_occurrence(
    graph: &ProofGraph,
    binding: &InternalEpisodeBindings,
    pattern: &[InternalGraphPatternNode],
) -> Result<VerifierInternalOccurrence, InternalCompressionVerifyError> {
    if binding.selected_nodes.len() != pattern.len()
        || binding.selected_nodes.is_empty()
        || binding
            .selected_nodes
            .windows(2)
            .any(|pair| pair[0].checked_add(1) != Some(pair[1]))
    {
        return Err(InternalCompressionVerifyError::InvalidSelection);
    }
    let selected_map = binding
        .selected_nodes
        .iter()
        .enumerate()
        .map(|(local, source)| {
            if usize::try_from(*source)
                .ok()
                .is_none_or(|source| source >= graph.nodes.len())
            {
                return Err(InternalCompressionVerifyError::InvalidSelection);
            }
            Ok((
                *source,
                u32::try_from(local).map_err(|_| InternalCompressionVerifyError::Capacity)?,
            ))
        })
        .collect::<Result<BTreeMap<_, _>, _>>()?;
    let mut expected_boundaries = BTreeSet::new();
    for source in &binding.selected_nodes {
        let source = usize::try_from(*source)
            .map_err(|_| InternalCompressionVerifyError::InvalidSelection)?;
        let node = graph
            .nodes
            .get(source)
            .ok_or(InternalCompressionVerifyError::InvalidSelection)?;
        expected_boundaries.extend(
            node.inputs
                .iter()
                .filter(|input| !selected_map.contains_key(input))
                .copied(),
        );
    }
    let expected_boundaries = expected_boundaries.into_iter().collect::<Vec<_>>();
    if expected_boundaries != binding.boundary_nodes {
        return Err(InternalCompressionVerifyError::BoundaryMismatch);
    }
    let boundary_map = binding
        .boundary_nodes
        .iter()
        .enumerate()
        .map(|(slot, source)| {
            Ok((
                *source,
                u32::try_from(slot).map_err(|_| InternalCompressionVerifyError::Capacity)?,
            ))
        })
        .collect::<Result<BTreeMap<_, _>, InternalCompressionVerifyError>>()?;
    let boundary_types = binding
        .boundary_nodes
        .iter()
        .map(|source| {
            graph
                .nodes
                .get(
                    usize::try_from(*source)
                        .map_err(|_| InternalCompressionVerifyError::Capacity)?,
                )
                .map(|node| node.output_type)
                .ok_or(InternalCompressionVerifyError::BoundaryMismatch)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let nodes = binding
        .selected_nodes
        .iter()
        .map(|source| {
            let node = graph
                .nodes
                .get(
                    usize::try_from(*source)
                        .map_err(|_| InternalCompressionVerifyError::Capacity)?,
                )
                .ok_or(InternalCompressionVerifyError::InvalidSelection)?;
            let inputs = node
                .inputs
                .iter()
                .map(|input| {
                    selected_map.get(input).copied().map_or_else(
                        || {
                            boundary_map
                                .get(input)
                                .copied()
                                .map(InternalPatternInput::Boundary)
                                .ok_or(InternalCompressionVerifyError::BoundaryMismatch)
                        },
                        |local| Ok(InternalPatternInput::Internal(local)),
                    )
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(VerifierInternalNode {
                operation: node.operation.clone(),
                output_type: node.output_type,
                inputs,
                arguments: node.arguments.clone(),
            })
        })
        .collect::<Result<Vec<_>, InternalCompressionVerifyError>>()?;
    Ok(VerifierInternalOccurrence {
        nodes,
        boundary_types,
    })
}

fn verifier_instantiate_internal(
    pattern: &[InternalGraphPatternNode],
    binding: &InternalEpisodeBindings,
) -> Result<Vec<VerifierInternalNode>, InternalCompressionVerifyError> {
    pattern
        .iter()
        .map(|node| {
            let arguments = node
                .arguments
                .iter()
                .map(|argument| match argument {
                    GraphPatternArgument::Fixed(value) => Ok(*value),
                    GraphPatternArgument::Parameter(index) => binding
                        .values
                        .get(
                            usize::try_from(*index)
                                .map_err(|_| InternalCompressionVerifyError::Capacity)?,
                        )
                        .copied()
                        .ok_or(InternalCompressionVerifyError::MissingBinding),
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(VerifierInternalNode {
                operation: node.operation.clone(),
                output_type: node.output_type,
                inputs: node.inputs.clone(),
                arguments,
            })
        })
        .collect()
}

fn verifier_internal_units(
    nodes: &[VerifierInternalNode],
) -> Result<u64, InternalCompressionVerifyError> {
    nodes.iter().try_fold(0_u64, |total, node| {
        total
            .checked_add(1)
            .and_then(|value| value.checked_add(u64::try_from(node.arguments.len()).ok()?))
            .ok_or(InternalCompressionVerifyError::Capacity)
    })
}

fn verifier_internal_compressed_units(
    pattern: &[InternalGraphPatternNode],
    boundary_types: &[ProofValueType],
    bindings: &[InternalEpisodeBindings],
) -> Result<u64, InternalCompressionVerifyError> {
    let fixed_arguments = pattern.iter().try_fold(0_u64, |total, node| {
        let count = node
            .arguments
            .iter()
            .filter(|argument| matches!(argument, GraphPatternArgument::Fixed(_)))
            .count();
        total
            .checked_add(
                u64::try_from(count).map_err(|_| InternalCompressionVerifyError::Capacity)?,
            )
            .ok_or(InternalCompressionVerifyError::Capacity)
    })?;
    let parameters = bindings.first().map_or(0, |binding| binding.values.len());
    let boundaries = boundary_types.len();
    if bindings.iter().any(|binding| {
        binding.values.len() != parameters || binding.boundary_nodes.len() != boundaries
    }) {
        return Err(InternalCompressionVerifyError::MissingBinding);
    }
    let parameters =
        u64::try_from(parameters).map_err(|_| InternalCompressionVerifyError::Capacity)?;
    let boundaries =
        u64::try_from(boundaries).map_err(|_| InternalCompressionVerifyError::Capacity)?;
    let episodes =
        u64::try_from(bindings.len()).map_err(|_| InternalCompressionVerifyError::Capacity)?;
    u64::try_from(pattern.len())
        .map_err(|_| InternalCompressionVerifyError::Capacity)?
        .checked_add(fixed_arguments)
        .and_then(|value| value.checked_add(parameters))
        .and_then(|value| value.checked_add(boundaries))
        .and_then(|value| {
            value.checked_add(parameters.checked_add(boundaries)?.checked_mul(episodes)?)
        })
        .ok_or(InternalCompressionVerifyError::Capacity)
}

fn verifier_internal_identity(
    pattern: &[InternalGraphPatternNode],
    boundary_types: &[ProofValueType],
    episodes: &[Hash32],
    uncompressed: u64,
    compressed: u64,
) -> Result<Hash32, InternalCompressionVerifyError> {
    let value = CanonicalValue::Array(vec![
        CanonicalValue::Text("cairn.consolidator.internal-compression.v1".to_owned()),
        CanonicalValue::Array(
            boundary_types
                .iter()
                .map(|ty| CanonicalValue::Unsigned(verifier_proof_type_code(*ty)))
                .collect(),
        ),
        CanonicalValue::Array(
            pattern
                .iter()
                .map(|node| {
                    CanonicalValue::Array(vec![
                        CanonicalValue::Text(node.operation.clone()),
                        CanonicalValue::Unsigned(verifier_proof_type_code(node.output_type)),
                        CanonicalValue::Array(
                            node.inputs
                                .iter()
                                .copied()
                                .map(verifier_internal_input_value)
                                .collect(),
                        ),
                        CanonicalValue::Array(
                            node.arguments
                                .iter()
                                .map(|argument| match argument {
                                    GraphPatternArgument::Fixed(value) => {
                                        CanonicalValue::Array(vec![
                                            CanonicalValue::Unsigned(0),
                                            CanonicalValue::Unsigned(verifier_proof_type_code(
                                                value.ty,
                                            )),
                                            CanonicalValue::Bytes(value.value.as_bytes().to_vec()),
                                        ])
                                    }
                                    GraphPatternArgument::Parameter(index) => {
                                        CanonicalValue::Array(vec![
                                            CanonicalValue::Unsigned(1),
                                            CanonicalValue::Unsigned(u64::from(*index)),
                                        ])
                                    }
                                })
                                .collect(),
                        ),
                    ])
                })
                .collect(),
        ),
        CanonicalValue::Array(
            episodes
                .iter()
                .map(|episode| CanonicalValue::Bytes(episode.as_bytes().to_vec()))
                .collect(),
        ),
        CanonicalValue::Unsigned(uncompressed),
        CanonicalValue::Unsigned(compressed),
    ]);
    value
        .encode_canonical()
        .map(|bytes| Hash32::digest(&bytes))
        .map_err(|_| InternalCompressionVerifyError::IdentityMismatch)
}

fn verifier_internal_mechanism_identity(
    pattern: &[InternalGraphPatternNode],
    boundary_types: &[ProofValueType],
    parameter_types: &[ProofValueType],
    output: u32,
) -> Result<Hash32, InternalCompressionVerifyError> {
    let value = CanonicalValue::Array(vec![
        CanonicalValue::Text("cairn.consolidator.compiled-internal-mechanism.v1".to_owned()),
        CanonicalValue::Array(
            boundary_types
                .iter()
                .map(|ty| CanonicalValue::Unsigned(verifier_proof_type_code(*ty)))
                .collect(),
        ),
        CanonicalValue::Array(
            parameter_types
                .iter()
                .map(|ty| CanonicalValue::Unsigned(verifier_proof_type_code(*ty)))
                .collect(),
        ),
        CanonicalValue::Unsigned(u64::from(output)),
        CanonicalValue::Array(
            pattern
                .iter()
                .map(|node| {
                    CanonicalValue::Array(vec![
                        CanonicalValue::Text(node.operation.clone()),
                        CanonicalValue::Unsigned(verifier_proof_type_code(node.output_type)),
                        CanonicalValue::Array(
                            node.inputs
                                .iter()
                                .copied()
                                .map(verifier_internal_input_value)
                                .collect(),
                        ),
                        CanonicalValue::Array(
                            node.arguments
                                .iter()
                                .map(|argument| match argument {
                                    GraphPatternArgument::Fixed(value) => {
                                        CanonicalValue::Array(vec![
                                            CanonicalValue::Unsigned(0),
                                            CanonicalValue::Unsigned(verifier_proof_type_code(
                                                value.ty,
                                            )),
                                            CanonicalValue::Bytes(value.value.as_bytes().to_vec()),
                                        ])
                                    }
                                    GraphPatternArgument::Parameter(index) => {
                                        CanonicalValue::Array(vec![
                                            CanonicalValue::Unsigned(1),
                                            CanonicalValue::Unsigned(u64::from(*index)),
                                        ])
                                    }
                                })
                                .collect(),
                        ),
                    ])
                })
                .collect(),
        ),
    ]);
    value
        .encode_canonical()
        .map(|bytes| Hash32::digest(&bytes))
        .map_err(|_| InternalCompressionVerifyError::IdentityMismatch)
}

fn verifier_internal_input_value(input: InternalPatternInput) -> CanonicalValue {
    match input {
        InternalPatternInput::Internal(index) => CanonicalValue::Array(vec![
            CanonicalValue::Unsigned(0),
            CanonicalValue::Unsigned(u64::from(index)),
        ]),
        InternalPatternInput::Boundary(index) => CanonicalValue::Array(vec![
            CanonicalValue::Unsigned(1),
            CanonicalValue::Unsigned(u64::from(index)),
        ]),
    }
}

const fn verifier_proof_type_code(value: ProofValueType) -> u64 {
    match value {
        ProofValueType::Boolean => 0,
        ProofValueType::Integer => 1,
        ProofValueType::Atom => 2,
        ProofValueType::Entity => 3,
        ProofValueType::EntitySet => 4,
        ProofValueType::Relation => 5,
        ProofValueType::Action => 6,
        ProofValueType::Effect => 7,
        ProofValueType::Mechanism => 8,
    }
}

const fn verifier_evidence_type(value_type: EvidenceValueType) -> ProofValueType {
    match value_type {
        EvidenceValueType::Boolean => ProofValueType::Boolean,
        EvidenceValueType::Integer => ProofValueType::Integer,
        EvidenceValueType::Atom => ProofValueType::Atom,
        EvidenceValueType::Entity => ProofValueType::Entity,
        EvidenceValueType::EntitySet => ProofValueType::EntitySet,
        EvidenceValueType::Relation => ProofValueType::Relation,
        EvidenceValueType::Action => ProofValueType::Action,
        EvidenceValueType::Effect => ProofValueType::Effect,
        EvidenceValueType::Mechanism => ProofValueType::Mechanism,
    }
}

fn verifier_ancestral_subgraph(
    graph: &ProofGraph,
) -> Result<Vec<ProofNode>, CompressionVerifyError> {
    let output = usize::try_from(graph.output).map_err(|_| CompressionVerifyError::Capacity)?;
    if graph.nodes.is_empty() || output >= graph.nodes.len() {
        return Err(CompressionVerifyError::InvalidOutput);
    }
    for (index, node) in graph.nodes.iter().enumerate() {
        if node.operation.is_empty()
            || node
                .inputs
                .iter()
                .any(|input| usize::try_from(*input).map_or(true, |value| value >= index))
        {
            return Err(CompressionVerifyError::InvalidTopology);
        }
    }
    let mut ancestors = BTreeSet::from([graph.output]);
    let mut pending = vec![graph.output];
    while let Some(node_id) = pending.pop() {
        let index = usize::try_from(node_id).map_err(|_| CompressionVerifyError::Capacity)?;
        for input in &graph.nodes[index].inputs {
            if ancestors.insert(*input) {
                pending.push(*input);
            }
        }
    }
    let selected = ancestors.into_iter().collect::<Vec<_>>();
    let remap = selected
        .iter()
        .enumerate()
        .map(|(new, old)| {
            Ok((
                *old,
                u32::try_from(new).map_err(|_| CompressionVerifyError::Capacity)?,
            ))
        })
        .collect::<Result<std::collections::BTreeMap<_, _>, CompressionVerifyError>>()?;
    selected
        .iter()
        .map(|old| {
            let index = usize::try_from(*old).map_err(|_| CompressionVerifyError::Capacity)?;
            let mut node = graph.nodes[index].clone();
            node.inputs = node.inputs.iter().map(|input| remap[input]).collect();
            Ok(node)
        })
        .collect()
}

fn verifier_instantiate(
    pattern: &[GraphPatternNode],
    bindings: &EpisodeBindings,
) -> Result<Vec<ProofNode>, CompressionVerifyError> {
    pattern
        .iter()
        .map(|node| {
            let arguments = node
                .arguments
                .iter()
                .map(|argument| match argument {
                    GraphPatternArgument::Fixed(value) => Ok(*value),
                    GraphPatternArgument::Parameter(index) => bindings
                        .values
                        .get(
                            usize::try_from(*index)
                                .map_err(|_| CompressionVerifyError::Capacity)?,
                        )
                        .copied()
                        .ok_or(CompressionVerifyError::MissingBinding),
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(ProofNode {
                operation: node.operation.clone(),
                output_type: node.output_type,
                inputs: node.inputs.clone(),
                arguments,
            })
        })
        .collect()
}

fn graph_units(nodes: &[ProofNode]) -> Result<u64, CompressionVerifyError> {
    nodes.iter().try_fold(0_u64, |total, node| {
        total
            .checked_add(1)
            .and_then(|value| value.checked_add(u64::try_from(node.arguments.len()).ok()?))
            .ok_or(CompressionVerifyError::Capacity)
    })
}

fn compressed_units(
    pattern: &[GraphPatternNode],
    bindings: &[EpisodeBindings],
) -> Result<u64, CompressionVerifyError> {
    let fixed_arguments = pattern.iter().try_fold(0_u64, |total, node| {
        let count = node
            .arguments
            .iter()
            .filter(|argument| matches!(argument, GraphPatternArgument::Fixed(_)))
            .count();
        let count = u64::try_from(count).map_err(|_| CompressionVerifyError::Capacity)?;
        total
            .checked_add(count)
            .ok_or(CompressionVerifyError::Capacity)
    })?;
    let parameter_count = bindings.first().map_or(0, |binding| binding.values.len());
    if bindings
        .iter()
        .any(|binding| binding.values.len() != parameter_count)
    {
        return Err(CompressionVerifyError::MissingBinding);
    }
    let parameters =
        u64::try_from(parameter_count).map_err(|_| CompressionVerifyError::Capacity)?;
    let episodes = u64::try_from(bindings.len()).map_err(|_| CompressionVerifyError::Capacity)?;
    u64::try_from(pattern.len())
        .map_err(|_| CompressionVerifyError::Capacity)?
        .checked_add(fixed_arguments)
        .and_then(|value| value.checked_add(parameters))
        .and_then(|value| value.checked_add(parameters.checked_mul(episodes)?))
        .ok_or(CompressionVerifyError::Capacity)
}

/// Opaque proof that an independently reconstructed canonical hidden-world
/// schema equals the learner's proposal.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedLatentSchemaCompilation {
    schema: Hash32,
    evidence: Hash32,
    replayed_rules: u64,
}

impl VerifiedLatentSchemaCompilation {
    #[must_use]
    pub const fn schema(&self) -> Hash32 {
        self.schema
    }

    #[must_use]
    pub const fn evidence(&self) -> Hash32 {
        self.evidence
    }

    #[must_use]
    pub const fn replayed_rules(&self) -> u64 {
        self.replayed_rules
    }
}

/// Opaque proof that a schema was rebound and replayed against a current,
/// source-independent visible experience.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedLatentSchemaTransfer {
    schema: Hash32,
    experience: Hash32,
    action_roles: BTreeMap<ActionId, u16>,
    replayed_transitions: u64,
}

impl VerifiedLatentSchemaTransfer {
    #[must_use]
    pub const fn schema(&self) -> Hash32 {
        self.schema
    }

    #[must_use]
    pub const fn experience(&self) -> Hash32 {
        self.experience
    }

    #[must_use]
    pub const fn action_roles(&self) -> &BTreeMap<ActionId, u16> {
        &self.action_roles
    }

    #[must_use]
    pub const fn replayed_transitions(&self) -> u64 {
        self.replayed_transitions
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LatentSchemaVerifyError {
    EmptyTheory,
    IncompleteTheory,
    UnresolvedTheory,
    DimensionMismatch,
    SchemaMismatch,
    InvalidBinding,
    ReplayMismatch,
    Canonicalization,
    Capacity,
}

/// Separate reference implementation for hidden-world schema compilation and
/// current-experience transfer. It does not call the producer compiler.
#[derive(Clone, Copy, Debug, Default)]
pub struct LatentSchemaVerifier;

impl LatentSchemaVerifier {
    /// Independently alpha-normalize a complete theory and compare every
    /// canonical schema field and semantic hash.
    ///
    /// # Errors
    ///
    /// Rejects incomplete or unresolved theories, canonicalization capacity,
    /// and any difference from the proposed schema.
    pub fn verify_compilation(
        &self,
        theory: &LatentTransitionTheory,
        proposed: &LatentMechanismSchema,
        evidence: Hash32,
        maximum_permutations: u64,
    ) -> Result<VerifiedLatentSchemaCompilation, LatentSchemaVerifyError> {
        let latent_states = theory.latent_states();
        let source_rules = theory.rules().cloned().collect::<Vec<_>>();
        if latent_states == 0 || source_rules.is_empty() {
            return Err(LatentSchemaVerifyError::EmptyTheory);
        }
        if theory.unresolved_successors() != 0 {
            return Err(LatentSchemaVerifyError::UnresolvedTheory);
        }
        let actions = source_rules
            .iter()
            .map(|rule| rule.action)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        if source_rules.len()
            != usize::from(latent_states)
                .checked_mul(actions.len())
                .ok_or(LatentSchemaVerifyError::Capacity)?
        {
            return Err(LatentSchemaVerifyError::IncompleteTheory);
        }
        let by_key = source_rules
            .iter()
            .map(|rule| ((rule.latent, rule.action), rule))
            .collect::<BTreeMap<_, _>>();
        let dimensions = u16::try_from(theory.space().dimensions())
            .map_err(|_| LatentSchemaVerifyError::Capacity)?;
        let mut best: Option<(Vec<u8>, Vec<LatentActionRole>)> = None;
        for relabeling in verifier_latent_permutations(latent_states, maximum_permutations)? {
            let mut canonical_roles = Vec::with_capacity(actions.len());
            let mut new_to_old = vec![0_u8; usize::from(latent_states)];
            for (old, new) in relabeling.iter().copied().enumerate() {
                new_to_old[usize::from(new)] =
                    u8::try_from(old).map_err(|_| LatentSchemaVerifyError::Capacity)?;
            }
            for action in &actions {
                let role_rules = new_to_old
                    .iter()
                    .map(|old| {
                        let rule = by_key
                            .get(&(*old, *action))
                            .ok_or(LatentSchemaVerifyError::IncompleteTheory)?;
                        Ok(LatentSchemaRule {
                            next_latent: *relabeling
                                .get(usize::from(rule.next_latent))
                                .ok_or(LatentSchemaVerifyError::IncompleteTheory)?,
                            delta: rule.delta.clone(),
                        })
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                canonical_roles.push(LatentActionRole { rules: role_rules });
            }
            canonical_roles.sort_unstable();
            let bytes = verifier_schema_value(dimensions, latent_states, &canonical_roles)
                .encode_canonical()
                .map_err(|_| LatentSchemaVerifyError::Canonicalization)?;
            if best.as_ref().is_none_or(|(current, _)| bytes < *current) {
                best = Some((bytes, canonical_roles));
            }
        }
        let (bytes, canonical_roles) = best.ok_or(LatentSchemaVerifyError::Capacity)?;
        if proposed.dimensions() != dimensions
            || proposed.latent_states() != latent_states
            || proposed.roles() != canonical_roles
            || proposed.semantic_hash() != Hash32::digest(&bytes)
        {
            return Err(LatentSchemaVerifyError::SchemaMismatch);
        }
        Ok(VerifiedLatentSchemaCompilation {
            schema: proposed.semantic_hash(),
            evidence,
            replayed_rules: u64::try_from(source_rules.len())
                .map_err(|_| LatentSchemaVerifyError::Capacity)?,
        })
    }

    /// Independently execute a claimed action-role binding against every
    /// visible transition and verify the constructed theory's full rule set.
    ///
    /// # Errors
    ///
    /// Rejects incompatible dimensions, non-bijections, malformed traces,
    /// arithmetic overflow, or any replay/theory mismatch.
    #[allow(clippy::too_many_lines)] // One linear reference replay keeps the trust path auditable.
    pub fn verify_transfer(
        &self,
        schema: &LatentMechanismSchema,
        space: &StateSpace,
        actions: &[ActionId],
        episodes: &[LatentEpisode],
        binding: &LatentSchemaBinding,
    ) -> Result<VerifiedLatentSchemaTransfer, LatentSchemaVerifyError> {
        if usize::from(schema.dimensions()) != space.dimensions() {
            return Err(LatentSchemaVerifyError::DimensionMismatch);
        }
        let action_set = actions.iter().copied().collect::<BTreeSet<_>>();
        let role_set = binding
            .action_roles
            .values()
            .copied()
            .collect::<BTreeSet<_>>();
        if action_set.len() != actions.len()
            || binding
                .action_roles
                .keys()
                .copied()
                .collect::<BTreeSet<_>>()
                != action_set
            || role_set.len() != schema.roles().len()
            || role_set
                .iter()
                .copied()
                .ne(0..u16::try_from(schema.roles().len())
                    .map_err(|_| LatentSchemaVerifyError::Capacity)?)
        {
            return Err(LatentSchemaVerifyError::InvalidBinding);
        }
        let mut support = BTreeMap::<(u8, ActionId), u32>::new();
        let mut replayed = 0_u64;
        for episode in episodes {
            let mut latent = 0_u8;
            let mut prior: Option<&WorldState> = None;
            for transition in &episode.transitions {
                if !space.contains(&transition.before)
                    || !space.contains(&transition.after)
                    || prior.is_some_and(|after| after != &transition.before)
                {
                    return Err(LatentSchemaVerifyError::ReplayMismatch);
                }
                let action_schema = binding
                    .action_roles
                    .get(&transition.action)
                    .and_then(|role| schema.roles().get(usize::from(*role)))
                    .ok_or(LatentSchemaVerifyError::InvalidBinding)?;
                let transition_rule = action_schema
                    .rules
                    .get(usize::from(latent))
                    .ok_or(LatentSchemaVerifyError::InvalidBinding)?;
                if verifier_apply_delta(space, &transition.before, &transition_rule.delta)?
                    != transition.after
                {
                    return Err(LatentSchemaVerifyError::ReplayMismatch);
                }
                let entry = support.entry((latent, transition.action)).or_default();
                *entry = entry
                    .checked_add(1)
                    .ok_or(LatentSchemaVerifyError::Capacity)?;
                latent = transition_rule.next_latent;
                prior = Some(&transition.after);
                replayed = replayed
                    .checked_add(1)
                    .ok_or(LatentSchemaVerifyError::Capacity)?;
            }
        }
        if replayed != binding.replayed_transitions
            || binding.theory.space() != space
            || binding.theory.latent_states() != schema.latent_states()
            || binding.theory.unresolved_successors() != 0
        {
            return Err(LatentSchemaVerifyError::ReplayMismatch);
        }
        let actual = binding
            .theory
            .rules()
            .map(|rule| {
                (
                    (rule.latent, rule.action),
                    (rule.next_latent, rule.delta.clone(), rule.support),
                )
            })
            .collect::<BTreeMap<_, _>>();
        let expected = binding
            .action_roles
            .iter()
            .flat_map(|(action, role)| {
                schema.roles()[usize::from(*role)]
                    .rules
                    .iter()
                    .enumerate()
                    .map(move |(latent, rule)| (*action, latent, rule))
            })
            .map(|(action, latent, rule)| {
                let latent = u8::try_from(latent).map_err(|_| LatentSchemaVerifyError::Capacity)?;
                Ok((
                    (latent, action),
                    (
                        rule.next_latent,
                        rule.delta.clone(),
                        support.get(&(latent, action)).copied().unwrap_or_default(),
                    ),
                ))
            })
            .collect::<Result<BTreeMap<_, _>, LatentSchemaVerifyError>>()?;
        if actual != expected {
            return Err(LatentSchemaVerifyError::ReplayMismatch);
        }
        let experience = verifier_experience_value(space, actions, episodes)
            .encode_canonical()
            .map_err(|_| LatentSchemaVerifyError::Canonicalization)?;
        Ok(VerifiedLatentSchemaTransfer {
            schema: schema.semantic_hash(),
            experience: Hash32::digest(&experience),
            action_roles: binding.action_roles.clone(),
            replayed_transitions: replayed,
        })
    }
}

fn verifier_latent_permutations(
    latent_states: u8,
    maximum: u64,
) -> Result<Vec<Vec<u8>>, LatentSchemaVerifyError> {
    fn extend(
        remaining: &mut Vec<usize>,
        prefix: &mut Vec<usize>,
        output: &mut Vec<Vec<usize>>,
        maximum: u64,
    ) -> Result<(), LatentSchemaVerifyError> {
        if remaining.is_empty() {
            if u64::try_from(output.len()).map_err(|_| LatentSchemaVerifyError::Capacity)?
                >= maximum
            {
                return Err(LatentSchemaVerifyError::Capacity);
            }
            output.push(prefix.clone());
            return Ok(());
        }
        for index in 0..remaining.len() {
            let value = remaining.remove(index);
            prefix.push(value);
            extend(remaining, prefix, output, maximum)?;
            prefix.pop();
            remaining.insert(index, value);
        }
        Ok(())
    }
    if maximum == 0 {
        return Err(LatentSchemaVerifyError::Capacity);
    }
    let mut raw = Vec::new();
    extend(
        &mut (1..latent_states).map(usize::from).collect(),
        &mut Vec::new(),
        &mut raw,
        maximum,
    )?;
    raw.into_iter()
        .map(|order| {
            let mut map = vec![0_u8; usize::from(latent_states)];
            for (new, old) in order.into_iter().enumerate() {
                map[old] = u8::try_from(new + 1).map_err(|_| LatentSchemaVerifyError::Capacity)?;
            }
            Ok(map)
        })
        .collect()
}

fn verifier_schema_value(
    dimensions: u16,
    latent_states: u8,
    roles: &[LatentActionRole],
) -> CanonicalValue {
    CanonicalValue::Array(vec![
        CanonicalValue::Text("cairn.latent-mechanism-schema.v1".to_owned()),
        CanonicalValue::Unsigned(u64::from(dimensions)),
        CanonicalValue::Unsigned(u64::from(latent_states)),
        CanonicalValue::Array(
            roles
                .iter()
                .map(|role| {
                    CanonicalValue::Array(
                        role.rules
                            .iter()
                            .map(|rule| {
                                CanonicalValue::Array(vec![
                                    CanonicalValue::Unsigned(u64::from(rule.next_latent)),
                                    CanonicalValue::Array(
                                        rule.delta
                                            .iter()
                                            .map(|value| {
                                                if *value >= 0 {
                                                    CanonicalValue::Unsigned(value.unsigned_abs())
                                                } else {
                                                    CanonicalValue::Negative(*value)
                                                }
                                            })
                                            .collect(),
                                    ),
                                ])
                            })
                            .collect(),
                    )
                })
                .collect(),
        ),
    ])
}

fn verifier_apply_delta(
    space: &StateSpace,
    before: &WorldState,
    delta: &[i64],
) -> Result<WorldState, LatentSchemaVerifyError> {
    if before.0.len() != delta.len() {
        return Err(LatentSchemaVerifyError::ReplayMismatch);
    }
    before
        .0
        .iter()
        .zip(delta)
        .zip(space.lower().iter().zip(space.upper()))
        .map(|((value, delta), (lower, upper))| {
            value
                .checked_add(*delta)
                .map(|next| next.clamp(*lower, *upper))
                .ok_or(LatentSchemaVerifyError::Capacity)
        })
        .collect::<Result<Vec<_>, _>>()
        .map(WorldState)
}

fn verifier_experience_value(
    space: &StateSpace,
    actions: &[ActionId],
    episodes: &[LatentEpisode],
) -> CanonicalValue {
    let ordered_actions = actions.iter().copied().collect::<BTreeSet<_>>();
    CanonicalValue::Array(vec![
        CanonicalValue::Text("cairn.latent-schema-experience.v1".to_owned()),
        CanonicalValue::Array(
            space
                .lower()
                .iter()
                .map(|value| verifier_integer(*value))
                .collect(),
        ),
        CanonicalValue::Array(
            space
                .upper()
                .iter()
                .map(|value| verifier_integer(*value))
                .collect(),
        ),
        CanonicalValue::Array(
            ordered_actions
                .iter()
                .map(|action| CanonicalValue::Unsigned(u64::from(action.0)))
                .collect(),
        ),
        CanonicalValue::Array(
            episodes
                .iter()
                .map(|episode| {
                    CanonicalValue::Array(
                        episode
                            .transitions
                            .iter()
                            .map(|transition| {
                                CanonicalValue::Array(vec![
                                    CanonicalValue::Array(
                                        transition
                                            .before
                                            .0
                                            .iter()
                                            .map(|value| verifier_integer(*value))
                                            .collect(),
                                    ),
                                    CanonicalValue::Unsigned(u64::from(transition.action.0)),
                                    CanonicalValue::Array(
                                        transition
                                            .after
                                            .0
                                            .iter()
                                            .map(|value| verifier_integer(*value))
                                            .collect(),
                                    ),
                                ])
                            })
                            .collect(),
                    )
                })
                .collect(),
        ),
    ])
}

fn verifier_integer(value: i64) -> CanonicalValue {
    if value >= 0 {
        CanonicalValue::Unsigned(value.unsigned_abs())
    } else {
        CanonicalValue::Negative(value)
    }
}
