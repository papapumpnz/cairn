//! Independent promotion boundary combining replay provenance and learning gates.

use cairn_consolidator::{
    EpisodeBindings, GraphCompressionProposal, GraphPatternArgument, GraphPatternNode,
    InternalEpisodeBindings, InternalGraphCompressionProposal, InternalGraphPatternNode,
    InternalPatternInput, ProofDatum, ProofGraph, ProofNode, ProofValueType,
};
use cairn_core::{CanonicalEncode, CanonicalValue, CollectionProcedureProblem, Hash32};
use cairn_mechanism::{
    admit_persistent, AdmissionError, InventionEvidence, Mechanism, MechanismStatus,
};
pub use cairn_proof::{IndependentVerifier, ReplayVerifier, VerifyError};
use cairn_rkl::{
    CheckedTheory, ReferenceTheoryExecutor, RuntimeEnvironment, TheoryExecLimits,
    TheoryExecutionRequest, Type, TypeEnvironment, Value,
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
    NoCompression,
    Capacity,
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
