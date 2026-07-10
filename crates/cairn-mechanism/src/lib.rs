//! Universal, modality-neutral learned mechanism representation.
//!
//! A stored definition does not qualify as learning. Admission and promotion
//! require explicit compression, held-out transfer, independent replay, and
//! measured future-work evidence.

use cairn_core::Hash32;
use cairn_rkl::{Binder, Definition, Effect, Expr, RewriteRule, Type};
use std::collections::BTreeSet;

pub const MECHANISM_IR_SEMANTICS_VERSION: &str = "1.0.0";

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct DomainClass(pub String);

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct VariableId(pub String);

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct CardinalityRange {
    pub minimum: u32,
    pub maximum: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SupportGeometry {
    pub observed_domains: BTreeSet<DomainClass>,
    pub observed_cardinalities: BTreeSet<CardinalityRange>,
    pub varied_variables: BTreeSet<VariableId>,
    pub invariant_variables: BTreeSet<VariableId>,
    pub unknown_variables: BTreeSet<VariableId>,
    pub validated_compositions: BTreeSet<Hash32>,
    pub failed_conditions: BTreeSet<Hash32>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Signature {
    pub inputs: Vec<Type>,
    pub output: Type,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DerivationCertificate {
    pub source_proofs: BTreeSet<Hash32>,
    pub verifier_semantics: String,
    pub verified_cases: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Mechanism {
    pub signature: Signature,
    pub variables: Vec<Binder>,
    pub entity_definitions: Vec<Definition>,
    pub preconditions: Expr,
    pub transition_rules: Vec<RewriteRule>,
    pub invariants: Vec<Expr>,
    pub termination: Expr,
    pub effects: Vec<Effect>,
    pub support: SupportGeometry,
    pub proof: DerivationCertificate,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum MechanismStatus {
    Candidate,
    Local,
    Provisional,
    Validated,
    Persistent,
    Compiled,
    Deprecated,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InventionEvidence {
    pub target_absent_from_named_vocabulary: bool,
    pub source_code_changes: u32,
    pub independent_experiences: u32,
    pub initial_success_parts_per_million: u32,
    pub post_experience_success_parts_per_million: u32,
    pub uncompressed_cost: u64,
    pub compressed_cost: u64,
    pub held_out_verified_cases: u64,
    pub candidate_executions_without: u64,
    pub candidate_executions_with: u64,
    pub examples_without: u32,
    pub examples_with: u32,
    pub false_acceptances_without: u32,
    pub false_acceptances_with: u32,
    pub actual_reuse_count: u32,
    pub necessary_reuse_count: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AdmissionError {
    EmptySignature,
    EmptyVariables,
    InvalidCardinalityRange,
    ConflictingSupportAxis(VariableId),
    MissingProof,
    Unverified,
    NamedVocabularyContainsTarget,
    SourceChangedDuringExperiment,
    InsufficientIndependentExperience,
    InitialCapabilityTooHigh,
    PostExperienceCapabilityTooLow,
    NoCompression,
    NoHeldOutPrediction,
    NoSearchReduction,
    NoSampleReduction,
    FalseAcceptanceRegression,
    NoActualReuse,
    NoNecessaryReuse,
}

impl Mechanism {
    /// Validate representation invariants independently of promotion policy.
    ///
    /// # Errors
    ///
    /// Rejects empty typing/proof data and contradictory support claims.
    pub fn validate(&self) -> Result<(), AdmissionError> {
        if self.signature.inputs.is_empty() {
            return Err(AdmissionError::EmptySignature);
        }
        if self.variables.is_empty() {
            return Err(AdmissionError::EmptyVariables);
        }
        if self
            .support
            .observed_cardinalities
            .iter()
            .any(|range| range.minimum > range.maximum)
        {
            return Err(AdmissionError::InvalidCardinalityRange);
        }
        for variable in &self.support.varied_variables {
            if self.support.invariant_variables.contains(variable)
                || self.support.unknown_variables.contains(variable)
            {
                return Err(AdmissionError::ConflictingSupportAxis(variable.clone()));
            }
        }
        for variable in &self.support.invariant_variables {
            if self.support.unknown_variables.contains(variable) {
                return Err(AdmissionError::ConflictingSupportAxis(variable.clone()));
            }
        }
        if self.proof.source_proofs.is_empty() {
            return Err(AdmissionError::MissingProof);
        }
        if self.proof.verified_cases == 0 || self.proof.verifier_semantics.is_empty() {
            return Err(AdmissionError::Unverified);
        }
        Ok(())
    }
}

/// Apply the strict invention and reuse evidence gate from ADR-0007.
///
/// # Errors
///
/// Returns the first deterministic unmet learning criterion.
pub fn admit_persistent(
    mechanism: &Mechanism,
    evidence: &InventionEvidence,
) -> Result<MechanismStatus, AdmissionError> {
    mechanism.validate()?;
    if !evidence.target_absent_from_named_vocabulary {
        return Err(AdmissionError::NamedVocabularyContainsTarget);
    }
    if evidence.source_code_changes != 0 {
        return Err(AdmissionError::SourceChangedDuringExperiment);
    }
    if evidence.independent_experiences < 2 {
        return Err(AdmissionError::InsufficientIndependentExperience);
    }
    if evidence.initial_success_parts_per_million >= 200_000 {
        return Err(AdmissionError::InitialCapabilityTooHigh);
    }
    if evidence.post_experience_success_parts_per_million <= 700_000 {
        return Err(AdmissionError::PostExperienceCapabilityTooLow);
    }
    if evidence.compressed_cost >= evidence.uncompressed_cost {
        return Err(AdmissionError::NoCompression);
    }
    if evidence.held_out_verified_cases == 0 {
        return Err(AdmissionError::NoHeldOutPrediction);
    }
    if evidence.candidate_executions_with >= evidence.candidate_executions_without {
        return Err(AdmissionError::NoSearchReduction);
    }
    if evidence.examples_with >= evidence.examples_without {
        return Err(AdmissionError::NoSampleReduction);
    }
    if evidence.false_acceptances_with > evidence.false_acceptances_without {
        return Err(AdmissionError::FalseAcceptanceRegression);
    }
    if evidence.actual_reuse_count == 0 {
        return Err(AdmissionError::NoActualReuse);
    }
    if evidence.necessary_reuse_count == 0 {
        return Err(AdmissionError::NoNecessaryReuse);
    }
    Ok(MechanismStatus::Persistent)
}
