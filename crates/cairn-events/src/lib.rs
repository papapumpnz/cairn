//! Structurally diverse event hypotheses over generic entity denotations.

use cairn_reg::{AttrKey, AttrValue, NodeId};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventEntity {
    pub id: u32,
    pub members: BTreeSet<NodeId>,
    pub attributes: BTreeMap<AttrKey, AttrValue>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum EventKind {
    Preserve,
    Update,
    Create,
    Delete,
    Split,
    Merge,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventHypothesis {
    pub kind: EventKind,
    pub sources: Vec<u32>,
    pub targets: Vec<u32>,
    pub preserved_members: usize,
    pub attribute_deltas: Vec<AttrKey>,
    pub structural_cost: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EventLimits {
    pub maximum_hypotheses: usize,
    pub maximum_arity: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventIndex {
    pub hypotheses: Vec<EventHypothesis>,
    pub candidates_evaluated: u64,
    pub capacity_reached: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EventError {
    EmptyCapacity,
    DuplicateEntity(u32),
}

#[derive(Clone, Copy, Debug, Default)]
pub struct EventGenerator;

impl EventGenerator {
    /// Generate unary and bounded hyperedge explanations without assuming a
    /// modality, geometry, or one-source/one-target correspondence.
    ///
    /// # Errors
    ///
    /// Rejects empty caps and duplicate local entity identities.
    pub fn generate(
        &self,
        before: &[EventEntity],
        after: &[EventEntity],
        limits: EventLimits,
    ) -> Result<EventIndex, EventError> {
        if limits.maximum_hypotheses == 0 || limits.maximum_arity == 0 {
            return Err(EventError::EmptyCapacity);
        }
        validate_unique(before)?;
        validate_unique(after)?;
        let mut hypotheses = Vec::new();
        let mut evaluated = 0_u64;
        let mut matched_before = BTreeSet::new();
        let mut matched_after = BTreeSet::new();
        for source in before {
            for target in after {
                evaluated = evaluated.saturating_add(1);
                let overlap = source.members.intersection(&target.members).count();
                if overlap == 0 {
                    continue;
                }
                let deltas = attribute_deltas(source, target);
                hypotheses.push(EventHypothesis {
                    kind: if source.members == target.members && deltas.is_empty() {
                        EventKind::Preserve
                    } else {
                        EventKind::Update
                    },
                    sources: vec![source.id],
                    targets: vec![target.id],
                    preserved_members: overlap,
                    attribute_deltas: deltas,
                    structural_cost: 1,
                });
                matched_before.insert(source.id);
                matched_after.insert(target.id);
            }
        }
        for source in before {
            if !matched_before.contains(&source.id) {
                hypotheses.push(EventHypothesis {
                    kind: EventKind::Delete,
                    sources: vec![source.id],
                    targets: Vec::new(),
                    preserved_members: 0,
                    attribute_deltas: Vec::new(),
                    structural_cost: 1,
                });
            }
        }
        for target in after {
            if !matched_after.contains(&target.id) {
                hypotheses.push(EventHypothesis {
                    kind: EventKind::Create,
                    sources: Vec::new(),
                    targets: vec![target.id],
                    preserved_members: 0,
                    attribute_deltas: target.attributes.keys().copied().collect(),
                    structural_cost: 1,
                });
            }
        }
        if limits.maximum_arity >= 2 {
            add_hyperedges(before, after, &mut hypotheses, &mut evaluated);
        }
        hypotheses.sort_by(|left, right| {
            (
                left.structural_cost,
                kind_rank(&left.kind),
                &left.sources,
                &left.targets,
            )
                .cmp(&(
                    right.structural_cost,
                    kind_rank(&right.kind),
                    &right.sources,
                    &right.targets,
                ))
        });
        let capacity_reached = hypotheses.len() > limits.maximum_hypotheses;
        hypotheses.truncate(limits.maximum_hypotheses);
        Ok(EventIndex {
            hypotheses,
            candidates_evaluated: evaluated,
            capacity_reached,
        })
    }
}

fn add_hyperedges(
    before: &[EventEntity],
    after: &[EventEntity],
    output: &mut Vec<EventHypothesis>,
    evaluated: &mut u64,
) {
    for source in before {
        let targets = after
            .iter()
            .filter(|target| !source.members.is_disjoint(&target.members))
            .map(|target| target.id)
            .collect::<Vec<_>>();
        *evaluated = evaluated.saturating_add(after.len() as u64);
        if targets.len() > 1 {
            output.push(EventHypothesis {
                kind: EventKind::Split,
                sources: vec![source.id],
                targets,
                preserved_members: source.members.len(),
                attribute_deltas: Vec::new(),
                structural_cost: 2,
            });
        }
    }
    for target in after {
        let sources = before
            .iter()
            .filter(|source| !source.members.is_disjoint(&target.members))
            .map(|source| source.id)
            .collect::<Vec<_>>();
        *evaluated = evaluated.saturating_add(before.len() as u64);
        if sources.len() > 1 {
            output.push(EventHypothesis {
                kind: EventKind::Merge,
                sources,
                targets: vec![target.id],
                preserved_members: target.members.len(),
                attribute_deltas: Vec::new(),
                structural_cost: 2,
            });
        }
    }
}

fn validate_unique(entities: &[EventEntity]) -> Result<(), EventError> {
    let mut ids = BTreeSet::new();
    for entity in entities {
        if !ids.insert(entity.id) {
            return Err(EventError::DuplicateEntity(entity.id));
        }
    }
    Ok(())
}

fn attribute_deltas(source: &EventEntity, target: &EventEntity) -> Vec<AttrKey> {
    source
        .attributes
        .keys()
        .chain(target.attributes.keys())
        .copied()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .filter(|key| source.attributes.get(key) != target.attributes.get(key))
        .collect()
}

const fn kind_rank(kind: &EventKind) -> u8 {
    match kind {
        EventKind::Preserve => 0,
        EventKind::Update => 1,
        EventKind::Create => 2,
        EventKind::Delete => 3,
        EventKind::Split => 4,
        EventKind::Merge => 5,
    }
}
