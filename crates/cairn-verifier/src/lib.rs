//! Independent promotion boundary combining replay provenance and learning gates.

use cairn_core::CanonicalEncode;
use cairn_core::Hash32;
use cairn_mechanism::{
    admit_persistent, AdmissionError, InventionEvidence, Mechanism, MechanismStatus,
};
pub use cairn_proof::{IndependentVerifier, ReplayVerifier, VerifyError};
use cairn_rkl::CheckedTheory;
use std::collections::BTreeSet;

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
