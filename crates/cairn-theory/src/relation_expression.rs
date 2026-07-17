//! Bounded composition of previously justified scalar relation operators.
//!
//! This reference path searches source-independent left-associated expression
//! structures separately from task-local anonymous attribute bindings. It is a
//! proposal learner; exact acceptance remains the verifier's responsibility.

use crate::{
    EntityObservationError, EntityObservationInventor, EntityObservationLimits,
    LearnedEntityObservationGrouping, LearnedObservationPredicateGrouping,
    ObservationPredicateLimits, PredicateGroupingLimits, RawEntityObservationEvidence,
    RawEntityObservationTrace, RawObservationPrimitive, RawRelationObservationTrace,
    RelationKeyOperator,
};
use cairn_core::{CanonicalEncode, CanonicalValue, Hash32, RelationRecord, RelationRole};
use cairn_invent::InventionLimits;
use cairn_rkl::{
    BinaryOp, Binder, BoundChecker, Definition, Expr, IntRange, Type, TypeChecker, VarName,
};
use std::collections::{BTreeMap, BTreeSet};

type ExpressionEpisodeBindings =
    BTreeMap<Hash32, BTreeMap<RelationRole, RelationExpressionBinding>>;
type MaterializedExpressionEpisodes = (
    Vec<RawEntityObservationTrace>,
    ExpressionEpisodeBindings,
    u64,
);

/// A source-independent scalar key program over logical parameter slots.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RelationKeyExpression {
    Parameter(u32),
    Binary {
        operator: RelationKeyOperator,
        left: Box<Self>,
        right: Box<Self>,
    },
}

impl RelationKeyExpression {
    #[must_use]
    pub fn parameter_count(&self) -> u32 {
        match self {
            Self::Parameter(index) => index.saturating_add(1),
            Self::Binary { left, right, .. } => left.parameter_count().max(right.parameter_count()),
        }
    }

    #[must_use]
    pub fn binary_node_count(&self) -> u32 {
        match self {
            Self::Parameter(_) => 0,
            Self::Binary { left, right, .. } => 1_u32
                .saturating_add(left.binary_node_count())
                .saturating_add(right.binary_node_count()),
        }
    }
}

/// Task-local binding from logical expression parameters to anonymous
/// attribute positions.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RelationExpressionBinding {
    pub attributes: Vec<usize>,
}

/// Explicit proposal language and resource bounds for expression composition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RelationExpressionLimits {
    pub permitted_binary_operators: BTreeSet<RelationKeyOperator>,
    pub parameter_arity: usize,
    pub maximum_binary_nodes: u32,
    pub maximum_candidates: u64,
    pub maximum_relation_evaluations: u64,
    pub maximum_absolute_attribute: i64,
    pub shape_policy: RelationExpressionShapePolicy,
}

/// Reachable expression-tree structures. The default preserves the historical
/// left-associated language; full trees require an opaque curriculum/verifier
/// extension.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum RelationExpressionShapePolicy {
    #[default]
    LeftAssociated,
    FullBinaryTrees,
}

impl Default for RelationExpressionLimits {
    fn default() -> Self {
        Self {
            permitted_binary_operators: BTreeSet::new(),
            parameter_arity: 0,
            maximum_binary_nodes: 0,
            maximum_candidates: 0,
            maximum_relation_evaluations: 0,
            maximum_absolute_attribute: 1_000_000_000,
            shape_policy: RelationExpressionShapePolicy::LeftAssociated,
        }
    }
}

/// Learned expression structure, current bindings, and downstream executable
/// entity/mechanism explanation.
#[derive(Clone, Debug, PartialEq)]
pub struct LearnedRelationExpressionGrouping {
    pub expression: RelationKeyExpression,
    pub relation_definition: Definition,
    pub selected_bindings: BTreeMap<Hash32, RelationExpressionBinding>,
    pub entity_grouping: LearnedEntityObservationGrouping,
    pub expression_candidates: u64,
    pub relation_evaluations: u64,
}

/// Source-independent proposal retained after task-local bindings are removed.
#[derive(Clone, Debug, PartialEq)]
pub struct RelationExpressionMechanism {
    pub expression: RelationKeyExpression,
    pub relation_definition: Definition,
    pub observation_mechanism: LearnedObservationPredicateGrouping,
}

/// One untrusted current-binding proposal emitted by a learned expression.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RelationExpressionProposal {
    pub binding: RelationExpressionBinding,
    pub output: BTreeSet<i64>,
}

/// One exact scalar input/output example. Inputs are anonymous frontend
/// channels; their task-local binding is learned rather than entering the
/// reusable expression identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScalarExpressionExample {
    pub inputs: Vec<i64>,
    pub output: i64,
}

