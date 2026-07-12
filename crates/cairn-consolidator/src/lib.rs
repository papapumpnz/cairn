//! Offline proof-subgraph compression separated from online task solving.

use cairn_cas::{Cas, CasError, StoredArtifact};
use cairn_core::{
    ArtifactEnvelope, ArtifactError, ArtifactId, ArtifactKind, BlobId, BudgetRecord,
    CanonicalEncode, CanonicalError, CanonicalLimits, CanonicalValue, EvidenceElementId,
    EvidenceValueType, Hash32, ProducerRecord, RelationalObservation, RelationalObservationLimits,
    SchemaDescriptor, SchemaRef, SchemaRegistry, SemanticsVersion,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

pub const CONSOLIDATOR_SEMANTICS_VERSION: &str = "2.3.0";
pub const PROOF_GRAPH_SCHEMA: &str = "cairn.consolidator.proof-graph";
pub const PROOF_GRAPH_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ConsolidationLimits {
    pub maximum_episodes: usize,
    pub maximum_nodes_per_episode: usize,
    pub minimum_independent_episodes: usize,
}

impl Default for ConsolidationLimits {
    fn default() -> Self {
        Self {
            maximum_episodes: 10_000,
            maximum_nodes_per_episode: 10_000,
            minimum_independent_episodes: 2,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ConsolidationMetrics {
    pub episodes_compared: u64,
    pub structural_cohorts: u64,
    pub proposals_created: u64,
    pub cohorts_without_compression: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConsolidationReport {
    pub proposals: Vec<GraphCompressionProposal>,
    pub metrics: ConsolidationMetrics,
}

/// Separate offline runtime. Online task learning may submit immutable proof
/// graphs, but only this runtime compares them and proposes persistent-library
/// candidates.
#[derive(Clone, Debug)]
pub struct ConsolidationRuntime {
    limits: ConsolidationLimits,
    episodes: BTreeMap<Hash32, ProofGraph>,
}

impl ConsolidationRuntime {
    #[must_use]
    pub const fn new(limits: ConsolidationLimits) -> Self {
        Self {
            limits,
            episodes: BTreeMap::new(),
        }
    }

    /// Admit one independently identified, structurally valid proof graph.
    ///
    /// # Errors
    ///
    /// Rejects duplicates, invalid DAGs, and configured resource limits.
    pub fn submit(&mut self, graph: ProofGraph) -> Result<(), ConsolidationError> {
        if self.episodes.contains_key(&graph.episode) {
            return Err(ConsolidationError::DuplicateEpisode);
        }
        if self.episodes.len() >= self.limits.maximum_episodes
            || graph.nodes.len() > self.limits.maximum_nodes_per_episode
        {
            return Err(ConsolidationError::Capacity);
        }
        normalize_ancestral_subgraph(&graph)?;
        self.episodes.insert(graph.episode, graph);
        Ok(())
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.episodes.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.episodes.is_empty()
    }

    /// Discover structural cohorts without task-family labels and attempt
    /// exact compression for each sufficiently supported cohort.
    ///
    /// # Errors
    ///
    /// Returns canonicalization or capacity failures. Ordinary cohorts that
    /// cannot compress are measured and skipped.
    pub fn consolidate(&self) -> Result<ConsolidationReport, ConsolidationError> {
        let mut cohorts: BTreeMap<Hash32, Vec<ProofGraph>> = BTreeMap::new();
        for graph in self.episodes.values() {
            let normalized = normalize_ancestral_subgraph(graph)?;
            cohorts
                .entry(graph_shape_hash(&normalized)?)
                .or_default()
                .push(graph.clone());
        }
        let mut report = ConsolidationReport {
            proposals: Vec::new(),
            metrics: ConsolidationMetrics {
                episodes_compared: u64::try_from(self.episodes.len())
                    .map_err(|_| ConsolidationError::Capacity)?,
                structural_cohorts: u64::try_from(cohorts.len())
                    .map_err(|_| ConsolidationError::Capacity)?,
                ..ConsolidationMetrics::default()
            },
        };
        for graphs in cohorts
            .into_values()
            .filter(|graphs| graphs.len() >= self.limits.minimum_independent_episodes.max(2))
        {
            match Consolidator.compress_proof_subgraphs(&graphs) {
                Ok(proposal) => report.proposals.push(proposal),
                Err(ConsolidationError::NoCompression) => {
                    report.metrics.cohorts_without_compression += 1;
                }
                Err(error) => return Err(error),
            }
        }
        report.metrics.proposals_created =
            u64::try_from(report.proposals.len()).map_err(|_| ConsolidationError::Capacity)?;
        Ok(report)
    }
}

/// A domain-neutral type attached to an opaque, canonical proof argument.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ProofValueType {
    Boolean,
    Integer,
    Atom,
    Entity,
    EntitySet,
    Relation,
    Action,
    Effect,
    Mechanism,
}

/// Content-addressed proof data. Consolidation can compare and rebind values
/// without retaining a complete source observation or answer.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ProofDatum {
    pub ty: ProofValueType,
    pub value: Hash32,
}

impl ProofDatum {
    #[must_use]
    pub fn digest(ty: ProofValueType, canonical_value: &[u8]) -> Self {
        Self {
            ty,
            value: Hash32::digest(canonical_value),
        }
    }
}

/// One topologically ordered node in an immutable execution or proof DAG.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProofNode {
    pub operation: String,
    pub output_type: ProofValueType,
    pub inputs: Vec<u32>,
    pub arguments: Vec<ProofDatum>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProofGraph {
    pub episode: Hash32,
    pub nodes: Vec<ProofNode>,
    pub output: u32,
}

/// Evidence-only proof prefix supplied by a frontend or curriculum
/// materializer. It contains no completed mechanism occurrence or result node.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProofFrontier {
    pub episode: Hash32,
    pub nodes: Vec<ProofNode>,
}

/// Upper bound for deterministic observation-to-frontier lowering.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ObservationFrontierLimits {
    pub observation: RelationalObservationLimits,
    pub maximum_frontier_nodes: u32,
}

impl Default for ObservationFrontierLimits {
    fn default() -> Self {
        Self {
            observation: RelationalObservationLimits::default(),
            maximum_frontier_nodes: 1_000_000,
        }
    }
}

/// Exact source of one lowered proof-frontier node.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ObservationFrontierProvenance {
    Observation,
    Element(EvidenceElementId),
    Relation {
        predicate: Hash32,
        participants: Vec<EvidenceElementId>,
    },
}

/// Untrusted deterministic lowering of generic relational observation evidence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoweredObservationFrontier {
    pub frontier: ProofFrontier,
    pub provenance: Vec<ObservationFrontierProvenance>,
    pub commitment: Hash32,
}

impl CanonicalEncode for ProofGraph {
    fn encode_canonical(&self) -> Result<Vec<u8>, CanonicalError> {
        proof_graph_value(self).encode_canonical()
    }
}

impl ProofGraph {
    /// Decode and structurally validate one complete canonical proof DAG.
    ///
    /// # Errors
    ///
    /// Rejects non-canonical bytes, resource-limit violations, unknown type
    /// codes, malformed hashes, invalid topology, and unreachable output.
    pub fn decode_canonical(
        bytes: &[u8],
        limits: CanonicalLimits,
    ) -> Result<Self, ProofGraphDecodeError> {
        let value = cairn_core::decode_canonical(bytes, limits)
            .map_err(ProofGraphDecodeError::Canonical)?;
        Self::from_canonical_value(value)
    }

    fn from_canonical_value(value: CanonicalValue) -> Result<Self, ProofGraphDecodeError> {
        let CanonicalValue::Array(values) = value else {
            return Err(ProofGraphDecodeError::InvalidShape("root is not an array"));
        };
        let [marker, episode, nodes, output] = values.as_slice() else {
            return Err(ProofGraphDecodeError::InvalidShape(
                "root field count is not four",
            ));
        };
        if marker != &CanonicalValue::Text("cairn.consolidator.proof-graph.v1".to_owned()) {
            return Err(ProofGraphDecodeError::InvalidShape("marker mismatch"));
        }
        let CanonicalValue::Array(nodes) = nodes else {
            return Err(ProofGraphDecodeError::InvalidShape("nodes is not an array"));
        };
        let graph = Self {
            episode: decode_hash(episode)?,
            nodes: nodes
                .iter()
                .map(decode_proof_node)
                .collect::<Result<Vec<_>, _>>()?,
            output: decode_u32(output)?,
        };
        normalize_ancestral_subgraph(&graph).map_err(ProofGraphDecodeError::InvalidGraph)?;
        Ok(graph)
    }
}

/// Non-semantic envelope metadata for a persisted proof DAG. Parent IDs must
/// identify the observations, execution trace, or proof certificate from which
/// the graph was derived.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProofGraphArtifactMetadata {
    pub parent_ids: Vec<ArtifactId>,
    pub producer: ProducerRecord,
    pub budget: BudgetRecord,
    pub created_at_logical: u64,
}

/// Canonical CAS boundary for immutable proof DAGs submitted to consolidation.
#[derive(Clone, Copy, Debug, Default)]
pub struct ProofGraphArchive;

impl ProofGraphArchive {
    /// Validate and persist one proof graph as an immutable artifact.
    ///
    /// # Errors
    ///
    /// Rejects absent lineage, malformed graphs, invalid metadata, capacity,
    /// canonicalization, storage, and integrity failures.
    pub fn persist(
        &self,
        cas: &Cas,
        graph: &ProofGraph,
        metadata: ProofGraphArtifactMetadata,
    ) -> Result<StoredArtifact, ProofGraphArchiveError> {
        if metadata.parent_ids.is_empty() {
            return Err(ProofGraphArchiveError::MissingParent);
        }
        normalize_ancestral_subgraph(graph).map_err(ProofGraphArchiveError::InvalidGraph)?;
        let envelope = ArtifactEnvelope::new(
            proof_graph_schema()?,
            proof_graph_kind()?,
            proof_graph_semantics(),
            metadata.parent_ids,
            Some(graph.episode),
            metadata.producer,
            metadata.budget,
            metadata.created_at_logical,
            graph.clone(),
        )?;
        cas.put_envelope(&envelope)
            .map_err(ProofGraphArchiveError::Storage)
    }

    /// Load an untrusted proof graph, validate its canonical envelope, and
    /// recheck the complete DAG before returning it to the consolidator.
    ///
    /// # Errors
    ///
    /// Rejects storage corruption, unsupported schema/semantics, missing
    /// lineage, scope mismatch, malformed nodes, and invalid topology.
    pub fn load(&self, cas: &Cas, blob_id: BlobId) -> Result<ProofGraph, ProofGraphArchiveError> {
        let mut registry = SchemaRegistry::new();
        registry.register(SchemaDescriptor {
            name: PROOF_GRAPH_SCHEMA.to_owned(),
            current_version: PROOF_GRAPH_SCHEMA_VERSION,
            additive_read_versions: BTreeSet::new(),
            maximum_payload_bytes: 8 * 1024 * 1024,
        })?;
        let envelope = cas.get_envelope(
            blob_id,
            &registry,
            &BTreeSet::from([proof_graph_semantics()]),
            CanonicalLimits {
                max_input_bytes: 9 * 1024 * 1024,
                max_depth: 32,
                max_collection_items: 300_000,
                max_scalar_bytes: 8 * 1024 * 1024,
            },
        )?;
        if envelope.artifact_kind != proof_graph_kind()? || envelope.parent_ids.is_empty() {
            return Err(ProofGraphArchiveError::InvalidEnvelope);
        }
        let graph = ProofGraph::from_canonical_value(envelope.payload)?;
        if envelope.task_scope != Some(graph.episode) {
            return Err(ProofGraphArchiveError::InvalidEnvelope);
        }
        Ok(graph)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProofGraphDecodeError {
    Canonical(CanonicalError),
    InvalidShape(&'static str),
    InvalidHash,
    UnknownType(u64),
    IntegerOverflow,
    InvalidGraph(ConsolidationError),
}

impl fmt::Display for ProofGraphDecodeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for ProofGraphDecodeError {}

#[derive(Debug)]
pub enum ProofGraphArchiveError {
    MissingParent,
    InvalidEnvelope,
    InvalidGraph(ConsolidationError),
    Decode(ProofGraphDecodeError),
    Artifact(ArtifactError),
    Storage(CasError),
}

impl fmt::Display for ProofGraphArchiveError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Decode(error) => error.fmt(formatter),
            Self::Artifact(error) => error.fmt(formatter),
            Self::Storage(error) => error.fmt(formatter),
            other => write!(formatter, "{other:?}"),
        }
    }
}

impl std::error::Error for ProofGraphArchiveError {}

impl From<ProofGraphDecodeError> for ProofGraphArchiveError {
    fn from(value: ProofGraphDecodeError) -> Self {
        Self::Decode(value)
    }
}

impl From<ArtifactError> for ProofGraphArchiveError {
    fn from(value: ArtifactError) -> Self {
        Self::Artifact(value)
    }
}

