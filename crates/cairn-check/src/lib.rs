//! Enforceable repository policy and runtime attestation for CAIRN.
//!
//! The checker is deliberately separate from inference code. It inspects the
//! resolved Cargo graph, manifests, lockfile, build/configuration files, and
//! production source, returning stable structured violations. It never changes
//! the inspected repository.

mod attest;
mod policy;

pub use attest::{
    collect_attestation, write_attestation, AttestError, ProcessTreeEntry, RuntimeAttestation,
    RuntimeFinding, RuntimeFindingCode,
};
pub use policy::{
    check_repository, CheckError, CheckOptions, CheckReport, Violation, ViolationCode,
};
