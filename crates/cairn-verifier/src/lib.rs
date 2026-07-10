//! Independent promotion boundary combining replay provenance and learning gates.

use cairn_mechanism::{
    admit_persistent, AdmissionError, InventionEvidence, Mechanism, MechanismStatus,
};
pub use cairn_proof::{IndependentVerifier, ReplayVerifier, VerifyError};

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
    ) -> Result<MechanismStatus, AdmissionError> {
        admit_persistent(mechanism, evidence)
    }
}