/// One independently scoped scalar-expression experience.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScalarExpressionTrace {
    pub experience: Hash32,
    pub examples: Vec<ScalarExpressionExample>,
}

/// Learned source-independent scalar mechanism plus task-local bindings.
#[derive(Clone, Debug, PartialEq)]
pub struct LearnedScalarExpression {
    pub expression: RelationKeyExpression,
    pub definition: Definition,
    pub selected_bindings: BTreeMap<Hash32, RelationExpressionBinding>,
    pub expression_candidates: u64,
    pub candidate_evaluations: u64,
}

/// Reusable scalar expression with no source channel positions.
#[derive(Clone, Debug, PartialEq)]
pub struct ScalarExpressionMechanism {
    pub expression: RelationKeyExpression,
    pub definition: Definition,
}

impl LearnedScalarExpression {
    #[must_use]
    pub fn source_independent(&self) -> ScalarExpressionMechanism {
        ScalarExpressionMechanism {
            expression: self.expression.clone(),
            definition: self.definition.clone(),
        }
    }

    /// Canonical identity excluding current channel bindings.
    ///
    /// # Errors
    ///
    /// Returns canonical serialization failure.
    pub fn mechanism_hash(&self) -> Result<Hash32, cairn_core::CanonicalError> {
        self.source_independent().semantic_hash()
    }
}

