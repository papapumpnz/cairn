//! Independent, synthesis-free replay verification for checked CAIRN theories.

use cairn_core::{CanonicalEncode, CanonicalValue, Hash32};
use cairn_rkl::{
    canonical_runtime_value, CheckedTheory, ReferenceTheoryExecutor, RenderedObservation,
    TheoryExecError, TheoryExecLimits, TheoryExecutionRequest, TheoryExecutionTrace,
    RKL_SEMANTICS_VERSION, THEORY_EXECUTION_SEMANTICS_VERSION,
};

pub const VERIFIER_SEMANTICS_VERSION: &str = "1.0.0";

#[derive(Clone, Debug)]
pub struct DemonstrationReplayCase {
    pub evidence_hash: Hash32,
    pub request: TheoryExecutionRequest,
    pub expected: RenderedObservation,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DemonstrationReplay {
    pub evidence_hash: Hash32,
    pub output_hash: Hash32,
    pub rule_firings: u64,
    pub expression_steps: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DemonstrationReplayCertificate {
    pub verifier_semantics: String,
    pub rkl_semantics: String,
    pub execution_semantics: String,
    pub theory_hash: Hash32,
    pub demonstrations: Vec<DemonstrationReplay>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PredictionCertificate {
    pub verifier_semantics: String,
    pub rkl_semantics: String,
    pub execution_semantics: String,
    pub theory_hash: Hash32,
    pub evidence_hash: Hash32,
    pub output_hash: Hash32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerificationReport {
    pub theory_hash: Hash32,
    pub verified_cases: u64,
    pub expression_steps: u64,
    pub rule_firings: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum VerifyError {
    CanonicalTheory,
    SemanticsMismatch,
    TheoryMismatch,
    EvidenceMismatch,
    PredictionMismatch,
    Execution(TheoryExecError),
    Capacity,
}

impl From<TheoryExecError> for VerifyError {
    fn from(value: TheoryExecError) -> Self {
        Self::Execution(value)
    }
}

pub trait IndependentVerifier {
    /// Replay all demonstrations without access to synthesis or memory.
    ///
    /// # Errors
    ///
    /// Rejects execution failures or any exact output mismatch.
    fn verify_demonstrations(
        &self,
        theory: &CheckedTheory,
        cases: &[DemonstrationReplayCase],
        limits: TheoryExecLimits,
    ) -> Result<(DemonstrationReplayCertificate, VerificationReport), VerifyError>;

    /// Replay and validate one previously issued prediction certificate.
    ///
    /// # Errors
    ///
    /// Rejects semantic, theory, evidence, execution, or output mismatch.
    fn verify_prediction(
        &self,
        theory: &CheckedTheory,
        evidence_hash: Hash32,
        request: &TheoryExecutionRequest,
        certificate: &PredictionCertificate,
        limits: TheoryExecLimits,
    ) -> Result<VerificationReport, VerifyError>;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ReplayVerifier;

impl ReplayVerifier {
    /// Issue a prediction certificate after independent reference replay.
    ///
    /// # Errors
    ///
    /// Returns canonicalization or execution failure.
    pub fn certify_prediction(
        &self,
        theory: &CheckedTheory,
        evidence_hash: Hash32,
        request: &TheoryExecutionRequest,
        limits: TheoryExecLimits,
    ) -> Result<PredictionCertificate, VerifyError> {
        let theory_hash = theory_hash(theory)?;
        let trace = ReferenceTheoryExecutor.execute(theory, request, limits)?;
        Ok(PredictionCertificate {
            verifier_semantics: VERIFIER_SEMANTICS_VERSION.to_owned(),
            rkl_semantics: RKL_SEMANTICS_VERSION.to_owned(),
            execution_semantics: THEORY_EXECUTION_SEMANTICS_VERSION.to_owned(),
            theory_hash,
            evidence_hash,
            output_hash: output_hash(&trace),
        })
    }
}

impl IndependentVerifier for ReplayVerifier {
    fn verify_demonstrations(
        &self,
        theory: &CheckedTheory,
        cases: &[DemonstrationReplayCase],
        limits: TheoryExecLimits,
    ) -> Result<(DemonstrationReplayCertificate, VerificationReport), VerifyError> {
        let theory_hash = theory_hash(theory)?;
        let mut demonstrations = Vec::with_capacity(cases.len());
        let mut report = VerificationReport {
            theory_hash,
            verified_cases: 0,
            expression_steps: 0,
            rule_firings: 0,
        };
        for case in cases {
            let trace = ReferenceTheoryExecutor.execute(theory, &case.request, limits)?;
            if trace.rendered != case.expected {
                return Err(VerifyError::PredictionMismatch);
            }
            accumulate(&mut report, &trace)?;
            demonstrations.push(DemonstrationReplay {
                evidence_hash: case.evidence_hash,
                output_hash: output_hash(&trace),
                rule_firings: u64::try_from(trace.rule_firings.len())
                    .map_err(|_| VerifyError::Capacity)?,
                expression_steps: trace.expression_steps,
            });
        }
        Ok((
            DemonstrationReplayCertificate {
                verifier_semantics: VERIFIER_SEMANTICS_VERSION.to_owned(),
                rkl_semantics: RKL_SEMANTICS_VERSION.to_owned(),
                execution_semantics: THEORY_EXECUTION_SEMANTICS_VERSION.to_owned(),
                theory_hash,
                demonstrations,
            },
            report,
        ))
    }

    fn verify_prediction(
        &self,
        theory: &CheckedTheory,
        evidence_hash: Hash32,
        request: &TheoryExecutionRequest,
        certificate: &PredictionCertificate,
        limits: TheoryExecLimits,
    ) -> Result<VerificationReport, VerifyError> {
        validate_versions(
            &certificate.verifier_semantics,
            &certificate.rkl_semantics,
            &certificate.execution_semantics,
        )?;
        let actual_theory = theory_hash(theory)?;
        if certificate.theory_hash != actual_theory {
            return Err(VerifyError::TheoryMismatch);
        }
        if certificate.evidence_hash != evidence_hash {
            return Err(VerifyError::EvidenceMismatch);
        }
        let trace = ReferenceTheoryExecutor.execute(theory, request, limits)?;
        if certificate.output_hash != output_hash(&trace) {
            return Err(VerifyError::PredictionMismatch);
        }
        let mut report = VerificationReport {
            theory_hash: actual_theory,
            verified_cases: 0,
            expression_steps: 0,
            rule_firings: 0,
        };
        accumulate(&mut report, &trace)?;
        Ok(report)
    }
}

fn theory_hash(theory: &CheckedTheory) -> Result<Hash32, VerifyError> {
    theory
        .encode_canonical()
        .map(|bytes| Hash32::digest(&bytes))
        .map_err(|_| VerifyError::CanonicalTheory)
}

fn output_hash(trace: &TheoryExecutionTrace) -> Hash32 {
    rendered_output_hash(&trace.rendered)
}

/// Canonical digest of a rendered observation, shared by certificates and
/// counterexample reports.
///
/// # Panics
///
/// Panics only if the internally constructed, map-free canonical value cannot
/// encode, which indicates a defect in the canonical encoder.
#[must_use]
pub fn rendered_output_hash(rendered: &RenderedObservation) -> Hash32 {
    let value = CanonicalValue::Array(vec![
        CanonicalValue::Text("cairn.rendered-observation.v1".to_owned()),
        canonical_runtime_value(&rendered.canvas),
        canonical_runtime_value(&rendered.background),
        CanonicalValue::Array(
            rendered
                .layers
                .iter()
                .map(canonical_runtime_value)
                .collect(),
        ),
        CanonicalValue::Array(
            rendered
                .writes
                .iter()
                .map(|(target, value)| {
                    CanonicalValue::Array(vec![
                        canonical_runtime_value(target),
                        canonical_runtime_value(value),
                    ])
                })
                .collect(),
        ),
        CanonicalValue::Array(rendered.facts.iter().map(canonical_runtime_value).collect()),
    ]);
    let bytes = value
        .encode_canonical()
        .expect("runtime values always form canonical output");
    Hash32::digest(&bytes)
}

fn validate_versions(verifier: &str, rkl: &str, execution: &str) -> Result<(), VerifyError> {
    if verifier != VERIFIER_SEMANTICS_VERSION
        || rkl != RKL_SEMANTICS_VERSION
        || execution != THEORY_EXECUTION_SEMANTICS_VERSION
    {
        return Err(VerifyError::SemanticsMismatch);
    }
    Ok(())
}

fn accumulate(
    report: &mut VerificationReport,
    trace: &TheoryExecutionTrace,
) -> Result<(), VerifyError> {
    report.verified_cases = report
        .verified_cases
        .checked_add(1)
        .ok_or(VerifyError::Capacity)?;
    report.expression_steps = report
        .expression_steps
        .checked_add(trace.expression_steps)
        .ok_or(VerifyError::Capacity)?;
    report.rule_firings = report
        .rule_firings
        .checked_add(u64::try_from(trace.rule_firings.len()).map_err(|_| VerifyError::Capacity)?)
        .ok_or(VerifyError::Capacity)?;
    Ok(())
}
