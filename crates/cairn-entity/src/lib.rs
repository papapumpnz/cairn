//! Demand-driven entities defined only by evidence-graph relations.

use cairn_reg::{AttrKey, AttrValue, NodeId, RegError, RelGraph, RelationId};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum EntityExpr {
    ObservedMembers,
    Connected { adjacency: RelationId },
    GroupByAttribute { key: AttrKey },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EntityHypothesis {
    pub expression: EntityExpr,
    pub members: BTreeSet<NodeId>,
    pub structural_cost: u16,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EntityDemand {
    pub members: BTreeSet<NodeId>,
    pub adjacency_relations: Vec<RelationId>,
    pub grouping_attributes: Vec<AttrKey>,
    pub maximum_cost: u16,
    pub maximum_hypotheses: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DemandResult {
    pub hypotheses: Vec<EntityHypothesis>,
    pub expressions_evaluated: u64,
    pub capacity_reached: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EntityError {
    EmptyDemand,
    EmptyCapacity,
    Graph(RegError),
}

#[derive(Clone, Copy, Debug, Default)]
pub struct EntityGenerator;

impl EntityGenerator {
    /// Enumerate demanded entity denotations in deterministic cost order.
    ///
    /// # Errors
    ///
    /// Rejects empty demands/caps or malformed graph references.
    pub fn generate(
        &self,
        graph: &RelGraph,
        demand: &EntityDemand,
    ) -> Result<DemandResult, EntityError> {
        if demand.members.is_empty() {
            return Err(EntityError::EmptyDemand);
        }
        if demand.maximum_hypotheses == 0 || demand.maximum_cost == 0 {
            return Err(EntityError::EmptyCapacity);
        }
        for member in &demand.members {
            if graph.node(*member).is_none() {
                return Err(EntityError::Graph(RegError::UnknownNode(*member)));
            }
        }
        let mut candidates = vec![EntityHypothesis {
            expression: EntityExpr::ObservedMembers,
            members: demand.members.clone(),
            structural_cost: 1,
        }];
        let mut evaluated = 1_u64;
        if demand.maximum_cost >= 2 {
            for relation in &demand.adjacency_relations {
                evaluated = evaluated.saturating_add(1);
                for component in components(graph, &demand.members, *relation)? {
                    candidates.push(EntityHypothesis {
                        expression: EntityExpr::Connected {
                            adjacency: *relation,
                        },
                        members: component,
                        structural_cost: 2,
                    });
                }
            }
            for key in &demand.grouping_attributes {
                evaluated = evaluated.saturating_add(1);
                let mut groups = BTreeMap::<AttrValue, BTreeSet<NodeId>>::new();
                for member in &demand.members {
                    let node = graph
                        .node(*member)
                        .ok_or(EntityError::Graph(RegError::UnknownNode(*member)))?;
                    if let Some(attribute) = node.attributes.iter().find(|item| item.key == *key) {
                        groups
                            .entry(attribute.value.clone())
                            .or_default()
                            .insert(*member);
                    }
                }
                for members in groups.into_values() {
                    candidates.push(EntityHypothesis {
                        expression: EntityExpr::GroupByAttribute { key: *key },
                        members,
                        structural_cost: 2,
                    });
                }
            }
        }
        candidates.sort_by(|left, right| {
            (left.structural_cost, &left.expression, &left.members).cmp(&(
                right.structural_cost,
                &right.expression,
                &right.members,
            ))
        });
        candidates.dedup_by(|left, right| left.members == right.members);
        let capacity_reached = candidates.len() > demand.maximum_hypotheses;
        candidates.truncate(demand.maximum_hypotheses);
        Ok(DemandResult {
            hypotheses: candidates,
            expressions_evaluated: evaluated,
            capacity_reached,
        })
    }
}

fn components(
    graph: &RelGraph,
    members: &BTreeSet<NodeId>,
    relation: RelationId,
) -> Result<Vec<BTreeSet<NodeId>>, EntityError> {
    let definition = graph
        .relation(relation)
        .ok_or(EntityError::Graph(RegError::UnknownRelation(relation)))?;
    if definition.arguments.len() != 2 {
        return Err(EntityError::Graph(RegError::PathRequiresBinaryRelation(
            relation,
        )));
    }
    let index = graph
        .relation_index(relation)
        .ok_or(EntityError::Graph(RegError::UnknownRelation(relation)))?;
    let mut neighbors = BTreeMap::<NodeId, BTreeSet<NodeId>>::new();
    for fact_id in index.all() {
        let fact = graph
            .fact(*fact_id)
            .ok_or(EntityError::Graph(RegError::Capacity {
                resource: "relation index integrity",
                requested: u64::from(fact_id.0),
                maximum: graph.fact_count() as u64,
            }))?;
        let left = fact.arguments[0];
        let right = fact.arguments[1];
        if members.contains(&left) && members.contains(&right) {
            neighbors.entry(left).or_default().insert(right);
            neighbors.entry(right).or_default().insert(left);
        }
    }
    let mut unseen = members.clone();
    let mut output = Vec::new();
    while let Some(seed) = unseen.pop_first() {
        let mut component = BTreeSet::from([seed]);
        let mut queue = VecDeque::from([seed]);
        while let Some(node) = queue.pop_front() {
            for neighbor in neighbors.get(&node).into_iter().flatten() {
                if unseen.remove(neighbor) {
                    component.insert(*neighbor);
                    queue.push_back(*neighbor);
                }
            }
        }
        output.push(component);
    }
    Ok(output)
}
