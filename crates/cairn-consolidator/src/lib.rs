//! Offline proof-subgraph compression separated from online task solving.

use cairn_core::Hash32;

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
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Consolidator;

impl Consolidator {
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
