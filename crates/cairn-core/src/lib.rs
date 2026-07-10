//! Stable low-level identities and canonical artifact representation for CAIRN.
//!
//! This crate contains no inference or domain logic. Its canonical profile is
//! bounded, deterministic, and independently decodable. Observation metadata
//! is deliberately separated from semantic artifact identity by ADR-0001.

mod artifact;
mod canonical;
mod hash;

pub use artifact::{
    AdditiveMapReader, ArtifactEnvelope, ArtifactError, ArtifactKind, BudgetRecord, ProducerRecord,
    SchemaDecision, SchemaDescriptor, SchemaRef, SchemaRegistry, SemanticsVersion,
};
pub use canonical::{
    decode_canonical, CanonicalDecode, CanonicalEncode, CanonicalError, CanonicalLimits,
    CanonicalValue,
};
pub use hash::{ArtifactId, BlobId, Hash32, HashParseError};

/// Artifact protocol implemented by this crate.
pub const ARTIFACT_PROTOCOL_VERSION: u16 = 1;
