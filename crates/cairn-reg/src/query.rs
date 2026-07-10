use crate::{FactId, NodeId, RegError, RelGraph, RelationId};

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

/// Deterministic conjunctive relation query.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QueryPlan {
    pub variable_count: u16,
    pub patterns: Vec<RelationPattern>,
    pub result_variables: Vec<u16>,
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
        let mut rows = bindings
            .into_iter()
            .map(|binding| {
                self.result_variables
                    .iter()
                    .map(|variable| {
                        binding[usize::from(*variable)]
                            .ok_or(RegError::QueryVariableOutOfRange(*variable))
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
                if let Term::Variable(variable) = term {
                    self.validate_variable(*variable)?;
                }
            }
        }
        Ok(())
    }

    fn validate_variable(&self, variable: u16) -> Result<(), RegError> {
        if variable >= self.variable_count {
            Err(RegError::QueryVariableOutOfRange(variable))
        } else {
            Ok(())
        }
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