impl From<CasError> for ProofGraphArchiveError {
    fn from(value: CasError) -> Self {
        Self::Storage(value)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GraphPatternArgument {
    Fixed(ProofDatum),
    Parameter(u32),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GraphPatternNode {
    pub operation: String,
    pub output_type: ProofValueType,
    pub inputs: Vec<u32>,
    pub arguments: Vec<GraphPatternArgument>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EpisodeBindings {
    pub episode: Hash32,
    pub values: Vec<ProofDatum>,
}

/// A compression proposal that is executable in the deliberately small sense
/// required for admission: every source subgraph must be exactly reconstructible
/// from this pattern and its bindings.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GraphCompressionProposal {
    pattern: Vec<GraphPatternNode>,
    output: u32,
    bindings: Vec<EpisodeBindings>,
    source_episodes: Vec<Hash32>,
    uncompressed_nodes: u64,
    compressed_nodes: u64,
    verified_replays: u64,
    semantic_hash: Hash32,
}

impl GraphCompressionProposal {
    #[must_use]
    pub fn pattern(&self) -> &[GraphPatternNode] {
        &self.pattern
    }

    #[must_use]
    pub const fn output(&self) -> u32 {
        self.output
    }

    #[must_use]
    pub fn bindings(&self) -> &[EpisodeBindings] {
        &self.bindings
    }

    #[must_use]
    pub fn source_episodes(&self) -> &[Hash32] {
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
    pub const fn verified_replays(&self) -> u64 {
        self.verified_replays
    }

    #[must_use]
    pub const fn semantic_hash(&self) -> Hash32 {
        self.semantic_hash
    }
}

/// Explicit input edge in a mined internal proof mechanism. Boundary inputs
/// are parameters supplied by the surrounding host proof rather than hidden
/// dependencies on source-node identifiers.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum InternalPatternInput {
    Internal(u32),
    Boundary(u32),
}

/// One node in a boundary-aware internal proof mechanism.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InternalGraphPatternNode {
    pub operation: String,
    pub output_type: ProofValueType,
    pub inputs: Vec<InternalPatternInput>,
    pub arguments: Vec<GraphPatternArgument>,
}

/// Exact occurrence binding for one host proof. Selected and boundary node IDs
/// are source-local evidence and never enter the generalized mechanism body.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InternalEpisodeBindings {
    pub episode: Hash32,
    pub selected_nodes: Vec<u32>,
    pub boundary_nodes: Vec<u32>,
    pub values: Vec<ProofDatum>,
}

/// A repeated internal mechanism mined across independently identified host
/// proofs with different surrounding topology.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InternalGraphCompressionProposal {
    pattern: Vec<InternalGraphPatternNode>,
    boundary_types: Vec<ProofValueType>,
    output: u32,
    bindings: Vec<InternalEpisodeBindings>,
    source_episodes: Vec<Hash32>,
    uncompressed_nodes: u64,
    compressed_nodes: u64,
    verified_replays: u64,
    semantic_hash: Hash32,
}

impl InternalGraphCompressionProposal {
    #[must_use]
    pub fn pattern(&self) -> &[InternalGraphPatternNode] {
        &self.pattern
    }

    #[must_use]
    pub fn boundary_types(&self) -> &[ProofValueType] {
        &self.boundary_types
    }

    #[must_use]
    pub const fn output(&self) -> u32 {
        self.output
    }

    #[must_use]
    pub fn bindings(&self) -> &[InternalEpisodeBindings] {
        &self.bindings
    }

    #[must_use]
    pub fn source_episodes(&self) -> &[Hash32] {
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
    pub const fn verified_replays(&self) -> u64 {
        self.verified_replays
    }

    #[must_use]
    pub const fn semantic_hash(&self) -> Hash32 {
        self.semantic_hash
    }
}

/// Source-independent executable schema compiled from an internal compression
/// proposal. Its identity excludes episodes, occurrence bindings, and measured
/// costs so additional evidence cannot rename the learned mechanism.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompiledInternalMechanism {
    pattern: Vec<InternalGraphPatternNode>,
    boundary_types: Vec<ProofValueType>,
    parameter_types: Vec<ProofValueType>,
    output: u32,
    source_proposal: Hash32,
    semantic_hash: Hash32,
}

/// One occurrence selected by the runtime composition proposer. It is a
/// proposal only; the independent verifier must reconstruct the invocation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InternalMechanismInvocation {
    pub mechanism: Hash32,
    pub binding: InternalEpisodeBindings,
}

/// Deterministic proposal that several learned mechanisms jointly contribute
/// to one host proof result.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InternalCompositionPlan {
    pub episode: Hash32,
    pub invocations: Vec<InternalMechanismInvocation>,
    pub combinations_considered: u64,
}

/// Fixed-work limits for constructing proof candidates from an evidence-only
/// frontier.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InternalConstructionLimits {
    pub maximum_bindings_per_mechanism: u64,
    pub maximum_candidates: u64,
}

impl Default for InternalConstructionLimits {
    fn default() -> Self {
        Self {
            maximum_bindings_per_mechanism: 1_024,
            maximum_candidates: 10_000,
        }
    }
}

/// One untrusted proof candidate constructed by executable learned mechanisms.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConstructedInternalComposition {
    pub graph: ProofGraph,
    pub invocations: Vec<InternalMechanismInvocation>,
    pub candidates_considered: u64,
}

/// Task-local correspondence learned from one verified construction. Ordinals
/// refer to typed evidence order, never source values or node identities.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InternalConstructionPolicy {
    pub mechanisms: Vec<Hash32>,
    pub boundary_ordinals: Vec<Vec<u32>>,
    pub value_ordinals: Vec<Vec<u32>>,
}

/// Value-independent structural selector for one proof-frontier node.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FrontierNodeSelector {
    pub operation: String,
    pub output_type: ProofValueType,
    pub input_types: Vec<ProofValueType>,
    pub argument_types: Vec<ProofValueType>,
}

/// One typed value selected by its bounded relational neighbourhood rather
/// than source identity, literal value, or observation order.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RelationalValueSelector {
    pub value_type: ProofValueType,
    pub fingerprint: Hash32,
}

/// Task-local construction correspondence learned from relational structure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RelationalConstructionPolicy {
    pub mechanisms: Vec<Hash32>,
    pub boundary_selectors: Vec<Vec<FrontierNodeSelector>>,
    pub value_selectors: Vec<Vec<RelationalValueSelector>>,
}

impl CompiledInternalMechanism {
    #[must_use]
    pub fn pattern(&self) -> &[InternalGraphPatternNode] {
        &self.pattern
    }

    #[must_use]
    pub fn boundary_types(&self) -> &[ProofValueType] {
        &self.boundary_types
    }

    #[must_use]
    pub fn parameter_types(&self) -> &[ProofValueType] {
        &self.parameter_types
    }

    #[must_use]
    pub const fn output(&self) -> u32 {
        self.output
    }

    #[must_use]
    pub const fn source_proposal(&self) -> Hash32 {
        self.source_proposal
    }

    #[must_use]
    pub const fn semantic_hash(&self) -> Hash32 {
        self.semantic_hash
    }
}

impl CanonicalEncode for CompiledInternalMechanism {
    fn encode_canonical(&self) -> Result<Vec<u8>, CanonicalError> {
        internal_mechanism_value(
            &self.pattern,
            &self.boundary_types,
            &self.parameter_types,
            self.output,
        )
        .encode_canonical()
    }
}

/// Exhaustive reference-miner limits. Candidate windows are deterministic and
/// bounded before allocation or comparison.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InternalMiningLimits {
    pub minimum_nodes: usize,
    pub maximum_nodes: usize,
    pub minimum_independent_episodes: usize,
    pub maximum_windows_per_episode: usize,
}