impl ScalarExpressionMechanism {
    /// Canonical identity of the exact scalar AST and RKL definition.
    ///
    /// # Errors
    ///
    /// Returns canonical serialization failure.
    pub fn semantic_hash(&self) -> Result<Hash32, cairn_core::CanonicalError> {
        CanonicalValue::Array(vec![
            CanonicalValue::Text("cairn.scalar-expression.v1".to_owned()),
            expression_value(&self.expression),
            CanonicalValue::Bytes(self.definition.encode_canonical()?),
        ])
        .encode_canonical()
        .map(|bytes| Hash32::digest(&bytes))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScalarExpressionError {
    InsufficientExperiences,
    DuplicateExperience,
    InvalidEvidence,
    EmptyLanguage,
    Capacity,
    NoExactTheory,
    Ambiguous,
    InvalidBinding,
    Execution,
    Definition,
}

/// Exact task learner for scalar expression structure and anonymous channel
/// bindings. This path retains scalar values instead of collapsing them into
/// equality partitions.
#[derive(Clone, Copy, Debug, Default)]
pub struct ScalarExpressionInventor;

impl ScalarExpressionInventor {
    /// Learn one expression that uniquely explains every experience under a
    /// task-local binding per experience.
    ///
    /// # Errors
    ///
    /// Rejects malformed evidence, empty/capped language, binding ambiguity,
    /// or multiple source-independent exact expressions.
    pub fn invent(
        &self,
        traces: &[ScalarExpressionTrace],
        limits: &RelationExpressionLimits,
    ) -> Result<LearnedScalarExpression, ScalarExpressionError> {
        validate_scalar_traces(traces, limits)?;
        let expressions =
            enumerate_expressions(limits).map_err(|error| map_scalar_expression_error(&error))?;
        let mut exact = Vec::new();
        let mut candidate_evaluations = 0_u64;
        for expression in &expressions {
            let mut selected_bindings = BTreeMap::new();
            let mut expression_exact = true;
            for trace in traces {
                let channel_count = trace.examples[0].inputs.len();
                let bindings = ordered_bindings(
                    channel_count,
                    limits.parameter_arity,
                    limits.maximum_candidates,
                )
                .map_err(|error| map_scalar_expression_error(&error))?;
                let mut matching = Vec::new();
                for binding in bindings
                    .into_iter()
                    .filter(|binding| canonical_initial_binding(expression, binding))
                {
                    let mut binding_exact = true;
                    for example in &trace.examples {
                        candidate_evaluations = candidate_evaluations
                            .checked_add(1)
                            .ok_or(ScalarExpressionError::Capacity)?;
                        if candidate_evaluations > limits.maximum_relation_evaluations {
                            return Err(ScalarExpressionError::Capacity);
                        }
                        if evaluate_expression(expression, &example.inputs, &binding)
                            .map_err(|error| map_scalar_expression_error(&error))?
                            != example.output
                        {
                            binding_exact = false;
                            break;
                        }
                    }
                    if binding_exact {
                        matching.push(binding);
                    }
                }
                if matching.len() != 1 {
                    expression_exact = false;
                    break;
                }
                selected_bindings.insert(trace.experience, matching.remove(0));
            }
            if expression_exact {
                exact.push((expression.clone(), selected_bindings));
            }
        }
        if exact.is_empty() {
            return Err(ScalarExpressionError::NoExactTheory);
        }
        if exact.len() != 1 {
            return Err(ScalarExpressionError::Ambiguous);
        }
        let (expression, selected_bindings) =
            exact.pop().ok_or(ScalarExpressionError::NoExactTheory)?;
        let definition = compile_scalar_expression(&expression, limits)?;
        Ok(LearnedScalarExpression {
            expression,
            definition,
            selected_bindings,
            expression_candidates: u64::try_from(expressions.len())
                .map_err(|_| ScalarExpressionError::Capacity)?,
            candidate_evaluations,
        })
    }

    /// Rebind a learned scalar mechanism to all compatible current channels.
    ///
    /// # Errors
    ///
    /// Rejects malformed inputs, incompatible arity, capacity, or execution
    /// overflow. Returned values are proposals only.
    pub fn propose(
        &self,
        mechanism: &ScalarExpressionMechanism,
        inputs: &[i64],
        limits: &RelationExpressionLimits,
    ) -> Result<Vec<(RelationExpressionBinding, i64)>, ScalarExpressionError> {
        if inputs.len() < limits.parameter_arity
            || mechanism.expression.parameter_count()
                != u32::try_from(limits.parameter_arity)
                    .map_err(|_| ScalarExpressionError::Capacity)?
            || mechanism.expression.binary_node_count() > limits.maximum_binary_nodes
        {
            return Err(ScalarExpressionError::InvalidBinding);
        }
        ordered_bindings(
            inputs.len(),
            limits.parameter_arity,
            limits.maximum_candidates,
        )
        .map_err(|error| map_scalar_expression_error(&error))?
        .into_iter()
        .filter(|binding| canonical_initial_binding(&mechanism.expression, binding))
        .map(|binding| {
            evaluate_expression(&mechanism.expression, inputs, &binding)
                .map(|output| (binding, output))
                .map_err(|error| map_scalar_expression_error(&error))
        })
        .collect()
    }
}

impl RelationExpressionMechanism {
    /// Canonical semantic identity of expression plus downstream mechanism.
    ///
    /// # Errors
    ///
    /// Returns canonical serialization failure.
    pub fn semantic_hash(&self) -> Result<Hash32, cairn_core::CanonicalError> {
        expression_mechanism_hash(
            &self.expression,
            &self.relation_definition,
            &self.observation_mechanism,
        )
    }
}

impl LearnedRelationExpressionGrouping {
    /// Remove task-local attribute positions and retain executable structure.
    #[must_use]
    pub fn source_independent(&self) -> RelationExpressionMechanism {
        RelationExpressionMechanism {
            expression: self.expression.clone(),
            relation_definition: self.relation_definition.clone(),
            observation_mechanism: self.entity_grouping.mechanism.clone(),
        }
    }

    /// Canonical semantic identity excluding task-local bindings.
    ///
    /// # Errors
    ///
    /// Returns canonical serialization failure.
    pub fn mechanism_hash(&self) -> Result<Hash32, cairn_core::CanonicalError> {
        self.source_independent().semantic_hash()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RelationExpressionError {
    InsufficientEpisodes,
    DuplicateEpisode,
    InvalidEvidence,
    EmptyLanguage,
    Capacity,
    NoExactTheory,
    Ambiguous,
    InvalidBinding,
    RelationDefinition,
    RelationExecution,
    Canonicalization,
    Entity(EntityObservationError),
}

/// Exact reference learner for bounded compositions of permitted operators.
#[derive(Clone, Copy, Debug, Default)]
pub struct RelationExpressionInventor;

impl RelationExpressionInventor {
    /// Learn one source-independent expression structure jointly with current
    /// attribute bindings and the downstream entity explanation.
    ///
    /// # Errors
    ///
    /// Fails closed on malformed evidence, empty/capped language, ambiguity,
    /// or absence of an exact executable theory.
    #[allow(clippy::too_many_arguments)]
    pub fn invent(
        &self,
        traces: &[RawRelationObservationTrace],
        expression_limits: &RelationExpressionLimits,
        entity_limits: EntityObservationLimits,
        observation_limits: ObservationPredicateLimits,
        predicate_limits: PredicateGroupingLimits,
        invention_limits: InventionLimits,
    ) -> Result<LearnedRelationExpressionGrouping, RelationExpressionError> {
        validate_traces(traces, expression_limits)?;
        let expressions = enumerate_expressions(expression_limits)?;
        let mut exact = Vec::new();
        let mut total_evaluations = 0_u64;
        for expression in &expressions {
            let (materialized, bindings, evaluations) =
                materialize_all(traces, expression, expression_limits)?;
            total_evaluations = total_evaluations
                .checked_add(evaluations)
                .ok_or(RelationExpressionError::Capacity)?;
            if total_evaluations > expression_limits.maximum_relation_evaluations {
                return Err(RelationExpressionError::Capacity);
            }
            match EntityObservationInventor.invent(
                &materialized,
                entity_limits,
                observation_limits,
                predicate_limits,
                invention_limits,
            ) {
                Ok(entity_grouping) => {
                    let selected_bindings = selected_bindings(&entity_grouping, &bindings)?;
                    exact.push((expression.clone(), selected_bindings, entity_grouping));
                }
                Err(
                    EntityObservationError::NoExactTheory | EntityObservationError::NoRelations,
                ) => {}
                Err(error) => return Err(RelationExpressionError::Entity(error)),
            }
        }
        if exact.is_empty() {
            return Err(RelationExpressionError::NoExactTheory);
        }
        if exact.len() != 1 {
            return Err(RelationExpressionError::Ambiguous);
        }
        let (expression, selected_bindings, entity_grouping) =
            exact.pop().ok_or(RelationExpressionError::NoExactTheory)?;
        let relation_definition = compile_expression(&expression, expression_limits)?;
        Ok(LearnedRelationExpressionGrouping {
            expression,
            relation_definition,
            selected_bindings,
            entity_grouping,
            expression_candidates: u64::try_from(expressions.len())
                .map_err(|_| RelationExpressionError::Capacity)?,
            relation_evaluations: total_evaluations,
        })
    }

    /// Rebind a source-independent expression to all compatible current
    /// attribute positions and emit untrusted outputs without expected truth.
    ///
    /// # Errors
    ///
    /// Rejects malformed evidence, incompatible expression language, resource
    /// caps, invalid bindings, or downstream proposal execution.
    #[allow(clippy::too_many_arguments)]
    pub fn propose(
        &self,
        mechanism: &RelationExpressionMechanism,
        trace: &RawRelationObservationTrace,
        expression_limits: &RelationExpressionLimits,
        entity_limits: EntityObservationLimits,
        observation_limits: ObservationPredicateLimits,
        predicate_limits: PredicateGroupingLimits,
    ) -> Result<Vec<RelationExpressionProposal>, RelationExpressionError> {
        validate_trace(trace, expression_limits)?;
        if mechanism.expression.parameter_count()
            != u32::try_from(expression_limits.parameter_arity)
                .map_err(|_| RelationExpressionError::Capacity)?
            || mechanism.expression.binary_node_count() > expression_limits.maximum_binary_nodes
        {
            return Err(RelationExpressionError::InvalidBinding);
        }
        let (materialized, bindings, _) =
            materialize_trace(trace, &mechanism.expression, expression_limits)?;
        let mut proposals = Vec::new();
        for binding in bindings.into_values() {
            match EntityObservationInventor.predict(
                &mechanism.observation_mechanism,
                &materialized,
                &role_for_binding(&binding),
                entity_limits,
                observation_limits,
                predicate_limits,
            ) {
                Ok(output) => proposals.push(RelationExpressionProposal { binding, output }),
                Err(
                    EntityObservationError::NoRelations
                    | EntityObservationError::InvalidInterpretation
                    | EntityObservationError::Observation(_),
                ) => {}
                Err(error) => return Err(RelationExpressionError::Entity(error)),
            }
        }
        Ok(proposals)
    }
}

fn validate_traces(
    traces: &[RawRelationObservationTrace],
    limits: &RelationExpressionLimits,
) -> Result<(), RelationExpressionError> {
    if traces.len() < 2 {
        return Err(RelationExpressionError::InsufficientEpisodes);
    }
    if traces
        .iter()
        .map(|trace| trace.episode)
        .collect::<BTreeSet<_>>()
        .len()
        != traces.len()
    {
        return Err(RelationExpressionError::DuplicateEpisode);
    }
    if limits.permitted_binary_operators.is_empty()
        || limits.parameter_arity < 3
        || limits.maximum_binary_nodes
            < u32::try_from(limits.parameter_arity - 1).unwrap_or(u32::MAX)
        || limits.maximum_candidates == 0
        || limits.maximum_relation_evaluations == 0
        || limits.maximum_absolute_attribute <= 0
        || limits.permitted_binary_operators.iter().any(|operator| {
            !matches!(
                operator,
                RelationKeyOperator::Add
                    | RelationKeyOperator::Subtract
                    | RelationKeyOperator::Minimum
                    | RelationKeyOperator::Maximum
            )
        })
    {
        return Err(RelationExpressionError::EmptyLanguage);
    }
    for trace in traces {
        validate_trace(trace, limits)?;
    }
    Ok(())
}

fn validate_scalar_traces(
    traces: &[ScalarExpressionTrace],
    limits: &RelationExpressionLimits,
) -> Result<(), ScalarExpressionError> {
    if traces.len() < 2 {
        return Err(ScalarExpressionError::InsufficientExperiences);
    }
    if traces
        .iter()
        .map(|trace| trace.experience)
        .collect::<BTreeSet<_>>()
        .len()
        != traces.len()
    {
        return Err(ScalarExpressionError::DuplicateExperience);
    }
    if limits.permitted_binary_operators.is_empty()
        || limits.parameter_arity < 3
        || limits.maximum_binary_nodes
            < u32::try_from(limits.parameter_arity - 1).unwrap_or(u32::MAX)
        || limits.maximum_candidates == 0
        || limits.maximum_relation_evaluations == 0
        || limits.maximum_absolute_attribute <= 0
    {
        return Err(ScalarExpressionError::EmptyLanguage);
    }
    for trace in traces {
        let Some(channel_count) = trace.examples.first().map(|example| example.inputs.len()) else {
            return Err(ScalarExpressionError::InvalidEvidence);
        };
        if trace.examples.len() < 3
            || channel_count < limits.parameter_arity
            || trace.examples.iter().any(|example| {
                example.inputs.len() != channel_count
                    || example.inputs.iter().chain([&example.output]).any(|value| {
                        value
                            .checked_abs()
                            .is_none_or(|magnitude| magnitude > limits.maximum_absolute_attribute)
                    })
            })
        {
            return Err(ScalarExpressionError::InvalidEvidence);
        }
    }
    Ok(())
}

const fn map_scalar_expression_error(error: &RelationExpressionError) -> ScalarExpressionError {
    match error {
        RelationExpressionError::Capacity => ScalarExpressionError::Capacity,
        RelationExpressionError::EmptyLanguage => ScalarExpressionError::EmptyLanguage,
        RelationExpressionError::InvalidBinding => ScalarExpressionError::InvalidBinding,
        RelationExpressionError::RelationExecution => ScalarExpressionError::Execution,
        RelationExpressionError::RelationDefinition => ScalarExpressionError::Definition,
        _ => ScalarExpressionError::InvalidEvidence,
    }
}

fn validate_trace(
    trace: &RawRelationObservationTrace,
    limits: &RelationExpressionLimits,
) -> Result<(), RelationExpressionError> {
    let Some(attribute_count) = trace
        .primitives
        .values()
        .next()
        .map(|primitive| primitive.attributes.len())
    else {
        return Err(RelationExpressionError::InvalidEvidence);
    };
    if attribute_count < limits.parameter_arity
        || trace.primitives.values().any(|primitive| {
            primitive.attributes.len() != attribute_count
                || primitive.attributes.iter().any(|value| {
                    value
                        .checked_abs()
                        .is_none_or(|magnitude| magnitude > limits.maximum_absolute_attribute)
                })
        })
    {
        return Err(RelationExpressionError::InvalidEvidence);
    }
    Ok(())
}

fn enumerate_expressions(
    limits: &RelationExpressionLimits,
) -> Result<Vec<RelationKeyExpression>, RelationExpressionError> {
    if limits.shape_policy == RelationExpressionShapePolicy::FullBinaryTrees {
        return enumerate_expression_range(0, limits.parameter_arity, limits);
    }
    let mut expressions = vec![RelationKeyExpression::Parameter(0)];
    for parameter in 1..limits.parameter_arity {
        let parameter = u32::try_from(parameter).map_err(|_| RelationExpressionError::Capacity)?;
        let mut next = Vec::new();
        for expression in expressions {
            for operator in &limits.permitted_binary_operators {
                next.push(RelationKeyExpression::Binary {
                    operator: *operator,
                    left: Box::new(expression.clone()),
                    right: Box::new(RelationKeyExpression::Parameter(parameter)),
                });
                if u64::try_from(next.len()).unwrap_or(u64::MAX) > limits.maximum_candidates {
                    return Err(RelationExpressionError::Capacity);
                }
            }
        }
        expressions = next;
    }
    Ok(expressions)
}

fn enumerate_expression_range(
    start: usize,
    end: usize,
    limits: &RelationExpressionLimits,
) -> Result<Vec<RelationKeyExpression>, RelationExpressionError> {
    if end == start.saturating_add(1) {
        return Ok(vec![RelationKeyExpression::Parameter(
            u32::try_from(start).map_err(|_| RelationExpressionError::Capacity)?,
        )]);
    }
    let mut expressions = Vec::new();
    for split in start.saturating_add(1)..end {
        let left = enumerate_expression_range(start, split, limits)?;
        let right = enumerate_expression_range(split, end, limits)?;
        for left_expression in &left {
            for right_expression in &right {
                for operator in &limits.permitted_binary_operators {
                    expressions.push(RelationKeyExpression::Binary {
                        operator: *operator,
                        left: Box::new(left_expression.clone()),
                        right: Box::new(right_expression.clone()),
                    });
                    if u64::try_from(expressions.len()).unwrap_or(u64::MAX)
                        > limits.maximum_candidates
                    {
                        return Err(RelationExpressionError::Capacity);
                    }
                }
            }
        }
    }
    Ok(expressions)
}

fn materialize_all(
    traces: &[RawRelationObservationTrace],
    expression: &RelationKeyExpression,
    limits: &RelationExpressionLimits,
) -> Result<MaterializedExpressionEpisodes, RelationExpressionError> {
    let mut output = Vec::new();
    let mut bindings = BTreeMap::new();
    let mut evaluations = 0_u64;
    for trace in traces {
        let (lowered, trace_bindings, trace_evaluations) =
            materialize_trace(trace, expression, limits)?;
        evaluations = evaluations
            .checked_add(trace_evaluations)
            .ok_or(RelationExpressionError::Capacity)?;
        output.push(lowered);
        bindings.insert(trace.episode, trace_bindings);
    }
    Ok((output, bindings, evaluations))
}

fn materialize_trace(
    trace: &RawRelationObservationTrace,
    expression: &RelationKeyExpression,
    limits: &RelationExpressionLimits,
) -> Result<
    (
        RawEntityObservationTrace,
        BTreeMap<RelationRole, RelationExpressionBinding>,
        u64,
    ),
    RelationExpressionError,
> {
    let attribute_count = trace
        .primitives
        .values()
        .next()
        .ok_or(RelationExpressionError::InvalidEvidence)?
        .attributes
        .len();
    let bindings = ordered_bindings(
        attribute_count,
        limits.parameter_arity,
        limits.maximum_candidates,
    )?
    .into_iter()
    .filter(|binding| canonical_initial_binding(expression, binding))
    .collect::<Vec<_>>();
    let items = trace.primitives.iter().collect::<Vec<_>>();
    let mut relations = Vec::new();
    let mut role_bindings = BTreeMap::new();
    let mut evaluations = 0_u64;
    for binding in bindings {
        let role = role_for_binding(&binding);
        role_bindings.insert(role.clone(), binding.clone());
        for left_index in 0..items.len() {
            for right_index in (left_index + 1)..items.len() {
                evaluations = evaluations
                    .checked_add(1)
                    .ok_or(RelationExpressionError::Capacity)?;
                if evaluations > limits.maximum_relation_evaluations {
                    return Err(RelationExpressionError::Capacity);
                }
                let (left_id, left) = items[left_index];
                let (right_id, right) = items[right_index];
                if evaluate_expression(expression, &left.attributes, &binding)?
                    == evaluate_expression(expression, &right.attributes, &binding)?
                {
                    relations.push(RelationRecord {
                        role: role.clone(),
                        subject: left_id.clone(),
                        object: right_id.clone(),
                    });
                }
            }
        }
    }
    let primitives = trace
        .primitives
        .iter()
        .map(|(id, primitive)| {
            (
                id.clone(),
                RawObservationPrimitive {
                    observation: primitive.observation,
                },
            )
        })
        .collect();
    Ok((
        RawEntityObservationTrace {
            episode: trace.episode,
            evidence: RawEntityObservationEvidence {
                primitives,
                relations,
            },
            outputs: trace.outputs.clone(),
        },
        role_bindings,
        evaluations,
    ))
}

fn canonical_initial_binding(
    expression: &RelationKeyExpression,
    binding: &RelationExpressionBinding,
) -> bool {
    match expression {
        RelationKeyExpression::Parameter(_) => true,
        RelationKeyExpression::Binary {
            operator,
            left,
            right,
        } => {
            let local = match (&**left, &**right) {
                (
                    RelationKeyExpression::Parameter(left_parameter),
                    RelationKeyExpression::Parameter(right_parameter),
                ) if matches!(
                    operator,
                    RelationKeyOperator::Add
                        | RelationKeyOperator::Minimum
                        | RelationKeyOperator::Maximum
                ) =>
                {
                    usize::try_from(*left_parameter)
                        .ok()
                        .and_then(|index| binding.attributes.get(index))
                        .zip(
                            usize::try_from(*right_parameter)
                                .ok()
                                .and_then(|index| binding.attributes.get(index)),
                        )
                        .is_some_and(|(left_attribute, right_attribute)| {
                            left_attribute < right_attribute
                        })
                }
                _ => true,
            };
            local
                && canonical_initial_binding(left, binding)
                && canonical_initial_binding(right, binding)
        }
    }
}

fn ordered_bindings(
    attribute_count: usize,
    arity: usize,
    maximum_candidates: u64,
) -> Result<Vec<RelationExpressionBinding>, RelationExpressionError> {
    let mut output = Vec::new();
    enumerate_binding_prefixes(
        attribute_count,
        arity,
        &mut Vec::new(),
        &mut output,
        maximum_candidates,
    )?;
    Ok(output)
}

fn enumerate_binding_prefixes(
    attribute_count: usize,
    arity: usize,
    current: &mut Vec<usize>,
    output: &mut Vec<RelationExpressionBinding>,
    maximum_candidates: u64,
) -> Result<(), RelationExpressionError> {
    if current.len() == arity {
        output.push(RelationExpressionBinding {
            attributes: current.clone(),
        });
        if u64::try_from(output.len()).unwrap_or(u64::MAX) > maximum_candidates {
            return Err(RelationExpressionError::Capacity);
        }
        return Ok(());
    }
    for attribute in 0..attribute_count {
        if !current.contains(&attribute) {
            current.push(attribute);
            enumerate_binding_prefixes(
                attribute_count,
                arity,
                current,
                output,
                maximum_candidates,
            )?;
            current.pop();
        }
    }
    Ok(())
}

fn selected_bindings(
    learned: &LearnedEntityObservationGrouping,
    candidates: &BTreeMap<Hash32, BTreeMap<RelationRole, RelationExpressionBinding>>,
) -> Result<BTreeMap<Hash32, RelationExpressionBinding>, RelationExpressionError> {
    learned
        .selected_relations
        .iter()
        .map(|(episode, role)| {
            candidates
                .get(episode)
                .and_then(|bindings| bindings.get(role))
                .cloned()
                .map(|binding| (*episode, binding))
                .ok_or(RelationExpressionError::InvalidBinding)
        })
        .collect()
}

fn evaluate_expression(
    expression: &RelationKeyExpression,
    attributes: &[i64],
    binding: &RelationExpressionBinding,
) -> Result<i64, RelationExpressionError> {
    match expression {
        RelationKeyExpression::Parameter(parameter) => binding
            .attributes
            .get(usize::try_from(*parameter).map_err(|_| RelationExpressionError::Capacity)?)
            .and_then(|attribute| attributes.get(*attribute))
            .copied()
            .ok_or(RelationExpressionError::InvalidBinding),
        RelationKeyExpression::Binary {
            operator,
            left,
            right,
        } => {
            let left = evaluate_expression(left, attributes, binding)?;
            let right = evaluate_expression(right, attributes, binding)?;
            match operator {
                RelationKeyOperator::Add => left
                    .checked_add(right)
                    .ok_or(RelationExpressionError::RelationExecution),
                RelationKeyOperator::Subtract => left
                    .checked_sub(right)
                    .ok_or(RelationExpressionError::RelationExecution),
                RelationKeyOperator::Minimum => Ok(left.min(right)),
                RelationKeyOperator::Maximum => Ok(left.max(right)),
                _ => Err(RelationExpressionError::RelationDefinition),
            }
        }
    }
}

fn compile_expression(
    expression: &RelationKeyExpression,
    limits: &RelationExpressionLimits,
) -> Result<Definition, RelationExpressionError> {
    let arity = usize::try_from(expression.parameter_count())
        .map_err(|_| RelationExpressionError::Capacity)?;
    let left = (0..arity)
        .map(|index| VarName(format!("left_attribute_{index}")))
        .collect::<Vec<_>>();
    let right = (0..arity)
        .map(|index| VarName(format!("right_attribute_{index}")))
        .collect::<Vec<_>>();
    let parameter_type = Type::Int(IntRange {
        lower: -limits.maximum_absolute_attribute,
        upper: limits.maximum_absolute_attribute,
    });
    let parameters = left
        .iter()
        .chain(&right)
        .cloned()
        .map(|name| Binder {
            name,
            ty: parameter_type.clone(),
        })
        .collect();
    let definition = Definition {
        name: "invented_relation_expression".to_owned(),
        parameters,
        return_type: Type::Bool,
        body: Expr::Binary {
            op: BinaryOp::Equal,
            left: Box::new(lower_expression(expression, &left)?),
            right: Box::new(lower_expression(expression, &right)?),
        },
        recursion: None,
        proof_annotations: Vec::new(),
    };
    TypeChecker::new(256)
        .check_definition(&definition)
        .map_err(|_| RelationExpressionError::RelationDefinition)?;
    Ok(definition)
}

fn compile_scalar_expression(
    expression: &RelationKeyExpression,
    limits: &RelationExpressionLimits,
) -> Result<Definition, ScalarExpressionError> {
    let arity = usize::try_from(expression.parameter_count())
        .map_err(|_| ScalarExpressionError::Capacity)?;
    let variables = (0..arity)
        .map(|index| VarName(format!("attribute_{index}")))
        .collect::<Vec<_>>();
    let input_range = IntRange {
        lower: -limits.maximum_absolute_attribute,
        upper: limits.maximum_absolute_attribute,
    };
    let output_bound = limits
        .maximum_absolute_attribute
        .checked_mul(i64::from(expression.binary_node_count()).saturating_add(1))
        .ok_or(ScalarExpressionError::Capacity)?;
    let definition = Definition {
        name: "invented_scalar_expression".to_owned(),
        parameters: variables
            .iter()
            .cloned()
            .map(|name| Binder {
                name,
                ty: Type::Int(input_range),
            })
            .collect(),
        return_type: Type::Int(IntRange {
            lower: -output_bound,
            upper: output_bound,
        }),
        body: lower_expression(expression, &variables)
            .map_err(|error| map_scalar_expression_error(&error))?,
        recursion: None,
        proof_annotations: Vec::new(),
    };
    TypeChecker::new(256)
        .check_definition(&definition)
        .map_err(|_| ScalarExpressionError::Definition)?;
    Ok(definition)
}

fn lower_expression(
    expression: &RelationKeyExpression,
    variables: &[VarName],
) -> Result<Expr, RelationExpressionError> {
    match expression {
        RelationKeyExpression::Parameter(parameter) => variables
            .get(usize::try_from(*parameter).map_err(|_| RelationExpressionError::Capacity)?)
            .cloned()
            .map(Expr::Var)
            .ok_or(RelationExpressionError::InvalidBinding),
        RelationKeyExpression::Binary {
            operator,
            left,
            right,
        } => Ok(Expr::Binary {
            op: match operator {
                RelationKeyOperator::Add => BinaryOp::Add,
                RelationKeyOperator::Subtract => BinaryOp::Subtract,
                RelationKeyOperator::Minimum => BinaryOp::Minimum,
                RelationKeyOperator::Maximum => BinaryOp::Maximum,
                _ => return Err(RelationExpressionError::RelationDefinition),
            },
            left: Box::new(lower_expression(left, variables)?),
            right: Box::new(lower_expression(right, variables)?),
        }),
    }
}

fn role_for_binding(binding: &RelationExpressionBinding) -> RelationRole {
    RelationRole(format!(
        "cairn.task-local.expression.{}",
        binding
            .attributes
            .iter()
            .map(usize::to_string)
            .collect::<Vec<_>>()
            .join(".")
    ))
}

fn expression_mechanism_hash(
    expression: &RelationKeyExpression,
    definition: &Definition,
    mechanism: &LearnedObservationPredicateGrouping,
) -> Result<Hash32, cairn_core::CanonicalError> {
    CanonicalValue::Array(vec![
        CanonicalValue::Text("cairn.relation-expression.v1".to_owned()),
        expression_value(expression),
        CanonicalValue::Bytes(definition.encode_canonical()?),
        CanonicalValue::Bytes(mechanism.semantic_hash()?.as_bytes().to_vec()),
    ])
    .encode_canonical()
    .map(|bytes| Hash32::digest(&bytes))
}

fn expression_value(expression: &RelationKeyExpression) -> CanonicalValue {
    match expression {
        RelationKeyExpression::Parameter(parameter) => CanonicalValue::Array(vec![
            CanonicalValue::Unsigned(0),
            CanonicalValue::Unsigned(u64::from(*parameter)),
        ]),
        RelationKeyExpression::Binary {
            operator,
            left,
            right,
        } => CanonicalValue::Array(vec![
            CanonicalValue::Unsigned(1),
            CanonicalValue::Unsigned(operator_code(*operator)),
            expression_value(left),
            expression_value(right),
        ]),
    }
}

const fn operator_code(operator: RelationKeyOperator) -> u64 {
    match operator {
        RelationKeyOperator::Projection => 0,
        RelationKeyOperator::Add => 1,
        RelationKeyOperator::Subtract => 2,
        RelationKeyOperator::Minimum => 3,
        RelationKeyOperator::Maximum => 4,
        RelationKeyOperator::Sum => 5,
        RelationKeyOperator::MinimumFold => 6,
        RelationKeyOperator::MaximumFold => 7,
    }
}
