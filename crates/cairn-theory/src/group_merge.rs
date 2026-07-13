use cairn_core::{CanonicalEncode, CanonicalValue, Hash32};
use cairn_invent::{
    DefinitionInventor, InventionError, InventionLimits, InventionOutcome, UnorderedMappingEpisode,
    UnorderedMappingProblem,
};
use cairn_rkl::{Binder, IntRange, RuntimeEnvironment, Type, Value, VarName};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GroupMergeTrace {
    pub episode: Hash32,
    pub groups: Vec<BTreeSet<i64>>,
    pub outputs: BTreeSet<i64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LearnedGroupMerge {
    pub constructor: InventionOutcome,
    pub source_episodes: BTreeSet<Hash32>,
    pub observed_group_counts: BTreeSet<usize>,
    pub observed_group_cardinalities: BTreeSet<usize>,
}

impl LearnedGroupMerge {
    /// Return the canonical executable merge identity.
    ///
    /// # Errors
    ///
    /// Returns canonical encoding failure.
    pub fn semantic_hash(&self) -> Result<Hash32, cairn_core::CanonicalError> {
        CanonicalValue::Array(vec![
            CanonicalValue::Text("cairn.group-merge.v1".to_owned()),
            CanonicalValue::Bytes(self.constructor.proposal.definition.encode_canonical()?),
        ])
        .encode_canonical()
        .map(|bytes| Hash32::digest(&bytes))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GroupMergeError {
    InsufficientEpisodes,
    DuplicateEpisode,
    InvalidTrace,
    Capacity,
    Invention(InventionError),
}

#[derive(Clone, Copy, Debug, Default)]
pub struct GroupMergeInventor;

impl GroupMergeInventor {
    /// Invent a many-to-one aggregate constructor without group/output pairing.
    ///
    /// # Errors
    ///
    /// Rejects malformed/duplicate evidence, insufficient group variation,
    /// overflow, and bounded RKL invention failure.
    pub fn invent(
        &self,
        traces: &[GroupMergeTrace],
        limits: InventionLimits,
    ) -> Result<LearnedGroupMerge, GroupMergeError> {
        if traces.len() < 2 {
            return Err(GroupMergeError::InsufficientEpisodes);
        }
        let source_episodes = traces
            .iter()
            .map(|trace| trace.episode)
            .collect::<BTreeSet<_>>();
        if source_episodes.len() != traces.len() {
            return Err(GroupMergeError::DuplicateEpisode);
        }
        if traces.iter().any(|trace| {
            trace.groups.is_empty()
                || trace.outputs.len() != trace.groups.len()
                || trace.groups.iter().any(BTreeSet::is_empty)
        }) {
            return Err(GroupMergeError::InvalidTrace);
        }
        let observed_group_counts = traces
            .iter()
            .map(|trace| trace.groups.len())
            .collect::<BTreeSet<_>>();
        let observed_group_cardinalities = traces
            .iter()
            .flat_map(|trace| trace.groups.iter().map(BTreeSet::len))
            .collect::<BTreeSet<_>>();
        if observed_group_counts.len() < 2 || observed_group_cardinalities.len() < 2 {
            return Err(GroupMergeError::InvalidTrace);
        }
        let episodes = traces
            .iter()
            .map(|trace| {
                Ok(UnorderedMappingEpisode {
                    inputs: trace
                        .groups
                        .iter()
                        .map(group_environment)
                        .collect::<Result<Vec<_>, _>>()?,
                    expected_outputs: trace.outputs.iter().copied().map(Value::Int).collect(),
                })
            })
            .collect::<Result<Vec<_>, GroupMergeError>>()?;
        let constructor = DefinitionInventor
            .invent_unordered_mapping(&UnorderedMappingProblem {
                definition_name: "invented_group_merge".to_owned(),
                parameters: parameters(),
                return_type: integer_type(),
                literals: vec![Value::Int(0), Value::Int(1), Value::Int(2)],
                episodes,
                limits,
            })
            .map_err(GroupMergeError::Invention)?;
        Ok(LearnedGroupMerge {
            constructor,
            source_episodes,
            observed_group_counts,
            observed_group_cardinalities,
        })
    }

    /// Execute the learned merge on current groups.
    ///
    /// # Errors
    ///
    /// Rejects empty groups, overflow, non-integer output, or execution failure.
    pub fn merge(
        &self,
        learned: &LearnedGroupMerge,
        groups: &[BTreeSet<i64>],
    ) -> Result<BTreeSet<i64>, GroupMergeError> {
        groups
            .iter()
            .map(|group| {
                let environment = group_environment(group)?;
                match DefinitionInventor
                    .predict(
                        &learned.constructor.proposal,
                        &environment,
                        cairn_rkl::ExecLimits::default(),
                    )
                    .map_err(GroupMergeError::Invention)?
                {
                    Value::Int(value) => Ok(value),
                    _ => Err(GroupMergeError::InvalidTrace),
                }
            })
            .collect()
    }
}

pub(crate) fn group_environment(
    group: &BTreeSet<i64>,
) -> Result<RuntimeEnvironment, GroupMergeError> {
    let minimum = group
        .first()
        .copied()
        .ok_or(GroupMergeError::InvalidTrace)?;
    let maximum = group.last().copied().ok_or(GroupMergeError::InvalidTrace)?;
    let sum = group
        .iter()
        .try_fold(0_i64, |sum, value| sum.checked_add(*value))
        .ok_or(GroupMergeError::Capacity)?;
    let count = i64::try_from(group.len()).map_err(|_| GroupMergeError::Capacity)?;
    let mut environment = RuntimeEnvironment::new();
    for (name, value) in [
        ("group_count", count),
        ("group_sum", sum),
        ("group_minimum", minimum),
        ("group_maximum", maximum),
    ] {
        environment.insert(VarName(name.to_owned()), Value::Int(value));
    }
    Ok(environment)
}

fn parameters() -> Vec<Binder> {
    ["group_count", "group_sum", "group_minimum", "group_maximum"]
        .into_iter()
        .map(|name| Binder {
            name: VarName(name.to_owned()),
            ty: integer_type(),
        })
        .collect()
}

fn integer_type() -> Type {
    Type::Int(IntRange {
        lower: -1_000_000,
        upper: 1_000_000,
    })
}