impl Default for InternalMiningLimits {
    fn default() -> Self {
        Self {
            minimum_nodes: 3,
            maximum_nodes: 16,
            minimum_independent_episodes: 3,
            maximum_windows_per_episode: 10_000,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct InternalSourceNode {
    operation: String,
    output_type: ProofValueType,
    inputs: Vec<InternalPatternInput>,
    arguments: Vec<ProofDatum>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct InternalSourceSlice {
    episode: Hash32,
    nodes: Vec<InternalSourceNode>,
    boundary_types: Vec<ProofValueType>,
    selected_nodes: Vec<u32>,
    boundary_nodes: Vec<u32>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProofTrace {
    pub episode: Hash32,
    pub operations: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PatternElement {
    Fixed(String),
    Parameter(u32),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompressionProposal {
    pub pattern: Vec<PatternElement>,
    pub source_episodes: Vec<Hash32>,
    pub uncompressed_nodes: u64,
    pub compressed_nodes: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConsolidationError {
    InsufficientEpisodes,
    DuplicateEpisode,
    EmptyTrace,
    IncompatibleTraceShapes,
    NoSharedStructure,
    NoCompression,
    Capacity,
    InvalidOutput,
    InvalidTopology,
    IncompatibleGraphShapes,
    IncompatibleOperation,
    IncompatibleTypes,
    IncompatibleArguments,
    ReplayMismatch,
    Canonicalization,
    InvalidMiningLimits,
    NoInternalCompression,
    NoInternalMatch,
    DuplicateMechanism,
    NoInternalComposition,
    InvalidFrontier,
    NoConstructionCandidate,
    InvalidObservation,
    AmbiguousCorrespondence,
    MissingCorrespondence,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Consolidator;

impl Consolidator {
    /// Lower one validated modality-neutral relational observation to typed
    /// proof evidence with explicit per-node provenance. This path neither
    /// retrieves learned mechanisms nor consumes expected output.
    ///
    /// # Errors
    ///
    /// Rejects invalid observation identity/evidence, node capacity excess, or
    /// canonical commitment failure.
    pub fn lower_relational_observation(
        &self,
        observation: &RelationalObservation,
        limits: ObservationFrontierLimits,
    ) -> Result<LoweredObservationFrontier, ConsolidationError> {
        observation
            .verify(limits.observation)
            .map_err(|_| ConsolidationError::InvalidObservation)?;
        let node_count = 1_usize
            .checked_add(observation.elements.len())
            .and_then(|count| count.checked_add(observation.relations.len()))
            .ok_or(ConsolidationError::Capacity)?;
        if node_count
            > usize::try_from(limits.maximum_frontier_nodes)
                .map_err(|_| ConsolidationError::Capacity)?
        {
            return Err(ConsolidationError::Capacity);
        }
        let mut nodes = Vec::with_capacity(node_count);
        let mut provenance = Vec::with_capacity(node_count);
        nodes.push(ProofNode {
            operation: "observe-relational-evidence".to_owned(),
            output_type: ProofValueType::EntitySet,
            inputs: Vec::new(),
            arguments: vec![ProofDatum {
                ty: ProofValueType::Atom,
                value: observation.observation.id.0,
            }],
        });
        provenance.push(ObservationFrontierProvenance::Observation);
        let mut element_nodes = BTreeMap::new();
        for element in &observation.elements {
            let node = u32::try_from(nodes.len()).map_err(|_| ConsolidationError::Capacity)?;
            element_nodes.insert(element.id, node);
            let value_type = proof_value_type(element.value_type);
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
                        .ok_or(ConsolidationError::InvalidObservation)
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
        let frontier = ProofFrontier {
            episode: observation.observation.id.0,
            nodes,
        };
        let commitment = observation_frontier_commitment(&frontier, &provenance)?;
        Ok(LoweredObservationFrontier {
            frontier,
            provenance,
            commitment,
        })
    }

    /// Anti-unify the complete ancestral subgraph of each selected output.
    /// Unrelated nodes outside that subgraph are ignored. Operations, types,
    /// and dependency topology remain fixed; varying typed arguments become
    /// parameters, shared variation is represented by one parameter, and the
    /// result is replayed against every independent source before return.
    ///
    /// # Errors
    ///
    /// Rejects malformed DAGs, incompatible structures, duplicate episodes,
    /// proposals without positive compression, and any replay mismatch.
    pub fn compress_proof_subgraphs(
        &self,
        graphs: &[ProofGraph],
    ) -> Result<GraphCompressionProposal, ConsolidationError> {
        if graphs.len() < 2 {
            return Err(ConsolidationError::InsufficientEpisodes);
        }
        let mut ordered = graphs.iter().collect::<Vec<_>>();
        ordered.sort_unstable_by_key(|graph| graph.episode);
        if ordered
            .windows(2)
            .any(|pair| pair[0].episode == pair[1].episode)
        {
            return Err(ConsolidationError::DuplicateEpisode);
        }
        let normalized = ordered
            .iter()
            .map(|graph| normalize_ancestral_subgraph(graph))
            .collect::<Result<Vec<_>, _>>()?;
        let width = normalized[0].len();
        if width < 2 {
            return Err(ConsolidationError::NoSharedStructure);
        }
        if normalized.iter().any(|nodes| nodes.len() != width) {
            return Err(ConsolidationError::IncompatibleGraphShapes);
        }

        let (pattern, parameter_columns) = anti_unify_graph_nodes(&normalized)?;
        let bindings = bind_episodes(&ordered, &parameter_columns);
        let trace_count =
            u64::try_from(normalized.len()).map_err(|_| ConsolidationError::Capacity)?;
        let (uncompressed_nodes, compressed_nodes) = compression_costs(
            &normalized[0],
            &pattern,
            parameter_columns.len(),
            trace_count,
        )?;
        if compressed_nodes >= uncompressed_nodes {
            return Err(ConsolidationError::NoCompression);
        }
        for (nodes, episode_bindings) in normalized.iter().zip(&bindings) {
            if instantiate_pattern(&pattern, episode_bindings)? != *nodes {
                return Err(ConsolidationError::ReplayMismatch);
            }
        }
        let source_episodes = ordered
            .iter()
            .map(|graph| graph.episode)
            .collect::<Vec<_>>();
        let semantic_hash = graph_proposal_hash(
            &pattern,
            &source_episodes,
            uncompressed_nodes,
            compressed_nodes,
        )?;
        Ok(GraphCompressionProposal {
            output: u32::try_from(width - 1).map_err(|_| ConsolidationError::Capacity)?,
            pattern,
            bindings,
            source_episodes,
            uncompressed_nodes,
            compressed_nodes,
            verified_replays: trace_count,
            semantic_hash,
        })
    }

    /// Mine one repeated internal mechanism from differently shaped host proof
    /// DAGs. The reference miner exhaustively enumerates bounded contiguous
    /// topological windows, converts dependencies on surrounding nodes into
    /// typed boundary parameters, cohorts by structure without family labels,
    /// anti-unifies typed arguments, and exactly reconstructs every occurrence.
    ///
    /// # Errors
    ///
    /// Rejects malformed/duplicate graphs, invalid or exhausted limits,
    /// non-compressing evidence, incompatible types/topology, and replay or
    /// canonicalization failures.
    pub fn mine_internal_proof_subgraphs(
        &self,
        graphs: &[ProofGraph],
        limits: InternalMiningLimits,
    ) -> Result<InternalGraphCompressionProposal, ConsolidationError> {
        if graphs.len() < 2 {
            return Err(ConsolidationError::InsufficientEpisodes);
        }
        if limits.minimum_nodes < 2
            || limits.maximum_nodes < limits.minimum_nodes
            || limits.minimum_independent_episodes < 2
            || limits.maximum_windows_per_episode == 0
        {
            return Err(ConsolidationError::InvalidMiningLimits);
        }
        let mut ordered = graphs.iter().collect::<Vec<_>>();
        ordered.sort_unstable_by_key(|graph| graph.episode);
        if ordered
            .windows(2)
            .any(|pair| pair[0].episode == pair[1].episode)
        {
            return Err(ConsolidationError::DuplicateEpisode);
        }
        let mut cohorts = BTreeMap::<Hash32, BTreeMap<Hash32, InternalSourceSlice>>::new();
        for graph in ordered {
            normalize_ancestral_subgraph(graph)?;
            let mut considered = 0_usize;
            let maximum = limits.maximum_nodes.min(graph.nodes.len());
            for width in limits.minimum_nodes..=maximum {
                for start in 0..=graph.nodes.len() - width {
                    considered = considered
                        .checked_add(1)
                        .ok_or(ConsolidationError::Capacity)?;
                    if considered > limits.maximum_windows_per_episode {
                        return Err(ConsolidationError::Capacity);
                    }
                    let end = start + width;
                    if start == 0 && end == graph.nodes.len()
                        || !window_is_connected(graph, start, end)
                    {
                        continue;
                    }
                    let slice = internal_source_slice(graph, start, end)?;
                    let shape = internal_slice_shape_hash(&slice)?;
                    let episode_cohort = cohorts.entry(shape).or_default();
                    match episode_cohort.get(&graph.episode) {
                        Some(existing) if existing.selected_nodes <= slice.selected_nodes => {}
                        _ => {
                            episode_cohort.insert(graph.episode, slice);
                        }
                    }
                }
            }
        }
        let mut proposals = cohorts
            .into_values()
            .filter(|cohort| cohort.len() >= limits.minimum_independent_episodes)
            .filter_map(|cohort| {
                let slices = cohort.into_values().collect::<Vec<_>>();
                match compress_internal_slices(&slices) {
                    Ok(proposal) => Some(Ok(proposal)),
                    Err(
                        ConsolidationError::NoCompression | ConsolidationError::NoSharedStructure,
                    ) => None,
                    Err(error) => Some(Err(error)),
                }
            })
            .collect::<Result<Vec<_>, _>>()?;
        proposals.sort_by_key(|proposal| {
            (
                std::cmp::Reverse(
                    proposal
                        .uncompressed_nodes
                        .saturating_sub(proposal.compressed_nodes),
                ),
                proposal.semantic_hash,
            )
        });
        proposals
            .into_iter()
            .next()
            .ok_or(ConsolidationError::NoInternalCompression)
    }

    /// Rebind a mined internal mechanism to every matching occurrence in a new
    /// host proof. Fixed arguments and shared parameter equality are enforced;
    /// source node IDs are returned only as occurrence-local bindings.
    ///
    /// # Errors
    ///
    /// Rejects malformed hosts, zero work budget, capacity exhaustion, and
    /// absence of a structurally and type-correct occurrence.
    pub fn match_internal_proof_subgraph(
        &self,
        proposal: &InternalGraphCompressionProposal,
        graph: &ProofGraph,
        maximum_windows: usize,
    ) -> Result<Vec<InternalEpisodeBindings>, ConsolidationError> {
        let parameter_types = proposal
            .bindings
            .first()
            .map(|binding| {
                binding
                    .values
                    .iter()
                    .map(|value| value.ty)
                    .collect::<Vec<_>>()
            })
            .ok_or(ConsolidationError::NoInternalMatch)?;
        match_internal_pattern(
            &proposal.pattern,
            &proposal.boundary_types,
            &parameter_types,
            graph,
            maximum_windows,
        )
    }

    /// Compile a mined proposal into a source-independent executable proof
    /// schema. Compilation removes episode and cost evidence from semantic
    /// identity while retaining its proposal lineage explicitly.
    ///
    /// # Errors
    ///
    /// Rejects empty evidence, inconsistent parameter types, invalid pattern
    /// references, and canonicalization or capacity failures.
    pub fn compile_internal_proof_mechanism(
        &self,
        proposal: &InternalGraphCompressionProposal,
    ) -> Result<CompiledInternalMechanism, ConsolidationError> {
        let first = proposal
            .bindings
            .first()
            .ok_or(ConsolidationError::NoInternalCompression)?;
        let parameter_types = first
            .values
            .iter()
            .map(|value| value.ty)
            .collect::<Vec<_>>();
        if proposal.pattern.is_empty()
            || usize::try_from(proposal.output)
                .ok()
                .is_none_or(|output| output >= proposal.pattern.len())
            || proposal.bindings.iter().any(|binding| {
                binding.values.len() != parameter_types.len()
                    || binding
                        .values
                        .iter()
                        .zip(&parameter_types)
                        .any(|(value, ty)| value.ty != *ty)
            })
        {
            return Err(ConsolidationError::ReplayMismatch);
        }
        validate_compiled_internal_pattern(
            &proposal.pattern,
            &proposal.boundary_types,
            &parameter_types,
        )?;
        let semantic_hash = internal_mechanism_hash(
            &proposal.pattern,
            &proposal.boundary_types,
            &parameter_types,
            proposal.output,
        )?;
        Ok(CompiledInternalMechanism {
            pattern: proposal.pattern.clone(),
            boundary_types: proposal.boundary_types.clone(),
            parameter_types,
            output: proposal.output,
            source_proposal: proposal.semantic_hash,
            semantic_hash,
        })
    }

    /// Rebind a compiled mechanism to occurrences in a new host proof.
    ///
    /// # Errors
    ///
    /// Rejects malformed hosts, invalid work limits, capacity exhaustion, and
    /// absence of a type- and value-consistent occurrence.
    pub fn match_compiled_internal_mechanism(
        &self,
        mechanism: &CompiledInternalMechanism,
        graph: &ProofGraph,
        maximum_windows: usize,
    ) -> Result<Vec<InternalEpisodeBindings>, ConsolidationError> {
        match_internal_pattern(
            &mechanism.pattern,
            &mechanism.boundary_types,
            &mechanism.parameter_types,
            graph,
            maximum_windows,
        )
    }

    /// Expand one bound invocation through the executable reference schema.
    /// Returned nodes use the host proof's node identifiers for both internal
    /// and boundary inputs and can therefore be compared directly with the
    /// selected host occurrence.
    ///
    /// # Errors
    ///
    /// Rejects incomplete, mistyped, or out-of-range bindings.
    pub fn expand_compiled_internal_mechanism(
        &self,
        mechanism: &CompiledInternalMechanism,
        binding: &InternalEpisodeBindings,
    ) -> Result<Vec<ProofNode>, ConsolidationError> {
        expand_compiled_internal(mechanism, binding)
    }

    /// Exhaustively bind distinct learned mechanisms to pairwise-disjoint
    /// occurrences that all contribute ancestrally to the host result.
    ///
    /// # Errors
    ///
    /// Rejects fewer than two or duplicate mechanisms, malformed graphs,
    /// exhausted work bounds, absent matches, overlapping occurrences, and
    /// invocations that do not contribute to the selected proof output.
    pub fn compose_compiled_internal_mechanisms(
        &self,
        mechanisms: &[CompiledInternalMechanism],
        graph: &ProofGraph,
        maximum_windows_per_mechanism: usize,
        maximum_combinations: u64,
    ) -> Result<InternalCompositionPlan, ConsolidationError> {
        normalize_ancestral_subgraph(graph)?;
        if mechanisms.len() < 2 || maximum_windows_per_mechanism == 0 || maximum_combinations == 0 {
            return Err(ConsolidationError::NoInternalComposition);
        }
        let mut ordered = mechanisms.iter().collect::<Vec<_>>();
        ordered.sort_unstable_by_key(|mechanism| mechanism.semantic_hash());
        if ordered
            .windows(2)
            .any(|pair| pair[0].semantic_hash() == pair[1].semantic_hash())
        {
            return Err(ConsolidationError::DuplicateMechanism);
        }
        let candidates = ordered
            .iter()
            .map(|mechanism| {
                let mut bindings = self.match_compiled_internal_mechanism(
                    mechanism,
                    graph,
                    maximum_windows_per_mechanism,
                )?;
                bindings.sort_by(|left, right| {
                    (&left.selected_nodes, &left.boundary_nodes, &left.values).cmp(&(
                        &right.selected_nodes,
                        &right.boundary_nodes,
                        &right.values,
                    ))
                });
                Ok(bindings)
            })
            .collect::<Result<Vec<_>, ConsolidationError>>()?;
        let ancestors = proof_ancestors(graph)?;
        let mut considered = 0_u64;
        let mut selected = Vec::new();
        if choose_internal_composition(
            &ordered,
            &candidates,
            &ancestors,
            &mut selected,
            &mut considered,
            maximum_combinations,
        )? {
            return Ok(InternalCompositionPlan {
                episode: graph.episode,
                invocations: selected,
                combinations_considered: considered,
            });
        }
        Err(ConsolidationError::NoInternalComposition)
    }

    /// Construct complete proof candidates from evidence-only nodes by
    /// exhaustively instantiating learned mechanism boundaries and typed
    /// parameter values. The final joint-result node is a fixed substrate
    /// constructor; no completed host proof or expected result is inspected.
    ///
    /// # Errors
    ///
    /// Rejects malformed evidence, fewer than two or duplicate mechanisms,
    /// incompatible result types, missing typed bindings, and exhausted
    /// binding/candidate limits.
    #[allow(clippy::too_many_lines)] // Keep bounded construction and its accounting in one reference path.
    pub fn construct_compiled_internal_compositions(
        &self,
        mechanisms: &[CompiledInternalMechanism],
        frontier: &ProofFrontier,
        limits: InternalConstructionLimits,
    ) -> Result<Vec<ConstructedInternalComposition>, ConsolidationError> {
        validate_proof_frontier(frontier)?;
        if mechanisms.len() < 2
            || limits.maximum_bindings_per_mechanism == 0
            || limits.maximum_candidates == 0
        {
            return Err(ConsolidationError::NoConstructionCandidate);
        }
        let mut ordered = mechanisms.iter().collect::<Vec<_>>();
        ordered.sort_unstable_by_key(|mechanism| mechanism.semantic_hash());
        if ordered
            .windows(2)
            .any(|pair| pair[0].semantic_hash() == pair[1].semantic_hash())
        {
            return Err(ConsolidationError::DuplicateMechanism);
        }
        let output_types = ordered
            .iter()
            .map(|mechanism| {
                mechanism
                    .pattern()
                    .get(
                        usize::try_from(mechanism.output())
                            .map_err(|_| ConsolidationError::Capacity)?,
                    )
                    .map(|node| node.output_type)
                    .ok_or(ConsolidationError::InvalidOutput)
            })
            .collect::<Result<BTreeSet<_>, _>>()?;
        if output_types.len() != 1 {
            return Err(ConsolidationError::IncompatibleTypes);
        }
        let result_type = *output_types
            .first()
            .ok_or(ConsolidationError::InvalidOutput)?;
        let binding_sets = ordered
            .iter()
            .map(|mechanism| {
                frontier_bindings(mechanism, frontier, limits.maximum_bindings_per_mechanism)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let combinations = enumerate_products(&binding_sets, limits.maximum_candidates)?;
        let mut output = Vec::with_capacity(combinations.len());
        for (candidate_index, templates) in combinations.into_iter().enumerate() {
            let mut nodes = frontier.nodes.clone();
            let mut invocations = Vec::with_capacity(ordered.len());
            let mut joint_inputs = Vec::with_capacity(ordered.len());
            for (mechanism, template) in ordered.iter().zip(templates) {
                let start = u32::try_from(nodes.len()).map_err(|_| ConsolidationError::Capacity)?;
                let width = u32::try_from(mechanism.pattern().len())
                    .map_err(|_| ConsolidationError::Capacity)?;
                let selected_nodes = (0..width)
                    .map(|offset| {
                        start
                            .checked_add(offset)
                            .ok_or(ConsolidationError::Capacity)
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                let binding = InternalEpisodeBindings {
                    episode: frontier.episode,
                    selected_nodes,
                    boundary_nodes: template.boundary_nodes,
                    values: template.values,
                };
                let expanded = self.expand_compiled_internal_mechanism(mechanism, &binding)?;
                nodes.extend(expanded);
                joint_inputs.push(
                    *binding
                        .selected_nodes
                        .get(
                            usize::try_from(mechanism.output())
                                .map_err(|_| ConsolidationError::Capacity)?,
                        )
                        .ok_or(ConsolidationError::InvalidOutput)?,
                );
                invocations.push(InternalMechanismInvocation {
                    mechanism: mechanism.semantic_hash(),
                    binding,
                });
            }
            nodes.push(ProofNode {
                operation: "join-learned-results".to_owned(),
                output_type: result_type,
                inputs: joint_inputs,
                arguments: Vec::new(),
            });
            output.push(ConstructedInternalComposition {
                graph: ProofGraph {
                    episode: frontier.episode,
                    output: u32::try_from(nodes.len() - 1)
                        .map_err(|_| ConsolidationError::Capacity)?,
                    nodes,
                },
                invocations,
                candidates_considered: u64::try_from(candidate_index + 1)
                    .map_err(|_| ConsolidationError::Capacity)?,
            });
        }
        if output.is_empty() {
            return Err(ConsolidationError::NoConstructionCandidate);
        }
        Ok(output)
    }

    /// Infer a task-local typed correspondence policy from a construction that
    /// the caller has independently verified. The returned policy contains
    /// only evidence ordinals and cannot certify the source construction.
    ///
    /// # Errors
    ///
    /// Rejects mechanism/invocation disagreement or bindings not present in
    /// the evidence frontier.
    pub fn induce_internal_construction_policy(
        &self,
        mechanisms: &[CompiledInternalMechanism],
        frontier: &ProofFrontier,
        construction: &ConstructedInternalComposition,
    ) -> Result<InternalConstructionPolicy, ConsolidationError> {
        validate_proof_frontier(frontier)?;
        let mut ordered = mechanisms.iter().collect::<Vec<_>>();
        ordered.sort_unstable_by_key(|mechanism| mechanism.semantic_hash());
        let mut invocations = construction.invocations.iter().collect::<Vec<_>>();
        invocations.sort_unstable_by_key(|invocation| invocation.mechanism);
        if ordered.len() != invocations.len()
            || ordered
                .iter()
                .zip(&invocations)
                .any(|(mechanism, invocation)| {
                    mechanism.semantic_hash() != invocation.mechanism
                        || invocation.binding.episode != frontier.episode
                })
        {
            return Err(ConsolidationError::ReplayMismatch);
        }
        let mut boundary_ordinals = Vec::with_capacity(ordered.len());
        let mut value_ordinals = Vec::with_capacity(ordered.len());
        for (mechanism, invocation) in ordered.iter().zip(invocations) {
            let boundaries = boundary_candidate_dimensions(mechanism, frontier)?;
            let values = value_candidate_dimensions(mechanism, frontier);
            boundary_ordinals.push(binding_ordinals(
                &boundaries,
                &invocation.binding.boundary_nodes,
            )?);
            value_ordinals.push(binding_ordinals(&values, &invocation.binding.values)?);
        }
        Ok(InternalConstructionPolicy {
            mechanisms: ordered
                .iter()
                .map(|mechanism| mechanism.semantic_hash())
                .collect(),
            boundary_ordinals,
            value_ordinals,
        })
    }

    /// Apply a learned typed-ordinal policy to a changed evidence frontier and
    /// construct exactly one proof candidate without expected-proof input.
    ///
    /// # Errors
    ///
    /// Rejects changed mechanism identity, unsupported evidence cardinality,
    /// missing ordinal bindings, and malformed construction.
    pub fn apply_internal_construction_policy(
        &self,
        mechanisms: &[CompiledInternalMechanism],
        frontier: &ProofFrontier,
        policy: &InternalConstructionPolicy,
    ) -> Result<ConstructedInternalComposition, ConsolidationError> {
        validate_proof_frontier(frontier)?;
        let mut ordered = mechanisms.iter().collect::<Vec<_>>();
        ordered.sort_unstable_by_key(|mechanism| mechanism.semantic_hash());
        if policy.mechanisms
            != ordered
                .iter()
                .map(|mechanism| mechanism.semantic_hash())
                .collect::<Vec<_>>()
            || policy.boundary_ordinals.len() != ordered.len()
            || policy.value_ordinals.len() != ordered.len()
        {
            return Err(ConsolidationError::ReplayMismatch);
        }
        let templates = ordered
            .iter()
            .enumerate()
            .map(|(index, mechanism)| {
                let boundaries = boundary_candidate_dimensions(mechanism, frontier)?;
                let values = value_candidate_dimensions(mechanism, frontier);
                Ok(FrontierBinding {
                    boundary_nodes: select_ordinals(&boundaries, &policy.boundary_ordinals[index])?,
                    values: select_ordinals(&values, &policy.value_ordinals[index])?,
                })
            })
            .collect::<Result<Vec<_>, ConsolidationError>>()?;
        construct_internal_candidate(&ordered, frontier, templates, 1)
    }

    /// Infer task-local correspondence from one caller-verified construction.
    /// Values are represented only by typed two-round relational fingerprints;
    /// literal values, element identities, modality, and order are discarded.
    ///
    /// # Errors
    ///
    /// Rejects observation/frontier disagreement, invocation mismatch, values
    /// absent from evidence, or any ambiguous structural selector.
    pub fn induce_relational_construction_policy(
        &self,
        mechanisms: &[CompiledInternalMechanism],
        observation: &RelationalObservation,
        lowering: &LoweredObservationFrontier,
        construction: &ConstructedInternalComposition,
        limits: ObservationFrontierLimits,
    ) -> Result<RelationalConstructionPolicy, ConsolidationError> {
        let replayed = self.lower_relational_observation(observation, limits)?;
        if replayed != *lowering
            || construction.graph.episode != lowering.frontier.episode
            || construction
                .graph
                .nodes
                .get(..lowering.frontier.nodes.len())
                != Some(lowering.frontier.nodes.as_slice())
        {
            return Err(ConsolidationError::ReplayMismatch);
        }
        let mut ordered = mechanisms.iter().collect::<Vec<_>>();
        ordered.sort_unstable_by_key(|mechanism| mechanism.semantic_hash());
        let mut invocations = construction.invocations.iter().collect::<Vec<_>>();
        invocations.sort_unstable_by_key(|invocation| invocation.mechanism);
        if ordered.len() != invocations.len()
            || ordered
                .iter()
                .zip(&invocations)
                .any(|(mechanism, invocation)| {
                    mechanism.semantic_hash() != invocation.mechanism
                        || invocation.binding.episode != lowering.frontier.episode
                })
        {
            return Err(ConsolidationError::ReplayMismatch);
        }
        let fingerprints = relational_fingerprints(observation)?;
        let mut boundary_selectors = Vec::with_capacity(ordered.len());
        let mut value_selectors = Vec::with_capacity(ordered.len());
        for invocation in invocations {
            let boundaries = invocation
                .binding
                .boundary_nodes
                .iter()
                .map(|node| {
                    let selector = frontier_node_selector(&lowering.frontier, *node)?;
                    require_unique_frontier_selector(&lowering.frontier, &selector)?;
                    Ok(selector)
                })
                .collect::<Result<Vec<_>, ConsolidationError>>()?;
            let values = invocation
                .binding
                .values
                .iter()
                .map(|value| relational_value_selector(observation, &fingerprints, *value))
                .collect::<Result<Vec<_>, _>>()?;
            boundary_selectors.push(boundaries);
            value_selectors.push(values);
        }
        Ok(RelationalConstructionPolicy {
            mechanisms: ordered
                .iter()
                .map(|mechanism| mechanism.semantic_hash())
                .collect(),
            boundary_selectors,
            value_selectors,
        })
    }

    /// Apply relational correspondence to changed observation evidence and
    /// construct exactly one untrusted candidate without expected-proof input.
    ///
    /// # Errors
    ///
    /// Rejects mechanism identity changes, observation/frontier disagreement,
    /// or missing/ambiguous structural matches.
    pub fn apply_relational_construction_policy(
        &self,
        mechanisms: &[CompiledInternalMechanism],
        observation: &RelationalObservation,
        lowering: &LoweredObservationFrontier,
        policy: &RelationalConstructionPolicy,
        limits: ObservationFrontierLimits,
    ) -> Result<ConstructedInternalComposition, ConsolidationError> {
        let replayed = self.lower_relational_observation(observation, limits)?;
        if replayed != *lowering {
            return Err(ConsolidationError::ReplayMismatch);
        }
        let mut ordered = mechanisms.iter().collect::<Vec<_>>();
        ordered.sort_unstable_by_key(|mechanism| mechanism.semantic_hash());
        if policy.mechanisms
            != ordered
                .iter()
                .map(|mechanism| mechanism.semantic_hash())
                .collect::<Vec<_>>()
            || policy.boundary_selectors.len() != ordered.len()
            || policy.value_selectors.len() != ordered.len()
        {
            return Err(ConsolidationError::ReplayMismatch);
        }
        let fingerprints = relational_fingerprints(observation)?;
        let templates = ordered
            .iter()
            .enumerate()
            .map(|(index, _)| {
                let boundary_nodes = policy.boundary_selectors[index]
                    .iter()
                    .map(|selector| select_unique_frontier_node(&lowering.frontier, selector))
                    .collect::<Result<Vec<_>, _>>()?;
                let values = policy.value_selectors[index]
                    .iter()
                    .map(|selector| {
                        select_unique_relational_value(observation, &fingerprints, *selector)
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(FrontierBinding {
                    boundary_nodes,
                    values,
                })
            })
            .collect::<Result<Vec<_>, ConsolidationError>>()?;
        construct_internal_candidate(&ordered, &lowering.frontier, templates, 1)
    }

    /// Anti-unify equally shaped proof traces, replacing varying operations by
    /// typed holes while retaining shared structure.
    ///
    /// # Errors
    ///
    /// Requires independent nonempty episodes and a positive compression gain.
    pub fn anti_unify(
        &self,
        traces: &[ProofTrace],
    ) -> Result<CompressionProposal, ConsolidationError> {
        if traces.len() < 2 {
            return Err(ConsolidationError::InsufficientEpisodes);
        }
        let mut episodes = traces.iter().map(|trace| trace.episode).collect::<Vec<_>>();
        episodes.sort_unstable();
        episodes.dedup();
        if episodes.len() != traces.len() {
            return Err(ConsolidationError::DuplicateEpisode);
        }
        let width = traces[0].operations.len();
        if width == 0 || traces.iter().any(|trace| trace.operations.is_empty()) {
            return Err(ConsolidationError::EmptyTrace);
        }
        if traces.iter().any(|trace| trace.operations.len() != width) {
            return Err(ConsolidationError::IncompatibleTraceShapes);
        }
        let mut pattern = Vec::with_capacity(width);
        let mut fixed = 0_u64;
        let mut parameters = 0_u32;
        for index in 0..width {
            let first = &traces[0].operations[index];
            if traces.iter().all(|trace| trace.operations[index] == *first) {
                fixed += 1;
                pattern.push(PatternElement::Fixed(first.clone()));
            } else {
                pattern.push(PatternElement::Parameter(parameters));
                parameters = parameters
                    .checked_add(1)
                    .ok_or(ConsolidationError::Capacity)?;
            }
        }
        if fixed == 0 {
            return Err(ConsolidationError::NoSharedStructure);
        }
        let trace_count = u64::try_from(traces.len()).map_err(|_| ConsolidationError::Capacity)?;
        let width = u64::try_from(width).map_err(|_| ConsolidationError::Capacity)?;
        let uncompressed_nodes = trace_count
            .checked_mul(width)
            .ok_or(ConsolidationError::Capacity)?;
        let compressed_nodes = width
            .checked_add(u64::from(parameters).saturating_mul(trace_count))
            .ok_or(ConsolidationError::Capacity)?;
        if compressed_nodes >= uncompressed_nodes {
            return Err(ConsolidationError::NoCompression);
        }
        Ok(CompressionProposal {
            pattern,
            source_episodes: episodes,
            uncompressed_nodes,
            compressed_nodes,
        })
    }
}

fn proof_ancestors(graph: &ProofGraph) -> Result<BTreeSet<u32>, ConsolidationError> {
    let mut ancestors = BTreeSet::new();
    let mut frontier = vec![graph.output];
    while let Some(node_id) = frontier.pop() {
        if !ancestors.insert(node_id) {
            continue;
        }
        let node = graph
            .nodes
            .get(usize::try_from(node_id).map_err(|_| ConsolidationError::Capacity)?)
            .ok_or(ConsolidationError::InvalidTopology)?;
        frontier.extend(node.inputs.iter().copied());
    }
    Ok(ancestors)
}

fn choose_internal_composition(
    mechanisms: &[&CompiledInternalMechanism],
    candidates: &[Vec<InternalEpisodeBindings>],
    ancestors: &BTreeSet<u32>,
    selected: &mut Vec<InternalMechanismInvocation>,
    considered: &mut u64,
    maximum: u64,
) -> Result<bool, ConsolidationError> {
    let index = selected.len();
    if index == mechanisms.len() {
        return Ok(true);
    }
    let mechanism = mechanisms[index];
    let output_index =
        usize::try_from(mechanism.output()).map_err(|_| ConsolidationError::Capacity)?;
    for binding in &candidates[index] {
        *considered = considered
            .checked_add(1)
            .ok_or(ConsolidationError::Capacity)?;
        if *considered > maximum {
            return Err(ConsolidationError::Capacity);
        }
        let output_node = binding
            .selected_nodes
            .get(output_index)
            .ok_or(ConsolidationError::NoInternalComposition)?;
        if !ancestors.contains(output_node)
            || selected.iter().any(|invocation| {
                invocation
                    .binding
                    .selected_nodes
                    .iter()
                    .any(|node| binding.selected_nodes.contains(node))
            })
        {
            continue;
        }
        selected.push(InternalMechanismInvocation {
            mechanism: mechanism.semantic_hash(),
            binding: binding.clone(),
        });
        if choose_internal_composition(
            mechanisms, candidates, ancestors, selected, considered, maximum,
        )? {
            return Ok(true);
        }
        selected.pop();
    }
    Ok(false)
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct FrontierBinding {
    boundary_nodes: Vec<u32>,
    values: Vec<ProofDatum>,
}

fn construct_internal_candidate(
    mechanisms: &[&CompiledInternalMechanism],
    frontier: &ProofFrontier,
    templates: Vec<FrontierBinding>,
    candidates_considered: u64,
) -> Result<ConstructedInternalComposition, ConsolidationError> {
    if mechanisms.len() != templates.len() {
        return Err(ConsolidationError::ReplayMismatch);
    }
    let result_types = mechanisms
        .iter()
        .map(|mechanism| {
            mechanism
                .pattern()
                .get(
                    usize::try_from(mechanism.output())
                        .map_err(|_| ConsolidationError::Capacity)?,
                )
                .map(|node| node.output_type)
                .ok_or(ConsolidationError::InvalidOutput)
        })
        .collect::<Result<BTreeSet<_>, _>>()?;
    if result_types.len() != 1 {
        return Err(ConsolidationError::IncompatibleTypes);
    }
    let result_type = *result_types
        .first()
        .ok_or(ConsolidationError::InvalidOutput)?;
    let mut nodes = frontier.nodes.clone();
    let mut invocations = Vec::with_capacity(mechanisms.len());
    let mut joint_inputs = Vec::with_capacity(mechanisms.len());
    for (mechanism, template) in mechanisms.iter().zip(templates) {
        let start = u32::try_from(nodes.len()).map_err(|_| ConsolidationError::Capacity)?;
        let width =
            u32::try_from(mechanism.pattern().len()).map_err(|_| ConsolidationError::Capacity)?;
        let selected_nodes = (0..width)
            .map(|offset| {
                start
                    .checked_add(offset)
                    .ok_or(ConsolidationError::Capacity)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let binding = InternalEpisodeBindings {
            episode: frontier.episode,
            selected_nodes,
            boundary_nodes: template.boundary_nodes,
            values: template.values,
        };
        let expanded = Consolidator.expand_compiled_internal_mechanism(mechanism, &binding)?;
        nodes.extend(expanded);
        joint_inputs.push(
            *binding
                .selected_nodes
                .get(
                    usize::try_from(mechanism.output())
                        .map_err(|_| ConsolidationError::Capacity)?,
                )
                .ok_or(ConsolidationError::InvalidOutput)?,
        );
        invocations.push(InternalMechanismInvocation {
            mechanism: mechanism.semantic_hash(),
            binding,
        });
    }
    nodes.push(ProofNode {
        operation: "join-learned-results".to_owned(),
        output_type: result_type,
        inputs: joint_inputs,
        arguments: Vec::new(),
    });
    Ok(ConstructedInternalComposition {
        graph: ProofGraph {
            episode: frontier.episode,
            output: u32::try_from(nodes.len() - 1).map_err(|_| ConsolidationError::Capacity)?,
            nodes,
        },
        invocations,
        candidates_considered,
    })
}

fn boundary_candidate_dimensions(
    mechanism: &CompiledInternalMechanism,
    frontier: &ProofFrontier,
) -> Result<Vec<Vec<u32>>, ConsolidationError> {
    mechanism
        .boundary_types()
        .iter()
        .map(|ty| {
            frontier
                .nodes
                .iter()
                .enumerate()
                .filter(|(_, node)| node.output_type == *ty)
                .map(|(index, _)| u32::try_from(index).map_err(|_| ConsolidationError::Capacity))
                .collect::<Result<Vec<_>, _>>()
        })
        .collect()
}

fn value_candidate_dimensions(
    mechanism: &CompiledInternalMechanism,
    frontier: &ProofFrontier,
) -> Vec<Vec<ProofDatum>> {
    mechanism
        .parameter_types()
        .iter()
        .map(|ty| {
            let mut seen = BTreeSet::new();
            frontier
                .nodes
                .iter()
                .flat_map(|node| &node.arguments)
                .filter(|value| value.ty == *ty && seen.insert(**value))
                .copied()
                .collect()
        })
        .collect()
}

fn frontier_node_selector(
    frontier: &ProofFrontier,
    node: u32,
) -> Result<FrontierNodeSelector, ConsolidationError> {
    let node = frontier
        .nodes
        .get(usize::try_from(node).map_err(|_| ConsolidationError::Capacity)?)
        .ok_or(ConsolidationError::InvalidFrontier)?;
    let input_types = node
        .inputs
        .iter()
        .map(|input| {
            frontier
                .nodes
                .get(usize::try_from(*input).map_err(|_| ConsolidationError::Capacity)?)
                .map(|input| input.output_type)
                .ok_or(ConsolidationError::InvalidFrontier)
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(FrontierNodeSelector {
        operation: node.operation.clone(),
        output_type: node.output_type,
        input_types,
        argument_types: node.arguments.iter().map(|value| value.ty).collect(),
    })
}

fn frontier_node_matches(
    frontier: &ProofFrontier,
    index: usize,
    selector: &FrontierNodeSelector,
) -> Result<bool, ConsolidationError> {
    let node = frontier
        .nodes
        .get(index)
        .ok_or(ConsolidationError::InvalidFrontier)?;
    let input_types = node
        .inputs
        .iter()
        .map(|input| {
            frontier
                .nodes
                .get(usize::try_from(*input).map_err(|_| ConsolidationError::Capacity)?)
                .map(|input| input.output_type)
                .ok_or(ConsolidationError::InvalidFrontier)
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(node.operation == selector.operation
        && node.output_type == selector.output_type
        && input_types == selector.input_types
        && node
            .arguments
            .iter()
            .map(|value| value.ty)
            .eq(selector.argument_types.iter().copied()))
}

fn select_unique_frontier_node(
    frontier: &ProofFrontier,
    selector: &FrontierNodeSelector,
) -> Result<u32, ConsolidationError> {
    let mut matches = Vec::new();
    for index in 0..frontier.nodes.len() {
        if frontier_node_matches(frontier, index, selector)? {
            matches.push(u32::try_from(index).map_err(|_| ConsolidationError::Capacity)?);
        }
    }
    match matches.as_slice() {
        [only] => Ok(*only),
        [] => Err(ConsolidationError::MissingCorrespondence),
        _ => Err(ConsolidationError::AmbiguousCorrespondence),
    }
}

fn require_unique_frontier_selector(
    frontier: &ProofFrontier,
    selector: &FrontierNodeSelector,
) -> Result<(), ConsolidationError> {
    select_unique_frontier_node(frontier, selector).map(|_| ())
}

fn relational_fingerprints(
    observation: &RelationalObservation,
) -> Result<BTreeMap<EvidenceElementId, Hash32>, ConsolidationError> {
    let mut colors = observation
        .elements
        .iter()
        .map(|element| {
            CanonicalValue::Array(vec![
                CanonicalValue::Text("cairn.relational-fingerprint.seed.v1".to_owned()),
                CanonicalValue::Unsigned(proof_type_code(proof_value_type(element.value_type))),
            ])
            .encode_canonical()
            .map(|bytes| (element.id, Hash32::digest(&bytes)))
            .map_err(|_| ConsolidationError::Canonicalization)
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
                                .map_err(|_| ConsolidationError::Capacity)
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    let participants = relation
                        .participants
                        .iter()
                        .map(|participant| {
                            colors
                                .get(participant)
                                .map(|color| CanonicalValue::Bytes(color.as_bytes().to_vec()))
                                .ok_or(ConsolidationError::InvalidObservation)
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    CanonicalValue::Array(vec![
                        CanonicalValue::Bytes(relation.predicate.as_bytes().to_vec()),
                        CanonicalValue::Array(positions),
                        CanonicalValue::Array(participants),
                    ])
                    .encode_canonical()
                    .map_err(|_| ConsolidationError::Canonicalization)
                })
                .collect::<Result<Vec<_>, _>>()?;
            incident.sort_unstable();
            let encoded = CanonicalValue::Array(vec![
                CanonicalValue::Text("cairn.relational-fingerprint.round.v1".to_owned()),
                CanonicalValue::Unsigned(round),
                CanonicalValue::Unsigned(proof_type_code(proof_value_type(element.value_type))),
                CanonicalValue::Array(incident.into_iter().map(CanonicalValue::Bytes).collect()),
            ])
            .encode_canonical()
            .map_err(|_| ConsolidationError::Canonicalization)?;
            next.insert(element.id, Hash32::digest(&encoded));
        }
        colors = next;
    }
    Ok(colors)
}

fn relational_value_selector(
    observation: &RelationalObservation,
    fingerprints: &BTreeMap<EvidenceElementId, Hash32>,
    value: ProofDatum,
) -> Result<RelationalValueSelector, ConsolidationError> {
    let matches = observation
        .elements
        .iter()
        .filter(|element| {
            proof_value_type(element.value_type) == value.ty && element.value == value.value
        })
        .collect::<Vec<_>>();
    let [element] = matches.as_slice() else {
        return if matches.is_empty() {
            Err(ConsolidationError::MissingCorrespondence)
        } else {
            Err(ConsolidationError::AmbiguousCorrespondence)
        };
    };
    let fingerprint = *fingerprints
        .get(&element.id)
        .ok_or(ConsolidationError::InvalidObservation)?;
    let same_structure = observation
        .elements
        .iter()
        .filter(|candidate| proof_value_type(candidate.value_type) == value.ty)
        .filter(|candidate| fingerprints.get(&candidate.id) == Some(&fingerprint))
        .count();
    if same_structure != 1 {
        return Err(ConsolidationError::AmbiguousCorrespondence);
    }
    Ok(RelationalValueSelector {
        value_type: value.ty,
        fingerprint,
    })
}

fn select_unique_relational_value(
    observation: &RelationalObservation,
    fingerprints: &BTreeMap<EvidenceElementId, Hash32>,
    selector: RelationalValueSelector,
) -> Result<ProofDatum, ConsolidationError> {
    let matches = observation
        .elements
        .iter()
        .filter(|element| proof_value_type(element.value_type) == selector.value_type)
        .filter(|element| fingerprints.get(&element.id) == Some(&selector.fingerprint))
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [element] => Ok(ProofDatum {
            ty: selector.value_type,
            value: element.value,
        }),
        [] => Err(ConsolidationError::MissingCorrespondence),
        _ => Err(ConsolidationError::AmbiguousCorrespondence),
    }
}

fn binding_ordinals<T: Eq>(
    dimensions: &[Vec<T>],
    bindings: &[T],
) -> Result<Vec<u32>, ConsolidationError> {
    if dimensions.len() != bindings.len() {
        return Err(ConsolidationError::ReplayMismatch);
    }
    dimensions
        .iter()
        .zip(bindings)
        .map(|(values, binding)| {
            values
                .iter()
                .position(|value| value == binding)
                .ok_or(ConsolidationError::ReplayMismatch)
                .and_then(|index| u32::try_from(index).map_err(|_| ConsolidationError::Capacity))
        })
        .collect()
}

fn select_ordinals<T: Clone>(
    dimensions: &[Vec<T>],
    ordinals: &[u32],
) -> Result<Vec<T>, ConsolidationError> {
    if dimensions.len() != ordinals.len() {
        return Err(ConsolidationError::ReplayMismatch);
    }
    dimensions
        .iter()
        .zip(ordinals)
        .map(|(values, ordinal)| {
            values
                .get(usize::try_from(*ordinal).map_err(|_| ConsolidationError::Capacity)?)
                .cloned()
                .ok_or(ConsolidationError::ReplayMismatch)
        })
        .collect()
}

fn validate_proof_frontier(frontier: &ProofFrontier) -> Result<(), ConsolidationError> {
    if frontier.nodes.is_empty() {
        return Err(ConsolidationError::InvalidFrontier);
    }
    for (index, node) in frontier.nodes.iter().enumerate() {
        if node.operation.is_empty()
            || node.operation == "join-learned-results"
            || node
                .inputs
                .iter()
                .any(|input| usize::try_from(*input).map_or(true, |value| value >= index))
        {
            return Err(ConsolidationError::InvalidFrontier);
        }
    }
    Ok(())
}

fn frontier_bindings(
    mechanism: &CompiledInternalMechanism,
    frontier: &ProofFrontier,
    maximum: u64,
) -> Result<Vec<FrontierBinding>, ConsolidationError> {
    let boundary_dimensions = boundary_candidate_dimensions(mechanism, frontier)?;
    let value_dimensions = value_candidate_dimensions(mechanism, frontier);
    if boundary_dimensions.iter().any(Vec::is_empty) || value_dimensions.iter().any(Vec::is_empty) {
        return Err(ConsolidationError::NoConstructionCandidate);
    }
    let boundary_bindings = enumerate_products(&boundary_dimensions, maximum)?;
    let value_bindings = enumerate_products(&value_dimensions, maximum)?;
    let mut bindings = Vec::new();
    for boundary_nodes in boundary_bindings {
        for values in &value_bindings {
            if u64::try_from(bindings.len()).map_err(|_| ConsolidationError::Capacity)? >= maximum {
                return Err(ConsolidationError::Capacity);
            }
            bindings.push(FrontierBinding {
                boundary_nodes: boundary_nodes.clone(),
                values: values.clone(),
            });
        }
    }
    Ok(bindings)
}

fn enumerate_products<T: Clone>(
    dimensions: &[Vec<T>],
    maximum: u64,
) -> Result<Vec<Vec<T>>, ConsolidationError> {
    fn extend<T: Clone>(
        dimensions: &[Vec<T>],
        index: usize,
        maximum: u64,
        current: &mut Vec<T>,
        output: &mut Vec<Vec<T>>,
    ) -> Result<(), ConsolidationError> {
        if index == dimensions.len() {
            if u64::try_from(output.len()).map_err(|_| ConsolidationError::Capacity)? >= maximum {
                return Err(ConsolidationError::Capacity);
            }
            output.push(current.clone());
            return Ok(());
        }
        for value in &dimensions[index] {
            current.push(value.clone());
            extend(dimensions, index + 1, maximum, current, output)?;
            current.pop();
        }
        Ok(())
    }

    if maximum == 0 || dimensions.iter().any(Vec::is_empty) {
        return Err(ConsolidationError::NoConstructionCandidate);
    }
    let mut output = Vec::new();
    extend(dimensions, 0, maximum, &mut Vec::new(), &mut output)?;
    Ok(output)
}

fn anti_unify_graph_nodes(
    normalized: &[Vec<ProofNode>],
) -> Result<(Vec<GraphPatternNode>, Vec<Vec<ProofDatum>>), ConsolidationError> {
    let mut pattern = Vec::with_capacity(normalized[0].len());
    let mut parameter_columns: Vec<Vec<ProofDatum>> = Vec::new();
    for node_index in 0..normalized[0].len() {
        let representative = &normalized[0][node_index];
        validate_node_shapes(normalized, node_index, representative)?;
        let mut arguments = Vec::with_capacity(representative.arguments.len());
        for argument_index in 0..representative.arguments.len() {
            let column = normalized
                .iter()
                .map(|nodes| nodes[node_index].arguments[argument_index])
                .collect::<Vec<_>>();
            arguments.push(anti_unify_argument(column, &mut parameter_columns)?);
        }
        pattern.push(GraphPatternNode {
            operation: representative.operation.clone(),
            output_type: representative.output_type,
            inputs: representative.inputs.clone(),
            arguments,
        });
    }
    Ok((pattern, parameter_columns))
}

fn validate_node_shapes(
    normalized: &[Vec<ProofNode>],
    node_index: usize,
    representative: &ProofNode,
) -> Result<(), ConsolidationError> {
    for nodes in normalized.iter().skip(1) {
        let candidate = &nodes[node_index];
        if candidate.operation != representative.operation
            || candidate.inputs != representative.inputs
        {
            return Err(ConsolidationError::IncompatibleOperation);
        }
        if candidate.output_type != representative.output_type {
            return Err(ConsolidationError::IncompatibleTypes);
        }
        if candidate.arguments.len() != representative.arguments.len() {
            return Err(ConsolidationError::IncompatibleArguments);
        }
    }
    Ok(())
}

fn anti_unify_argument(
    column: Vec<ProofDatum>,
    parameter_columns: &mut Vec<Vec<ProofDatum>>,
) -> Result<GraphPatternArgument, ConsolidationError> {
    if column.iter().all(|value| *value == column[0]) {
        return Ok(GraphPatternArgument::Fixed(column[0]));
    }
    if column.iter().any(|value| value.ty != column[0].ty) {
        return Err(ConsolidationError::IncompatibleTypes);
    }
    let parameter = parameter_columns
        .iter()
        .position(|existing| *existing == column)
        .unwrap_or_else(|| {
            parameter_columns.push(column);
            parameter_columns.len() - 1
        });
    Ok(GraphPatternArgument::Parameter(
        u32::try_from(parameter).map_err(|_| ConsolidationError::Capacity)?,
    ))
}

fn bind_episodes(graphs: &[&ProofGraph], columns: &[Vec<ProofDatum>]) -> Vec<EpisodeBindings> {
    graphs
        .iter()
        .enumerate()
        .map(|(episode_index, graph)| EpisodeBindings {
            episode: graph.episode,
            values: columns.iter().map(|column| column[episode_index]).collect(),
        })
        .collect()
}

fn compression_costs(
    nodes: &[ProofNode],
    pattern: &[GraphPatternNode],
    parameters: usize,
    trace_count: u64,
) -> Result<(u64, u64), ConsolidationError> {
    let raw_per_trace = nodes
        .iter()
        .try_fold(0_u64, |total, node| {
            total
                .checked_add(1)
                .and_then(|value| value.checked_add(u64::try_from(node.arguments.len()).ok()?))
        })
        .ok_or(ConsolidationError::Capacity)?;
    let uncompressed = raw_per_trace
        .checked_mul(trace_count)
        .ok_or(ConsolidationError::Capacity)?;
    let fixed_arguments = pattern
        .iter()
        .try_fold(0_u64, |total, node| {
            let count = node
                .arguments
                .iter()
                .filter(|argument| matches!(argument, GraphPatternArgument::Fixed(_)))
                .count();
            total.checked_add(u64::try_from(count).ok()?)
        })
        .ok_or(ConsolidationError::Capacity)?;
    let parameter_count = u64::try_from(parameters).map_err(|_| ConsolidationError::Capacity)?;
    let compressed = u64::try_from(pattern.len())
        .map_err(|_| ConsolidationError::Capacity)?
        .checked_add(fixed_arguments)
        .and_then(|value| value.checked_add(parameter_count))
        .and_then(|value| value.checked_add(parameter_count.checked_mul(trace_count)?))
        .ok_or(ConsolidationError::Capacity)?;
    Ok((uncompressed, compressed))
}

fn normalize_ancestral_subgraph(graph: &ProofGraph) -> Result<Vec<ProofNode>, ConsolidationError> {
    if graph.nodes.is_empty() {
        return Err(ConsolidationError::EmptyTrace);
    }
    let output = usize::try_from(graph.output).map_err(|_| ConsolidationError::Capacity)?;
    if output >= graph.nodes.len() {
        return Err(ConsolidationError::InvalidOutput);
    }
    for (index, node) in graph.nodes.iter().enumerate() {
        if node.operation.is_empty()
            || node
                .inputs
                .iter()
                .any(|input| usize::try_from(*input).map_or(true, |value| value >= index))
        {
            return Err(ConsolidationError::InvalidTopology);
        }
    }
    let mut ancestors = BTreeSet::from([graph.output]);
    let mut pending = vec![graph.output];
    while let Some(node_id) = pending.pop() {
        let node =
            &graph.nodes[usize::try_from(node_id).map_err(|_| ConsolidationError::Capacity)?];
        for input in &node.inputs {
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
                u32::try_from(new).map_err(|_| ConsolidationError::Capacity)?,
            ))
        })
        .collect::<Result<BTreeMap<_, _>, ConsolidationError>>()?;
    selected
        .iter()
        .map(|old| {
            let mut node = graph.nodes
                [usize::try_from(*old).map_err(|_| ConsolidationError::Capacity)?]
            .clone();
            node.inputs = node.inputs.iter().map(|input| remap[input]).collect();
            Ok(node)
        })
        .collect()
}

fn instantiate_pattern(
    pattern: &[GraphPatternNode],
    bindings: &EpisodeBindings,
) -> Result<Vec<ProofNode>, ConsolidationError> {
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
                        .get(usize::try_from(*index).map_err(|_| ConsolidationError::Capacity)?)
                        .copied()
                        .ok_or(ConsolidationError::ReplayMismatch),
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

fn graph_proposal_hash(
    pattern: &[GraphPatternNode],
    episodes: &[Hash32],
    uncompressed: u64,
    compressed: u64,
) -> Result<Hash32, ConsolidationError> {
    let nodes = pattern
        .iter()
        .map(|node| {
            CanonicalValue::Array(vec![
                CanonicalValue::Text(node.operation.clone()),
                CanonicalValue::Unsigned(proof_type_code(node.output_type)),
                CanonicalValue::Array(
                    node.inputs
                        .iter()
                        .map(|input| CanonicalValue::Unsigned(u64::from(*input)))
                        .collect(),
                ),
                CanonicalValue::Array(
                    node.arguments
                        .iter()
                        .map(|argument| match argument {
                            GraphPatternArgument::Fixed(value) => CanonicalValue::Array(vec![
                                CanonicalValue::Unsigned(0),
                                CanonicalValue::Unsigned(proof_type_code(value.ty)),
                                CanonicalValue::Bytes(value.value.as_bytes().to_vec()),
                            ]),
                            GraphPatternArgument::Parameter(index) => CanonicalValue::Array(vec![
                                CanonicalValue::Unsigned(1),
                                CanonicalValue::Unsigned(u64::from(*index)),
                            ]),
                        })
                        .collect(),
                ),
            ])
        })
        .collect();
    let value = CanonicalValue::Array(vec![
        CanonicalValue::Text(CONSOLIDATOR_SEMANTICS_VERSION.to_owned()),
        CanonicalValue::Array(nodes),
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
        .map_err(|_| ConsolidationError::Canonicalization)
}

fn graph_shape_hash(nodes: &[ProofNode]) -> Result<Hash32, ConsolidationError> {
    let shape = CanonicalValue::Array(
        nodes
            .iter()
            .map(|node| {
                CanonicalValue::Array(vec![
                    CanonicalValue::Text(node.operation.clone()),
                    CanonicalValue::Unsigned(proof_type_code(node.output_type)),
                    CanonicalValue::Array(
                        node.inputs
                            .iter()
                            .map(|input| CanonicalValue::Unsigned(u64::from(*input)))
                            .collect(),
                    ),
                    CanonicalValue::Array(
                        node.arguments
                            .iter()
                            .map(|argument| CanonicalValue::Unsigned(proof_type_code(argument.ty)))
                            .collect(),
                    ),
                ])
            })
            .collect(),
    );
    shape
        .encode_canonical()
        .map(|bytes| Hash32::digest(&bytes))
        .map_err(|_| ConsolidationError::Canonicalization)
}

fn window_is_connected(graph: &ProofGraph, start: usize, end: usize) -> bool {
    let mut adjacency = vec![Vec::<usize>::new(); end - start];
    for source in start..end {
        for input in &graph.nodes[source].inputs {
            let Ok(target) = usize::try_from(*input) else {
                return false;
            };
            if (start..end).contains(&target) {
                adjacency[source - start].push(target - start);
                adjacency[target - start].push(source - start);
            }
        }
    }
    let mut seen = BTreeSet::from([0_usize]);
    let mut pending = vec![0_usize];
    while let Some(node) = pending.pop() {
        for neighbor in &adjacency[node] {
            if seen.insert(*neighbor) {
                pending.push(*neighbor);
            }
        }
    }
    seen.len() == end - start
}

fn internal_source_slice(
    graph: &ProofGraph,
    start: usize,
    end: usize,
) -> Result<InternalSourceSlice, ConsolidationError> {
    let selected_nodes = (start..end)
        .map(|node| u32::try_from(node).map_err(|_| ConsolidationError::Capacity))
        .collect::<Result<Vec<_>, _>>()?;
    let selected_map = selected_nodes
        .iter()
        .enumerate()
        .map(|(local, source)| {
            Ok((
                *source,
                u32::try_from(local).map_err(|_| ConsolidationError::Capacity)?,
            ))
        })
        .collect::<Result<BTreeMap<_, _>, ConsolidationError>>()?;
    let boundary_nodes = graph.nodes[start..end]
        .iter()
        .flat_map(|node| node.inputs.iter().copied())
        .filter(|input| !selected_map.contains_key(input))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let boundary_map = boundary_nodes
        .iter()
        .enumerate()
        .map(|(slot, source)| {
            Ok((
                *source,
                u32::try_from(slot).map_err(|_| ConsolidationError::Capacity)?,
            ))
        })
        .collect::<Result<BTreeMap<_, _>, ConsolidationError>>()?;
    let boundary_types = boundary_nodes
        .iter()
        .map(|source| {
            graph
                .nodes
                .get(usize::try_from(*source).map_err(|_| ConsolidationError::Capacity)?)
                .map(|node| node.output_type)
                .ok_or(ConsolidationError::InvalidTopology)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let nodes = graph.nodes[start..end]
        .iter()
        .map(|node| {
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
                                .ok_or(ConsolidationError::InvalidTopology)
                        },
                        |local| Ok(InternalPatternInput::Internal(local)),
                    )
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(InternalSourceNode {
                operation: node.operation.clone(),
                output_type: node.output_type,
                inputs,
                arguments: node.arguments.clone(),
            })
        })
        .collect::<Result<Vec<_>, ConsolidationError>>()?;
    Ok(InternalSourceSlice {
        episode: graph.episode,
        nodes,
        boundary_types,
        selected_nodes,
        boundary_nodes,
    })
}

fn internal_slice_shape_hash(slice: &InternalSourceSlice) -> Result<Hash32, ConsolidationError> {
    let value = CanonicalValue::Array(vec![
        CanonicalValue::Array(
            slice
                .boundary_types
                .iter()
                .map(|ty| CanonicalValue::Unsigned(proof_type_code(*ty)))
                .collect(),
        ),
        CanonicalValue::Array(
            slice
                .nodes
                .iter()
                .map(|node| {
                    CanonicalValue::Array(vec![
                        CanonicalValue::Text(node.operation.clone()),
                        CanonicalValue::Unsigned(proof_type_code(node.output_type)),
                        CanonicalValue::Array(
                            node.inputs
                                .iter()
                                .copied()
                                .map(internal_input_value)
                                .collect(),
                        ),
                        CanonicalValue::Array(
                            node.arguments
                                .iter()
                                .map(|argument| {
                                    CanonicalValue::Unsigned(proof_type_code(argument.ty))
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
        .map_err(|_| ConsolidationError::Canonicalization)
}

fn compress_internal_slices(
    slices: &[InternalSourceSlice],
) -> Result<InternalGraphCompressionProposal, ConsolidationError> {
    if slices.len() < 2 {
        return Err(ConsolidationError::InsufficientEpisodes);
    }
    let width = slices[0].nodes.len();
    let boundary_types = slices[0].boundary_types.clone();
    if width < 2
        || slices
            .iter()
            .any(|slice| slice.nodes.len() != width || slice.boundary_types != boundary_types)
    {
        return Err(ConsolidationError::IncompatibleGraphShapes);
    }
    let mut parameter_columns = Vec::<Vec<ProofDatum>>::new();
    let mut pattern = Vec::with_capacity(width);
    for node_index in 0..width {
        let representative = &slices[0].nodes[node_index];
        if slices.iter().skip(1).any(|slice| {
            let node = &slice.nodes[node_index];
            node.operation != representative.operation
                || node.output_type != representative.output_type
                || node.inputs != representative.inputs
                || node.arguments.len() != representative.arguments.len()
        }) {
            return Err(ConsolidationError::IncompatibleOperation);
        }
        let arguments = (0..representative.arguments.len())
            .map(|argument_index| {
                let column = slices
                    .iter()
                    .map(|slice| slice.nodes[node_index].arguments[argument_index])
                    .collect::<Vec<_>>();
                anti_unify_argument(column, &mut parameter_columns)
            })
            .collect::<Result<Vec<_>, _>>()?;
        pattern.push(InternalGraphPatternNode {
            operation: representative.operation.clone(),
            output_type: representative.output_type,
            inputs: representative.inputs.clone(),
            arguments,
        });
    }
    let bindings = slices
        .iter()
        .enumerate()
        .map(|(episode_index, slice)| InternalEpisodeBindings {
            episode: slice.episode,
            selected_nodes: slice.selected_nodes.clone(),
            boundary_nodes: slice.boundary_nodes.clone(),
            values: parameter_columns
                .iter()
                .map(|column| column[episode_index])
                .collect(),
        })
        .collect::<Vec<_>>();
    let (uncompressed_nodes, compressed_nodes) =
        internal_compression_costs(slices, &pattern, &boundary_types, &parameter_columns)?;
    if compressed_nodes >= uncompressed_nodes {
        return Err(ConsolidationError::NoCompression);
    }
    for (slice, binding) in slices.iter().zip(&bindings) {
        if instantiate_internal_pattern(&pattern, binding)? != slice.nodes {
            return Err(ConsolidationError::ReplayMismatch);
        }
    }
    let source_episodes = slices.iter().map(|slice| slice.episode).collect::<Vec<_>>();
    let semantic_hash = internal_proposal_hash(
        &pattern,
        &boundary_types,
        &source_episodes,
        uncompressed_nodes,
        compressed_nodes,
    )?;
    Ok(InternalGraphCompressionProposal {
        pattern,
        boundary_types,
        output: u32::try_from(width - 1).map_err(|_| ConsolidationError::Capacity)?,
        bindings,
        source_episodes,
        uncompressed_nodes,
        compressed_nodes,
        verified_replays: u64::try_from(slices.len()).map_err(|_| ConsolidationError::Capacity)?,
        semantic_hash,
    })
}

fn instantiate_internal_pattern(
    pattern: &[InternalGraphPatternNode],
    bindings: &InternalEpisodeBindings,
) -> Result<Vec<InternalSourceNode>, ConsolidationError> {
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
                        .get(usize::try_from(*index).map_err(|_| ConsolidationError::Capacity)?)
                        .copied()
                        .ok_or(ConsolidationError::ReplayMismatch),
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(InternalSourceNode {
                operation: node.operation.clone(),
                output_type: node.output_type,
                inputs: node.inputs.clone(),
                arguments,
            })
        })
        .collect()
}

fn bind_internal_parameters(
    pattern: &[InternalGraphPatternNode],
    nodes: &[InternalSourceNode],
    parameter_types: &[ProofValueType],
) -> Result<Option<Vec<ProofDatum>>, ConsolidationError> {
    if pattern.len() != nodes.len() {
        return Ok(None);
    }
    let mut values = vec![None; parameter_types.len()];
    for (pattern_node, source_node) in pattern.iter().zip(nodes) {
        if pattern_node.operation != source_node.operation
            || pattern_node.output_type != source_node.output_type
            || pattern_node.inputs != source_node.inputs
            || pattern_node.arguments.len() != source_node.arguments.len()
        {
            return Ok(None);
        }
        for (pattern_argument, source_argument) in
            pattern_node.arguments.iter().zip(&source_node.arguments)
        {
            match pattern_argument {
                GraphPatternArgument::Fixed(expected) if expected != source_argument => {
                    return Ok(None);
                }
                GraphPatternArgument::Fixed(_) => {}
                GraphPatternArgument::Parameter(index) => {
                    let index =
                        usize::try_from(*index).map_err(|_| ConsolidationError::Capacity)?;
                    if parameter_types.get(index) != Some(&source_argument.ty) {
                        return Ok(None);
                    }
                    match values.get_mut(index) {
                        Some(slot @ None) => *slot = Some(*source_argument),
                        Some(Some(existing)) if existing == source_argument => {}
                        Some(Some(_)) => return Ok(None),
                        None => return Err(ConsolidationError::ReplayMismatch),
                    }
                }
            }
        }
    }
    values
        .into_iter()
        .collect::<Option<Vec<_>>>()
        .map_or(Ok(None), |values| Ok(Some(values)))
}

fn validate_compiled_internal_pattern(
    pattern: &[InternalGraphPatternNode],
    boundary_types: &[ProofValueType],
    parameter_types: &[ProofValueType],
) -> Result<(), ConsolidationError> {
    let mut used_boundaries = BTreeSet::new();
    let mut used_parameters = BTreeSet::new();
    for (node_index, node) in pattern.iter().enumerate() {
        if node.operation.is_empty() {
            return Err(ConsolidationError::InvalidTopology);
        }
        for input in &node.inputs {
            match input {
                InternalPatternInput::Internal(index)
                    if usize::try_from(*index)
                        .ok()
                        .is_none_or(|index| index >= node_index) =>
                {
                    return Err(ConsolidationError::InvalidTopology);
                }
                InternalPatternInput::Internal(_) => {}
                InternalPatternInput::Boundary(index) => {
                    if usize::try_from(*index)
                        .ok()
                        .is_none_or(|index| index >= boundary_types.len())
                    {
                        return Err(ConsolidationError::InvalidTopology);
                    }
                    used_boundaries.insert(*index);
                }
            }
        }
        for argument in &node.arguments {
            if let GraphPatternArgument::Parameter(index) = argument {
                if usize::try_from(*index)
                    .ok()
                    .is_none_or(|index| index >= parameter_types.len())
                {
                    return Err(ConsolidationError::ReplayMismatch);
                }
                used_parameters.insert(*index);
            }
        }
    }
    let expected_boundaries = (0..boundary_types.len())
        .map(|index| u32::try_from(index).map_err(|_| ConsolidationError::Capacity))
        .collect::<Result<BTreeSet<_>, _>>()?;
    let expected_parameters = (0..parameter_types.len())
        .map(|index| u32::try_from(index).map_err(|_| ConsolidationError::Capacity))
        .collect::<Result<BTreeSet<_>, _>>()?;
    if used_boundaries != expected_boundaries || used_parameters != expected_parameters {
        return Err(ConsolidationError::ReplayMismatch);
    }
    Ok(())
}

fn match_internal_pattern(
    pattern: &[InternalGraphPatternNode],
    boundary_types: &[ProofValueType],
    parameter_types: &[ProofValueType],
    graph: &ProofGraph,
    maximum_windows: usize,
) -> Result<Vec<InternalEpisodeBindings>, ConsolidationError> {
    if maximum_windows == 0 || pattern.is_empty() {
        return Err(ConsolidationError::InvalidMiningLimits);
    }
    validate_compiled_internal_pattern(pattern, boundary_types, parameter_types)?;
    normalize_ancestral_subgraph(graph)?;
    let width = pattern.len();
    if width > graph.nodes.len() {
        return Err(ConsolidationError::NoInternalMatch);
    }
    let mut matches = Vec::new();
    let mut considered = 0_usize;
    for start in 0..=graph.nodes.len() - width {
        considered = considered
            .checked_add(1)
            .ok_or(ConsolidationError::Capacity)?;
        if considered > maximum_windows {
            return Err(ConsolidationError::Capacity);
        }
        let end = start + width;
        if !window_is_connected(graph, start, end) {
            continue;
        }
        let slice = internal_source_slice(graph, start, end)?;
        if slice.boundary_types != boundary_types {
            continue;
        }
        let Some(values) = bind_internal_parameters(pattern, &slice.nodes, parameter_types)? else {
            continue;
        };
        let binding = InternalEpisodeBindings {
            episode: graph.episode,
            selected_nodes: slice.selected_nodes,
            boundary_nodes: slice.boundary_nodes,
            values,
        };
        if instantiate_internal_pattern(pattern, &binding)? == slice.nodes {
            matches.push(binding);
        }
    }
    if matches.is_empty() {
        Err(ConsolidationError::NoInternalMatch)
    } else {
        Ok(matches)
    }
}

fn expand_compiled_internal(
    mechanism: &CompiledInternalMechanism,
    binding: &InternalEpisodeBindings,
) -> Result<Vec<ProofNode>, ConsolidationError> {
    if binding.selected_nodes.len() != mechanism.pattern.len()
        || binding.boundary_nodes.len() != mechanism.boundary_types.len()
        || binding.values.len() != mechanism.parameter_types.len()
        || binding
            .values
            .iter()
            .zip(&mechanism.parameter_types)
            .any(|(value, ty)| value.ty != *ty)
    {
        return Err(ConsolidationError::ReplayMismatch);
    }
    validate_compiled_internal_pattern(
        &mechanism.pattern,
        &mechanism.boundary_types,
        &mechanism.parameter_types,
    )?;
    mechanism
        .pattern
        .iter()
        .map(|node| {
            let inputs = node
                .inputs
                .iter()
                .map(|input| {
                    let (index, values) = match input {
                        InternalPatternInput::Internal(index) => (*index, &binding.selected_nodes),
                        InternalPatternInput::Boundary(index) => (*index, &binding.boundary_nodes),
                    };
                    values
                        .get(usize::try_from(index).map_err(|_| ConsolidationError::Capacity)?)
                        .copied()
                        .ok_or(ConsolidationError::ReplayMismatch)
                })
                .collect::<Result<Vec<_>, _>>()?;
            let arguments = node
                .arguments
                .iter()
                .map(|argument| match argument {
                    GraphPatternArgument::Fixed(value) => Ok(*value),
                    GraphPatternArgument::Parameter(index) => binding
                        .values
                        .get(usize::try_from(*index).map_err(|_| ConsolidationError::Capacity)?)
                        .copied()
                        .ok_or(ConsolidationError::ReplayMismatch),
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(ProofNode {
                operation: node.operation.clone(),
                output_type: node.output_type,
                inputs,
                arguments,
            })
        })
        .collect()
}

fn internal_compression_costs(
    slices: &[InternalSourceSlice],
    pattern: &[InternalGraphPatternNode],
    boundary_types: &[ProofValueType],
    parameter_columns: &[Vec<ProofDatum>],
) -> Result<(u64, u64), ConsolidationError> {
    let uncompressed = slices.iter().try_fold(0_u64, |total, slice| {
        let units = slice.nodes.iter().try_fold(0_u64, |node_total, node| {
            node_total
                .checked_add(1)
                .and_then(|value| value.checked_add(u64::try_from(node.arguments.len()).ok()?))
        })?;
        total.checked_add(units)
    });
    let uncompressed = uncompressed.ok_or(ConsolidationError::Capacity)?;
    let fixed_arguments = pattern.iter().try_fold(0_u64, |total, node| {
        total.checked_add(
            u64::try_from(
                node.arguments
                    .iter()
                    .filter(|argument| matches!(argument, GraphPatternArgument::Fixed(_)))
                    .count(),
            )
            .ok()?,
        )
    });
    let fixed_arguments = fixed_arguments.ok_or(ConsolidationError::Capacity)?;
    let parameters =
        u64::try_from(parameter_columns.len()).map_err(|_| ConsolidationError::Capacity)?;
    let boundaries =
        u64::try_from(boundary_types.len()).map_err(|_| ConsolidationError::Capacity)?;
    let episodes = u64::try_from(slices.len()).map_err(|_| ConsolidationError::Capacity)?;
    let compressed = u64::try_from(pattern.len())
        .map_err(|_| ConsolidationError::Capacity)?
        .checked_add(fixed_arguments)
        .and_then(|value| value.checked_add(parameters))
        .and_then(|value| value.checked_add(boundaries))
        .and_then(|value| {
            value.checked_add(parameters.checked_add(boundaries)?.checked_mul(episodes)?)
        })
        .ok_or(ConsolidationError::Capacity)?;
    Ok((uncompressed, compressed))
}

fn internal_mechanism_hash(
    pattern: &[InternalGraphPatternNode],
    boundary_types: &[ProofValueType],
    parameter_types: &[ProofValueType],
    output: u32,
) -> Result<Hash32, ConsolidationError> {
    internal_mechanism_value(pattern, boundary_types, parameter_types, output)
        .encode_canonical()
        .map(|bytes| Hash32::digest(&bytes))
        .map_err(|_| ConsolidationError::Canonicalization)
}

fn internal_mechanism_value(
    pattern: &[InternalGraphPatternNode],
    boundary_types: &[ProofValueType],
    parameter_types: &[ProofValueType],
    output: u32,
) -> CanonicalValue {
    CanonicalValue::Array(vec![
        CanonicalValue::Text("cairn.consolidator.compiled-internal-mechanism.v1".to_owned()),
        CanonicalValue::Array(
            boundary_types
                .iter()
                .map(|ty| CanonicalValue::Unsigned(proof_type_code(*ty)))
                .collect(),
        ),
        CanonicalValue::Array(
            parameter_types
                .iter()
                .map(|ty| CanonicalValue::Unsigned(proof_type_code(*ty)))
                .collect(),
        ),
        CanonicalValue::Unsigned(u64::from(output)),
        CanonicalValue::Array(
            pattern
                .iter()
                .map(|node| {
                    CanonicalValue::Array(vec![
                        CanonicalValue::Text(node.operation.clone()),
                        CanonicalValue::Unsigned(proof_type_code(node.output_type)),
                        CanonicalValue::Array(
                            node.inputs
                                .iter()
                                .copied()
                                .map(internal_input_value)
                                .collect(),
                        ),
                        CanonicalValue::Array(
                            node.arguments
                                .iter()
                                .map(|argument| match argument {
                                    GraphPatternArgument::Fixed(value) => {
                                        CanonicalValue::Array(vec![
                                            CanonicalValue::Unsigned(0),
                                            CanonicalValue::Unsigned(proof_type_code(value.ty)),
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
    ])
}

fn internal_proposal_hash(
    pattern: &[InternalGraphPatternNode],
    boundary_types: &[ProofValueType],
    episodes: &[Hash32],
    uncompressed: u64,
    compressed: u64,
) -> Result<Hash32, ConsolidationError> {
    let value = CanonicalValue::Array(vec![
        CanonicalValue::Text("cairn.consolidator.internal-compression.v1".to_owned()),
        CanonicalValue::Array(
            boundary_types
                .iter()
                .map(|ty| CanonicalValue::Unsigned(proof_type_code(*ty)))
                .collect(),
        ),
        CanonicalValue::Array(
            pattern
                .iter()
                .map(|node| {
                    CanonicalValue::Array(vec![
                        CanonicalValue::Text(node.operation.clone()),
                        CanonicalValue::Unsigned(proof_type_code(node.output_type)),
                        CanonicalValue::Array(
                            node.inputs
                                .iter()
                                .copied()
                                .map(internal_input_value)
                                .collect(),
                        ),
                        CanonicalValue::Array(
                            node.arguments
                                .iter()
                                .map(|argument| match argument {
                                    GraphPatternArgument::Fixed(value) => {
                                        CanonicalValue::Array(vec![
                                            CanonicalValue::Unsigned(0),
                                            CanonicalValue::Unsigned(proof_type_code(value.ty)),
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
        .map_err(|_| ConsolidationError::Canonicalization)
}

fn internal_input_value(input: InternalPatternInput) -> CanonicalValue {
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

const fn proof_value_type(value_type: EvidenceValueType) -> ProofValueType {
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

fn observation_frontier_commitment(
    frontier: &ProofFrontier,
    provenance: &[ObservationFrontierProvenance],
) -> Result<Hash32, ConsolidationError> {
    if frontier.nodes.len() != provenance.len() {
        return Err(ConsolidationError::InvalidFrontier);
    }
    let nodes = frontier
        .nodes
        .iter()
        .map(|node| {
            CanonicalValue::Array(vec![
                CanonicalValue::Text(node.operation.clone()),
                CanonicalValue::Unsigned(proof_type_code(node.output_type)),
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
                                CanonicalValue::Unsigned(proof_type_code(argument.ty)),
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
    .map_err(|_| ConsolidationError::Canonicalization)
}

const fn proof_type_code(value: ProofValueType) -> u64 {
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

fn proof_graph_value(graph: &ProofGraph) -> CanonicalValue {
    CanonicalValue::Array(vec![
        CanonicalValue::Text("cairn.consolidator.proof-graph.v1".to_owned()),
        CanonicalValue::Bytes(graph.episode.as_bytes().to_vec()),
        CanonicalValue::Array(
            graph
                .nodes
                .iter()
                .map(|node| {
                    CanonicalValue::Array(vec![
                        CanonicalValue::Text(node.operation.clone()),
                        CanonicalValue::Unsigned(proof_type_code(node.output_type)),
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
                                        CanonicalValue::Unsigned(proof_type_code(argument.ty)),
                                        CanonicalValue::Bytes(argument.value.as_bytes().to_vec()),
                                    ])
                                })
                                .collect(),
                        ),
                    ])
                })
                .collect(),
        ),
        CanonicalValue::Unsigned(u64::from(graph.output)),
    ])
}

fn decode_proof_node(value: &CanonicalValue) -> Result<ProofNode, ProofGraphDecodeError> {
    let CanonicalValue::Array(values) = value else {
        return Err(ProofGraphDecodeError::InvalidShape("node is not an array"));
    };
    let [operation, output_type, inputs, arguments] = values.as_slice() else {
        return Err(ProofGraphDecodeError::InvalidShape(
            "node field count is not four",
        ));
    };
    let CanonicalValue::Text(operation) = operation else {
        return Err(ProofGraphDecodeError::InvalidShape("operation is not text"));
    };
    if operation.is_empty() || operation.len() > 256 {
        return Err(ProofGraphDecodeError::InvalidShape(
            "operation length is invalid",
        ));
    }
    let CanonicalValue::Array(inputs) = inputs else {
        return Err(ProofGraphDecodeError::InvalidShape(
            "inputs is not an array",
        ));
    };
    let CanonicalValue::Array(arguments) = arguments else {
        return Err(ProofGraphDecodeError::InvalidShape(
            "arguments is not an array",
        ));
    };
    Ok(ProofNode {
        operation: operation.clone(),
        output_type: decode_proof_type(output_type)?,
        inputs: inputs
            .iter()
            .map(decode_u32)
            .collect::<Result<Vec<_>, _>>()?,
        arguments: arguments
            .iter()
            .map(|argument| {
                let CanonicalValue::Array(values) = argument else {
                    return Err(ProofGraphDecodeError::InvalidShape(
                        "argument is not an array",
                    ));
                };
                let [ty, value] = values.as_slice() else {
                    return Err(ProofGraphDecodeError::InvalidShape(
                        "argument field count is not two",
                    ));
                };
                Ok(ProofDatum {
                    ty: decode_proof_type(ty)?,
                    value: decode_hash(value)?,
                })
            })
            .collect::<Result<Vec<_>, _>>()?,
    })
}

fn decode_proof_type(value: &CanonicalValue) -> Result<ProofValueType, ProofGraphDecodeError> {
    match decode_u64(value)? {
        0 => Ok(ProofValueType::Boolean),
        1 => Ok(ProofValueType::Integer),
        2 => Ok(ProofValueType::Atom),
        3 => Ok(ProofValueType::Entity),
        4 => Ok(ProofValueType::EntitySet),
        5 => Ok(ProofValueType::Relation),
        6 => Ok(ProofValueType::Action),
        7 => Ok(ProofValueType::Effect),
        8 => Ok(ProofValueType::Mechanism),
        code => Err(ProofGraphDecodeError::UnknownType(code)),
    }
}

fn decode_hash(value: &CanonicalValue) -> Result<Hash32, ProofGraphDecodeError> {
    let CanonicalValue::Bytes(bytes) = value else {
        return Err(ProofGraphDecodeError::InvalidHash);
    };
    let fixed: [u8; 32] = bytes
        .as_slice()
        .try_into()
        .map_err(|_| ProofGraphDecodeError::InvalidHash)?;
    Ok(Hash32::from_bytes(fixed))
}

fn decode_u64(value: &CanonicalValue) -> Result<u64, ProofGraphDecodeError> {
    let CanonicalValue::Unsigned(value) = value else {
        return Err(ProofGraphDecodeError::InvalidShape(
            "integer is not unsigned",
        ));
    };
    Ok(*value)
}

fn decode_u32(value: &CanonicalValue) -> Result<u32, ProofGraphDecodeError> {
    u32::try_from(decode_u64(value)?).map_err(|_| ProofGraphDecodeError::IntegerOverflow)
}

fn proof_graph_schema() -> Result<SchemaRef, ArtifactError> {
    SchemaRef::new(PROOF_GRAPH_SCHEMA, PROOF_GRAPH_SCHEMA_VERSION)
}

fn proof_graph_kind() -> Result<ArtifactKind, ArtifactError> {
    ArtifactKind::new("consolidator.proof-graph")
}

const fn proof_graph_semantics() -> SemanticsVersion {
    SemanticsVersion::new(1, 0, 0)
}
