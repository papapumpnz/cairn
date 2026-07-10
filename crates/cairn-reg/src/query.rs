use crate::{AttrKey, AttrValue, FactId, NodeId, RegError, RelGraph, RelationId};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Term {
    Variable(u16),
    Node(NodeId),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RelationPattern {
    pub relation: RelationId,
    pub arguments: Vec<Term>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AttributeFilter {
    pub variable: u16,
    pub key: AttrKey,
    pub value: AttrValue,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PathConstraint {
    pub relation: RelationId,
    pub from: Term,
    pub to: Term,
    pub maximum_hops: u16,
    pub maximum_visited: u32,
}

/// Deterministic conjunctive relation query.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QueryPlan {
    pub variable_count: u16,
    pub patterns: Vec<RelationPattern>,
    pub attribute_filters: Vec<AttributeFilter>,
    pub path_constraints: Vec<PathConstraint>,
    pub result_variables: Vec<u16>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GroupCount {
    pub key: Vec<NodeId>,
    pub count: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QueryResult {
    pub columns: Vec<u16>,
    pub rows: Vec<Vec<NodeId>>,
    pub candidate_facts_examined: u64,
}

impl QueryPlan {
    /// Execute a left-deep deterministic join over immutable indexes.
    ///
    /// # Errors
    ///
    /// Rejects unknown relations, arity mismatches, and variables outside the
    /// plan's declared width before returning any result.
    pub fn execute(&self, graph: &RelGraph) -> Result<QueryResult, RegError> {
        self.validate(graph)?;
        let width = usize::from(self.variable_count);
        let mut bindings = vec![vec![None; width]];
        let mut examined = 0_u64;
        for pattern in &self.patterns {
            let mut next = Vec::new();
            for binding in &bindings {
                let candidates = candidate_facts(graph, pattern, binding);
                examined = examined.saturating_add(candidates.len() as u64);
                for fact_id in candidates {
                    let fact = graph.fact(*fact_id).ok_or(RegError::Capacity {
                        resource: "query fact index",
                        requested: u64::from(fact_id.0),
                        maximum: graph.fact_count() as u64,
                    })?;
                    if let Some(extended) = unify(pattern, fact.arguments, binding) {
                        next.push(extended);
                    }
                }
            }
            next.sort();
            next.dedup();
            bindings = next;
            if bindings.is_empty() {
                break;
            }
        }
        bindings.retain(|binding| self.matches_attributes(graph, binding));
        for constraint in &self.path_constraints {
            let mut next = Vec::new();
            for binding in bindings {
                let from = resolve_term(constraint.from, &binding)?;
                let to = resolve_term(constraint.to, &binding)?;
                if bounded_reachable(graph, *constraint, from, to)? {
                    next.push(binding);
                }
            }
            bindings = next;
        }
        let mut rows = bindings
            .into_iter()
            .map(|binding| {
                self.result_variables
                    .iter()
                    .map(|variable| {
                        binding[usize::from(*variable)]
                            .ok_or(RegError::QueryUnboundVariable(*variable))
                    })
                    .collect::<Result<Vec<_>, _>>()
            })
            .collect::<Result<Vec<_>, _>>()?;
        rows.sort();
        rows.dedup();
        Ok(QueryResult {
            columns: self.result_variables.clone(),
            rows,
            candidate_facts_examined: examined,
        })
    }

    fn validate(&self, graph: &RelGraph) -> Result<(), RegError> {
        for variable in &self.result_variables {
            self.validate_variable(*variable)?;
        }
        for filter in &self.attribute_filters {
            self.validate_variable(filter.variable)?;
        }
        for pattern in &self.patterns {
            let definition = graph
                .relation(pattern.relation)
                .ok_or(RegError::UnknownRelation(pattern.relation))?;
            if definition.arguments.len() != pattern.arguments.len() {
                return Err(RegError::QueryArity {
                    relation: pattern.relation,
                    expected: definition.arguments.len(),
                    actual: pattern.arguments.len(),
                });
            }
            for term in &pattern.arguments {
                self.validate_term(graph, *term)?;
            }
        }
        for constraint in &self.path_constraints {
            let definition = graph
                .relation(constraint.relation)
                .ok_or(RegError::UnknownRelation(constraint.relation))?;
            if definition.arguments.len() != 2 {
                return Err(RegError::PathRequiresBinaryRelation(constraint.relation));
            }
            if constraint.maximum_hops == 0 || constraint.maximum_visited == 0 {
                return Err(RegError::EmptyPathBound);
            }
            self.validate_term(graph, constraint.from)?;
            self.validate_term(graph, constraint.to)?;
        }
        Ok(())
    }

    fn validate_term(&self, graph: &RelGraph, term: Term) -> Result<(), RegError> {
        match term {
            Term::Variable(variable) => self.validate_variable(variable),
            Term::Node(node) if graph.node(node).is_none() => Err(RegError::UnknownNode(node)),
            Term::Node(_) => Ok(()),
        }
    }

    fn matches_attributes(&self, graph: &RelGraph, binding: &[Option<NodeId>]) -> bool {
        self.attribute_filters.iter().all(|filter| {
            binding[usize::from(filter.variable)].is_some_and(|node| {
                graph.node(node).is_some_and(|record| {
                    record
                        .attributes
                        .binary_search_by(|attribute| {
                            attribute
                                .key
                                .cmp(&filter.key)
                                .then_with(|| attribute.value.cmp(&filter.value))
                        })
                        .is_ok()
                })
            })
        })
    }

    fn validate_variable(&self, variable: u16) -> Result<(), RegError> {
        if variable >= self.variable_count {
            Err(RegError::QueryVariableOutOfRange(variable))
        } else {
            Ok(())
        }
    }
}

impl QueryResult {
    /// Count rows grouped by selected result-column indexes.
    ///
    /// # Errors
    ///
    /// Rejects a group column outside the result width.
    pub fn group_count(&self, group_columns: &[usize]) -> Result<Vec<GroupCount>, RegError> {
        if let Some(column) = group_columns
            .iter()
            .copied()
            .find(|column| *column >= self.columns.len())
        {
            return Err(RegError::ResultColumnOutOfRange(column));
        }
        let mut counts = BTreeMap::<Vec<NodeId>, u64>::new();
        for row in &self.rows {
            let key = group_columns.iter().map(|column| row[*column]).collect();
            let count = counts.entry(key).or_default();
            *count = count.saturating_add(1);
        }
        Ok(counts
            .into_iter()
            .map(|(key, count)| GroupCount { key, count })
            .collect())
    }
}

fn candidate_facts<'a>(
    graph: &'a RelGraph,
    pattern: &RelationPattern,
    binding: &[Option<NodeId>],
) -> &'a [FactId] {
    let index = graph
        .relation_index(pattern.relation)
        .expect("validated relation index");
    let best = pattern
        .arguments
        .iter()
        .enumerate()
        .filter_map(|(position, term)| match term {
            Term::Node(node) => Some((position, *node)),
            Term::Variable(variable) => {
                binding[usize::from(*variable)].map(|node| (position, node))
            }
        })
        .map(|(position, node)| index.at(position, node))
        .min_by_key(|facts| facts.len());
    best.unwrap_or_else(|| index.all())
}

fn unify(
    pattern: &RelationPattern,
    arguments: &[NodeId],
    binding: &[Option<NodeId>],
) -> Option<Vec<Option<NodeId>>> {
    let mut output = binding.to_vec();
    for (term, actual) in pattern.arguments.iter().zip(arguments) {
        match term {
            Term::Node(expected) if expected != actual => return None,
            Term::Node(_) => {}
            Term::Variable(variable) => {
                let slot = &mut output[usize::from(*variable)];
                if slot.is_some_and(|expected| expected != *actual) {
                    return None;
                }
                *slot = Some(*actual);
            }
        }
    }
    Some(output)
}

fn resolve_term(term: Term, binding: &[Option<NodeId>]) -> Result<NodeId, RegError> {
    match term {
        Term::Node(node) => Ok(node),
        Term::Variable(variable) => {
            binding[usize::from(variable)].ok_or(RegError::QueryUnboundVariable(variable))
        }
    }
}

fn bounded_reachable(
    graph: &RelGraph,
    constraint: PathConstraint,
    from: NodeId,
    to: NodeId,
) -> Result<bool, RegError> {
    if from == to {
        return Ok(true);
    }
    let index = graph
        .relation_index(constraint.relation)
        .ok_or(RegError::UnknownRelation(constraint.relation))?;
    let mut visited = BTreeSet::from([from]);
    let mut frontier = BTreeSet::from([from]);
    for _ in 0..constraint.maximum_hops {
        let mut next = BTreeSet::new();
        for node in frontier {
            for fact_id in index.at(0, node) {
                let fact = graph.fact(*fact_id).ok_or(RegError::Capacity {
                    resource: "path fact index",
                    requested: u64::from(fact_id.0),
                    maximum: graph.fact_count() as u64,
                })?;
                let target = fact.arguments[1];
                if target == to {
                    return Ok(true);
                }
                if visited.insert(target) {
                    if visited.len() > constraint.maximum_visited as usize {
                        return Err(RegError::Capacity {
                            resource: "path visited nodes",
                            requested: visited.len() as u64,
                            maximum: u64::from(constraint.maximum_visited),
                        });
                    }
                    next.insert(target);
                }
            }
        }
        if next.is_empty() {
            break;
        }
        frontier = next;
    }
    Ok(false)
}
