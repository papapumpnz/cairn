//! Bounded counterexample-guided search over complete checked theories.

use cairn_core::{CanonicalEncode, Hash32};
use cairn_proof::{
    rendered_output_hash, DemonstrationReplayCase, IndependentVerifier, ReplayVerifier,
    VerificationReport,
};
use cairn_rkl::{CheckedTheory, ReferenceTheoryExecutor, TheoryExecLimits};
use std::collections::BTreeSet;

#[derive(Clone, Debug)]
pub struct TheoryCandidate {
    pub theory: CheckedTheory,
    pub structural_cost: u32,
    pub origin: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Counterexample {
    pub candidate_hash: Hash32,
    pub demonstration_index: usize,
    pub expected_hash: Hash32,
    pub actual_hash: Option<Hash32>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CegisLimits {
    pub maximum_candidates: u64,
    pub execution: TheoryExecLimits,
}

#[derive(Clone, Debug)]
pub struct CegisOutcome {
    pub theory: CheckedTheory,
    pub semantic_hash: Hash32,
    pub candidates_executed: u64,
    pub counterexamples: Vec<Counterexample>,
    pub verification: VerificationReport,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CegisError {
    EmptyDemonstrations,
    EmptyCapacity,
    Canonicalization,
    Capacity { maximum: u64 },
    Unrealizable,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct CegisLoop;

impl CegisLoop {
    /// Search candidates in canonical cost order, retaining one minimized first
    /// failing demonstration as the semantic blocking witness.
    ///
    /// # Errors
    ///
    /// Distinguishes empty input, cap exhaustion, and exhausted candidate space.
    pub fn search(
        &self,
        mut candidates: Vec<TheoryCandidate>,
        demonstrations: &[DemonstrationReplayCase],
        limits: CegisLimits,
    ) -> Result<CegisOutcome, CegisError> {
        if demonstrations.is_empty() {
            return Err(CegisError::EmptyDemonstrations);
        }
        if limits.maximum_candidates == 0 {
            return Err(CegisError::EmptyCapacity);
        }
        let mut keyed = candidates
            .drain(..)
            .map(|candidate| {
                theory_hash(&candidate.theory)
                    .map(|hash| (candidate.structural_cost, hash, candidate))
            })
            .collect::<Result<Vec<_>, _>>()?;
        keyed.sort_by_key(|(cost, hash, _)| (*cost, *hash));
        keyed.dedup_by_key(|(_, hash, _)| *hash);
        let mut blocked = BTreeSet::new();
        let mut counterexamples = Vec::new();
        let mut executed = 0_u64;
        for (_, hash, candidate) in keyed {
            if !blocked.insert(hash) {
                continue;
            }
            executed = executed.saturating_add(1);
            if executed > limits.maximum_candidates {
                return Err(CegisError::Capacity {
                    maximum: limits.maximum_candidates,
                });
            }
            let mut mismatch = None;
            for (index, demonstration) in demonstrations.iter().enumerate() {
                let actual = ReferenceTheoryExecutor.execute(
                    &candidate.theory,
                    &demonstration.request,
                    limits.execution,
                );
                if actual
                    .as_ref()
                    .is_ok_and(|trace| trace.rendered == demonstration.expected)
                {
                    continue;
                }
                mismatch = Some(Counterexample {
                    candidate_hash: hash,
                    demonstration_index: index,
                    expected_hash: rendered_output_hash(&demonstration.expected),
                    actual_hash: actual
                        .ok()
                        .map(|trace| rendered_output_hash(&trace.rendered)),
                });
                break;
            }
            if let Some(counterexample) = mismatch {
                counterexamples.push(counterexample);
                continue;
            }
            let (_, verification) = ReplayVerifier
                .verify_demonstrations(&candidate.theory, demonstrations, limits.execution)
                .map_err(|_| CegisError::Unrealizable)?;
            return Ok(CegisOutcome {
                theory: candidate.theory,
                semantic_hash: hash,
                candidates_executed: executed,
                counterexamples,
                verification,
            });
        }
        Err(CegisError::Unrealizable)
    }
}

fn theory_hash(theory: &CheckedTheory) -> Result<Hash32, CegisError> {
    theory
        .encode_canonical()
        .map(|bytes| Hash32::digest(&bytes))
        .map_err(|_| CegisError::Canonicalization)
}
