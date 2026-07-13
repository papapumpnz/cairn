use crate::{GroupMergeError, GroupMergeInventor, GroupMergeTrace, LearnedGroupMerge};
use cairn_core::{Hash32, RelationRole};
use cairn_invent::InventionLimits;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GroupingHypothesisTrace {
    pub episode: Hash32,
    pub candidates: BTreeMap<RelationRole, Vec<BTreeSet<i64>>>,
    pub outputs: BTreeSet<i64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LearnedGroupingMerge {
    pub relation: RelationRole,
    pub merge: LearnedGroupMerge,
    pub relations_evaluated: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GroupingSelectionError {
    InsufficientEpisodes,
    NoSharedRelation,
    NoExactGrouping,
    MissingRelation,
    Ambiguous,
    Merge(GroupMergeError),
}

#[derive(Clone, Copy, Debug, Default)]
pub struct GroupingRelationInventor;

impl GroupingRelationInventor {
    /// Jointly select a relation-defined grouping and invent its merge rule.
    ///
    /// # Errors
    ///
    /// Rejects insufficient evidence, absent common roles, no exact theory, or
    /// equal-cost semantic ambiguity.
    pub fn invent(
        &self,
        traces: &[GroupingHypothesisTrace],
        limits: InventionLimits,
    ) -> Result<LearnedGroupingMerge, GroupingSelectionError> {
        if traces.len() < 2 {
            return Err(GroupingSelectionError::InsufficientEpisodes);
        }
        let mut roles = traces[0]
            .candidates
            .keys()
            .cloned()
            .collect::<BTreeSet<_>>();
        for trace in &traces[1..] {
            roles = roles
                .intersection(&trace.candidates.keys().cloned().collect())
                .cloned()
                .collect();
        }
        if roles.is_empty() {
            return Err(GroupingSelectionError::NoSharedRelation);
        }
        let evaluated = roles.len();
        let mut exact = Vec::new();
        for role in roles {
            let merge_traces = traces
                .iter()
                .map(|trace| GroupMergeTrace {
                    episode: trace.episode,
                    groups: trace.candidates[&role].clone(),
                    outputs: trace.outputs.clone(),
                })
                .collect::<Vec<_>>();
            if let Ok(merge) = GroupMergeInventor.invent(&merge_traces, limits) {
                exact.push((merge.constructor.proposal.structural_cost, role, merge));
            }
        }
        exact.sort_by(|left, right| {
            left.0
                .cmp(&right.0)
                .then_with(|| left.1.cmp(&right.1))
                .then_with(|| {
                    left.2
                        .constructor
                        .proposal
                        .semantic_hash
                        .cmp(&right.2.constructor.proposal.semantic_hash)
                })
        });
        if exact.len() > 1 && exact[0].0 == exact[1].0 {
            return Err(GroupingSelectionError::Ambiguous);
        }
        let Some((cost, relation, merge)) = exact.into_iter().next() else {
            return Err(GroupingSelectionError::NoExactGrouping);
        };
        if traces
            .iter()
            .any(|trace| !trace.candidates.contains_key(&relation))
        {
            return Err(GroupingSelectionError::MissingRelation);
        }
        if cost == 0 {
            return Err(GroupingSelectionError::Ambiguous);
        }
        Ok(LearnedGroupingMerge {
            relation,
            merge,
            relations_evaluated: evaluated,
        })
    }

    /// Rebind the learned relation and execute its aggregate.
    ///
    /// # Errors
    ///
    /// Rejects a missing relation or merge execution failure.
    pub fn predict(
        &self,
        learned: &LearnedGroupingMerge,
        candidates: &BTreeMap<RelationRole, Vec<BTreeSet<i64>>>,
    ) -> Result<BTreeSet<i64>, GroupingSelectionError> {
        let groups = candidates
            .get(&learned.relation)
            .ok_or(GroupingSelectionError::MissingRelation)?;
        GroupMergeInventor
            .merge(&learned.merge, groups)
            .map_err(GroupingSelectionError::Merge)
    }
}
