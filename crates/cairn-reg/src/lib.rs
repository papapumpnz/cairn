//! Typed immutable Relational Evidence Graph.
//!
//! The REG stores generic observations, hypotheses, relations, events, and
//! derivations. It deliberately has no dependency on a spatial frontend,
//! benchmark adapter, RKL, synthesizer, or memory implementation.

mod decode;
mod graph;
mod query;
mod types;

pub use graph::{
    DerivationArena, FactRef, NodeRef, RelGraph, RelGraphBuilder, RelGraphSnapshot, RelationIndex,
};
pub use query::{
    AttributeFilter, GroupCount, PathConstraint, QueryPlan, QueryResult, RelationPattern, Term,
};
pub use types::*;

/// Semantic contract for the first REG artifact format.
pub const REG_SEMANTICS_VERSION: &str = "1.0.0";
