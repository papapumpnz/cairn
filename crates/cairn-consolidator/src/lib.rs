//! Offline proof-subgraph compression separated from online task solving.

use cairn_cas::{Cas, CasError, StoredArtifact};
use cairn_core::{
    ArtifactEnvelope, ArtifactError, ArtifactId, ArtifactKind, BlobId, BudgetRecord,
    CanonicalEncode, CanonicalError, CanonicalLimits, CanonicalValue, Hash32, ProducerRecord,
    SchemaDescriptor, SchemaRef, SchemaRegistry, SemanticsVersion,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

pub const CONSOLIDATOR_SEMANTICS_VERSION: &str = "2.2.0";
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
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Consolidator;

impl Consolidator {
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
        if maximum_windows == 0 || proposal.pattern.is_empty() {
            return Err(ConsolidationError::InvalidMiningLimits);
        }
        normalize_ancestral_subgraph(graph)?;
        let width = proposal.pattern.len();
        if width > graph.nodes.len() {
            return Err(ConsolidationError::NoInternalMatch);
        }
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
            if slice.boundary_types != proposal.boundary_types {
                continue;
            }
            let Some(values) =
                bind_internal_parameters(&proposal.pattern, &slice.nodes, &parameter_types)?
            else {
                continue;
            };
            let binding = InternalEpisodeBindings {
                episode: graph.episode,
                selected_nodes: slice.selected_nodes,
                boundary_nodes: slice.boundary_nodes,
                values,
            };
            if instantiate_internal_pattern(&proposal.pattern, &binding)? == slice.nodes {
                matches.push(binding);
            }
        }
        if matches.is_empty() {
            Err(ConsolidationError::NoInternalMatch)
        } else {
            Ok(matches)
        }
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
