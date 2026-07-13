use cairn_core::Hash32;
use cairn_rkl::{
    ExecLimits, ReferenceExecutor, RuntimeEnvironment, TraceLevel, TypeChecker, TypeEnvironment,
    Value, VarName,
};
use cairn_theory::{CollectionTrace, LearnedExpansionMechanism};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedExpansionMechanism {
    mechanism: Hash32,
    experiences: BTreeSet<Hash32>,
    replayed_inputs: u64,
    replayed_outputs: u64,
    input_cardinalities: BTreeSet<usize>,
    output_cardinalities: BTreeSet<usize>,
}

impl VerifiedExpansionMechanism {
    #[must_use]
    pub const fn mechanism(&self) -> Hash32 {
        self.mechanism
    }
    #[must_use]
    pub const fn experiences(&self) -> &BTreeSet<Hash32> {
        &self.experiences
    }
    #[must_use]
    pub const fn replayed_inputs(&self) -> u64 {
        self.replayed_inputs
    }
    #[must_use]
    pub const fn replayed_outputs(&self) -> u64 {
        self.replayed_outputs
    }
    #[must_use]
    pub const fn input_cardinalities(&self) -> &BTreeSet<usize> {
        &self.input_cardinalities
    }
    #[must_use]
    pub const fn output_cardinalities(&self) -> &BTreeSet<usize> {
        &self.output_cardinalities
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExpansionVerifyError {
    InsufficientEvidence,
    DuplicateExperience,
    InvalidTrace,
    Definition,
    ReplayMismatch,
    Capacity,
    Canonicalization,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ExpansionMechanismVerifier;

impl ExpansionMechanismVerifier {
    /// Independently execute every `(item, slot)` binding and compare complete
    /// unordered output sets.
    ///
    /// # Errors
    ///
    /// Rejects malformed/duplicate evidence, raw definition failure, capacity,
    /// or exact replay mismatch.
    pub fn verify(
        &self,
        learned: &LearnedExpansionMechanism,
        traces: &[CollectionTrace],
        limits: ExecLimits,
    ) -> Result<VerifiedExpansionMechanism, ExpansionVerifyError> {
        if traces.len() < 2 || learned.multiplicity < 2 {
            return Err(ExpansionVerifyError::InsufficientEvidence);
        }
        let definition = &learned.constructor.proposal.definition;
        let mut types = TypeEnvironment::new();
        for parameter in &definition.parameters {
            types.insert(parameter.name.clone(), parameter.ty.clone());
        }
        let checked = TypeChecker::new(10_000)
            .check(&definition.body, &types)
            .map_err(|_| ExpansionVerifyError::Definition)?;
        let executor = ReferenceExecutor::without_definitions(limits, TraceLevel::Off);
        let mut receipt = VerifiedExpansionMechanism {
            mechanism: learned
                .semantic_hash()
                .map_err(|_| ExpansionVerifyError::Canonicalization)?,
            experiences: BTreeSet::new(),
            replayed_inputs: 0,
            replayed_outputs: 0,
            input_cardinalities: BTreeSet::new(),
            output_cardinalities: BTreeSet::new(),
        };
        for trace in traces {
            if trace.inputs.is_empty()
                || trace.outputs.len()
                    != trace
                        .inputs
                        .len()
                        .saturating_mul(learned.multiplicity as usize)
            {
                return Err(ExpansionVerifyError::InvalidTrace);
            }
            if !receipt.experiences.insert(trace.episode) {
                return Err(ExpansionVerifyError::DuplicateExperience);
            }
            let mut actual = BTreeSet::new();
            for item in &trace.inputs {
                for slot in 0..learned.multiplicity {
                    let mut environment = RuntimeEnvironment::new();
                    environment.insert(VarName("item".to_owned()), Value::Int(*item));
                    environment.insert(VarName("slot".to_owned()), Value::Int(i64::from(slot)));
                    match executor
                        .execute(&checked, &environment)
                        .map_err(|_| ExpansionVerifyError::Definition)?
                        .result
                    {
                        Value::Int(value) => {
                            actual.insert(value);
                        }
                        _ => return Err(ExpansionVerifyError::Definition),
                    }
                }
            }
            if actual != trace.outputs.iter().copied().collect() {
                return Err(ExpansionVerifyError::ReplayMismatch);
            }
            receipt.replayed_inputs = receipt
                .replayed_inputs
                .checked_add(
                    u64::try_from(trace.inputs.len())
                        .map_err(|_| ExpansionVerifyError::Capacity)?,
                )
                .ok_or(ExpansionVerifyError::Capacity)?;
            receipt.replayed_outputs = receipt
                .replayed_outputs
                .checked_add(
                    u64::try_from(trace.outputs.len())
                        .map_err(|_| ExpansionVerifyError::Capacity)?,
                )
                .ok_or(ExpansionVerifyError::Capacity)?;
            receipt.input_cardinalities.insert(trace.inputs.len());
            receipt.output_cardinalities.insert(trace.outputs.len());
        }
        Ok(receipt)
    }
}
