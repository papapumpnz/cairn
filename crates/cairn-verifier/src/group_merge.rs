use cairn_core::Hash32;
use cairn_rkl::{
    ExecLimits, ReferenceExecutor, RuntimeEnvironment, TraceLevel, TypeChecker, TypeEnvironment,
    Value, VarName,
};
use cairn_theory::{GroupMergeTrace, LearnedGroupMerge};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedGroupMerge {
    mechanism: Hash32,
    experiences: BTreeSet<Hash32>,
    replayed_groups: u64,
    group_counts: BTreeSet<usize>,
    group_cardinalities: BTreeSet<usize>,
}

impl VerifiedGroupMerge {
    #[must_use]
    pub const fn mechanism(&self) -> Hash32 {
        self.mechanism
    }
    #[must_use]
    pub const fn experiences(&self) -> &BTreeSet<Hash32> {
        &self.experiences
    }
    #[must_use]
    pub const fn replayed_groups(&self) -> u64 {
        self.replayed_groups
    }
    #[must_use]
    pub const fn group_counts(&self) -> &BTreeSet<usize> {
        &self.group_counts
    }
    #[must_use]
    pub const fn group_cardinalities(&self) -> &BTreeSet<usize> {
        &self.group_cardinalities
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GroupMergeVerifyError {
    InsufficientEvidence,
    DuplicateExperience,
    InvalidTrace,
    Definition,
    ReplayMismatch,
    Capacity,
    Canonicalization,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct GroupMergeVerifier;

impl GroupMergeVerifier {
    /// Independently recompute group aggregates and execute raw RKL.
    ///
    /// # Errors
    ///
    /// Rejects malformed or duplicate evidence, overflow, definition failure,
    /// and exact output-set mismatch.
    pub fn verify(
        &self,
        learned: &LearnedGroupMerge,
        traces: &[GroupMergeTrace],
        limits: ExecLimits,
    ) -> Result<VerifiedGroupMerge, GroupMergeVerifyError> {
        if traces.len() < 2 {
            return Err(GroupMergeVerifyError::InsufficientEvidence);
        }
        let definition = &learned.constructor.proposal.definition;
        let mut types = TypeEnvironment::new();
        for parameter in &definition.parameters {
            types.insert(parameter.name.clone(), parameter.ty.clone());
        }
        let checked = TypeChecker::new(10_000)
            .check(&definition.body, &types)
            .map_err(|_| GroupMergeVerifyError::Definition)?;
        let executor = ReferenceExecutor::without_definitions(limits, TraceLevel::Off);
        let mut receipt = VerifiedGroupMerge {
            mechanism: learned
                .semantic_hash()
                .map_err(|_| GroupMergeVerifyError::Canonicalization)?,
            experiences: BTreeSet::new(),
            replayed_groups: 0,
            group_counts: BTreeSet::new(),
            group_cardinalities: BTreeSet::new(),
        };
        for trace in traces {
            if trace.groups.is_empty() || trace.outputs.len() != trace.groups.len() {
                return Err(GroupMergeVerifyError::InvalidTrace);
            }
            if !receipt.experiences.insert(trace.episode) {
                return Err(GroupMergeVerifyError::DuplicateExperience);
            }
            let mut actual = BTreeSet::new();
            for group in &trace.groups {
                let minimum = group
                    .first()
                    .copied()
                    .ok_or(GroupMergeVerifyError::InvalidTrace)?;
                let maximum = group
                    .last()
                    .copied()
                    .ok_or(GroupMergeVerifyError::InvalidTrace)?;
                let sum = group
                    .iter()
                    .try_fold(0_i64, |sum, value| sum.checked_add(*value))
                    .ok_or(GroupMergeVerifyError::Capacity)?;
                let count =
                    i64::try_from(group.len()).map_err(|_| GroupMergeVerifyError::Capacity)?;
                let mut environment = RuntimeEnvironment::new();
                for (name, value) in [
                    ("group_count", count),
                    ("group_sum", sum),
                    ("group_minimum", minimum),
                    ("group_maximum", maximum),
                ] {
                    environment.insert(VarName(name.to_owned()), Value::Int(value));
                }
                match executor
                    .execute(&checked, &environment)
                    .map_err(|_| GroupMergeVerifyError::Definition)?
                    .result
                {
                    Value::Int(value) => {
                        actual.insert(value);
                    }
                    _ => return Err(GroupMergeVerifyError::Definition),
                }
                receipt.group_cardinalities.insert(group.len());
            }
            if actual != trace.outputs {
                return Err(GroupMergeVerifyError::ReplayMismatch);
            }
            receipt.replayed_groups = receipt
                .replayed_groups
                .checked_add(
                    u64::try_from(trace.groups.len())
                        .map_err(|_| GroupMergeVerifyError::Capacity)?,
                )
                .ok_or(GroupMergeVerifyError::Capacity)?;
            receipt.group_counts.insert(trace.groups.len());
        }
        Ok(receipt)
    }
}
