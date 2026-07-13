//! Independent certification and replay for learned relation expressions.

use crate::{
    EntityObservationVerifier, EntityObservationVerifyError,
    VerifiedRawRelationObservationPromotion,
};
use cairn_core::{EntityId, Hash32, RelationRecord, RelationRole};
use cairn_curriculum::RelationLanguageGrowthRequest;
use cairn_rkl::{BinaryOp, Binder, Definition, ExecLimits, Expr, IntRange, Type, VarName};
use cairn_theory::{
    EntityObservationLimits, LearnedRelationExpressionGrouping, ObservationPredicateLimits,
    PredicateGroupingLimits, RawEntityObservationEvidence, RawEntityObservationTrace,
    RawObservationPrimitive, RawRelationMechanism, RawRelationObservationTrace,
    RelationExpressionBinding, RelationExpressionLimits, RelationKeyExpression,
    RelationKeyOperator,
};
use std::collections::{BTreeMap, BTreeSet};

/// Hard policy bounds for proof-gated expression-language composition.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RelationExpressionLanguageLimits {
    pub maximum_parameter_arity: u32,
    pub maximum_binary_nodes: u32,
    pub maximum_source_operators: u32,
}

impl Default for RelationExpressionLanguageLimits {
    fn default() -> Self {
        Self {
            maximum_parameter_arity: 8,
            maximum_binary_nodes: 7,
            maximum_source_operators: 4,
        }
    }
}

/// Opaque authority to compose a bounded expression proposal language from
/// independently promoted binary operators. It cannot accept an answer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedRelationExpressionLanguage {
    source_mechanisms: BTreeSet<Hash32>,
    failure_core: Hash32,
    operators: BTreeSet<RelationKeyOperator>,
    parameter_arity: u32,
    maximum_binary_nodes: u32,
}

impl VerifiedRelationExpressionLanguage {
    #[must_use]
    pub const fn failure_core(&self) -> Hash32 {
        self.failure_core
    }

    #[must_use]
    pub const fn parameter_arity(&self) -> u32 {
        self.parameter_arity
    }

    #[must_use]
    pub const fn maximum_binary_nodes(&self) -> u32 {
        self.maximum_binary_nodes
    }

    #[must_use]
    pub const fn operators(&self) -> &BTreeSet<RelationKeyOperator> {
        &self.operators
    }

    #[must_use]
    pub const fn source_mechanisms(&self) -> &BTreeSet<Hash32> {
        &self.source_mechanisms
    }

    /// Apply only the certified structure to caller-owned resource budgets.
    ///
    /// # Errors
    ///
    /// Rejects platform-size overflow.
    pub fn apply(
        &self,
        mut limits: RelationExpressionLimits,
    ) -> Result<RelationExpressionLimits, RelationExpressionLanguageError> {
        limits.parameter_arity = usize::try_from(self.parameter_arity)
            .map_err(|_| RelationExpressionLanguageError::Capacity)?;
        limits.maximum_binary_nodes = self.maximum_binary_nodes;
        limits
            .permitted_binary_operators
            .clone_from(&self.operators);
        Ok(limits)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RelationExpressionLanguageError {
    InvalidPromotion,
    InsufficientOperators,
    UnsupportedOperator,
    InvalidRequestedArity,
    Capacity,
}

/// Separate authority converting multiple promoted operators and a redacted
/// structural failure into a bounded expression-composition language.
#[derive(Clone, Copy, Debug, Default)]
pub struct RelationExpressionLanguageConsolidator;

impl RelationExpressionLanguageConsolidator {
    /// Certify a composition language without supplying a final expression.
    ///
    /// # Errors
    ///
    /// Rejects invalid promotions, fewer than two distinct binary operators,
    /// unsupported operators, or arity/node/source caps.
    pub fn consolidate(
        &self,
        sources: &[(
            &RawRelationMechanism,
            &VerifiedRawRelationObservationPromotion,
        )],
        request: &RelationLanguageGrowthRequest,
        limits: RelationExpressionLanguageLimits,
    ) -> Result<VerifiedRelationExpressionLanguage, RelationExpressionLanguageError> {
        if sources.len() < 2
            || u32::try_from(sources.len()).unwrap_or(u32::MAX) > limits.maximum_source_operators
        {
            return Err(RelationExpressionLanguageError::InsufficientOperators);
        }
        let mut source_mechanisms = BTreeSet::new();
        let mut operators = BTreeSet::new();
        for (mechanism, promotion) in sources {
            let mechanism_id = mechanism
                .semantic_hash()
                .map_err(|_| RelationExpressionLanguageError::InvalidPromotion)?;
            if promotion.mechanism() != mechanism_id
                || promotion.verified_reuses() < 2
                || promotion.support().operator != mechanism.operator
                || promotion.support().key_arity != 2
                || mechanism.key_arity != 2
            {
                return Err(RelationExpressionLanguageError::InvalidPromotion);
            }
            if !matches!(
                mechanism.operator,
                RelationKeyOperator::Add
                    | RelationKeyOperator::Subtract
                    | RelationKeyOperator::Minimum
                    | RelationKeyOperator::Maximum
            ) {
                return Err(RelationExpressionLanguageError::UnsupportedOperator);
            }
            source_mechanisms.insert(mechanism_id);
            operators.insert(mechanism.operator);
        }
        if operators.len() < 2 || source_mechanisms.len() != sources.len() {
            return Err(RelationExpressionLanguageError::InsufficientOperators);
        }
        let parameter_arity = request.minimum_dependency_arity_lower_bound();
        let maximum_binary_nodes = parameter_arity.saturating_sub(1);
        if parameter_arity < 3
            || parameter_arity > limits.maximum_parameter_arity
            || maximum_binary_nodes > limits.maximum_binary_nodes
        {
            return Err(RelationExpressionLanguageError::InvalidRequestedArity);
        }
        Ok(VerifiedRelationExpressionLanguage {
            source_mechanisms,
            failure_core: request.failure_core(),
            operators,
            parameter_arity,
            maximum_binary_nodes,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedRelationExpressionGrouping {
    mechanism: Hash32,
    experiences: BTreeSet<Hash32>,
    replayed_groups: u64,
}

/// Opaque evidence that one learned expression replayed a disjoint cohort and
/// removed source-independent expression-structure search.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedRelationExpressionReuse {
    mechanism: Hash32,
    experiences: BTreeSet<Hash32>,
    replayed_groups: u64,
    expressions_without: u64,
    expressions_with: u64,
}

impl VerifiedRelationExpressionReuse {
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
    pub const fn expressions_without(&self) -> u64 {
        self.expressions_without
    }

    #[must_use]
    pub const fn expressions_with(&self) -> u64 {
        self.expressions_with
    }
}

/// Opaque two-cohort certificate authorizing proposal-only persistence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedRelationExpressionPromotion {
    mechanism: Hash32,
    experiences: BTreeSet<Hash32>,
    verified_reuses: u32,
    expressions_saved: u64,
}

impl VerifiedRelationExpressionPromotion {
    #[must_use]
    pub const fn mechanism(&self) -> Hash32 {
        self.mechanism
    }

    #[must_use]
    pub const fn experiences(&self) -> &BTreeSet<Hash32> {
        &self.experiences
    }

    #[must_use]
    pub const fn verified_reuses(&self) -> u32 {
        self.verified_reuses
    }

    #[must_use]
    pub const fn expressions_saved(&self) -> u64 {
        self.expressions_saved
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RelationExpressionPromotionError {
    InsufficientEvidence,
    MixedMechanism,
    OverlappingExperience,
    NoLearningReduction,
    Capacity,
}

/// Separate authority requiring two disjoint exact-reuse receipts before an
/// expression may be treated as persistent proposal knowledge.
#[derive(Clone, Copy, Debug, Default)]
pub struct RelationExpressionPromotionConsolidator;

impl RelationExpressionPromotionConsolidator {
    /// Consolidate independent receipts for one semantic expression.
    ///
    /// # Errors
    ///
    /// Rejects fewer than two receipts, mixed mechanisms, overlapping
    /// experience, missing reduction, or accounting overflow.
    pub fn consolidate(
        &self,
        receipts: &[VerifiedRelationExpressionReuse],
    ) -> Result<VerifiedRelationExpressionPromotion, RelationExpressionPromotionError> {
        if receipts.len() < 2 {
            return Err(RelationExpressionPromotionError::InsufficientEvidence);
        }
        let mechanism = receipts[0].mechanism;
        let mut experiences = BTreeSet::new();
        let mut expressions_saved = 0_u64;
        for receipt in receipts {
            if receipt.mechanism != mechanism {
                return Err(RelationExpressionPromotionError::MixedMechanism);
            }
            if receipt.expressions_without <= receipt.expressions_with {
                return Err(RelationExpressionPromotionError::NoLearningReduction);
            }
            if receipt
                .experiences
                .iter()
                .any(|experience| experiences.contains(experience))
            {
                return Err(RelationExpressionPromotionError::OverlappingExperience);
            }
            experiences.extend(receipt.experiences.iter().copied());
            expressions_saved = expressions_saved
                .checked_add(receipt.expressions_without - receipt.expressions_with)
                .ok_or(RelationExpressionPromotionError::Capacity)?;
        }
        Ok(VerifiedRelationExpressionPromotion {
            mechanism,
            experiences,
            verified_reuses: u32::try_from(receipts.len())
                .map_err(|_| RelationExpressionPromotionError::Capacity)?,
            expressions_saved,
        })
    }
}

impl VerifiedRelationExpressionGrouping {
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
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RelationExpressionVerifyError {
    InsufficientEvidence,
    DuplicateEpisode,
    BindingMismatch,
    InvalidEvidence,
    InvalidBinding,
    RelationDefinition,
    RelationExecution,
    Capacity,
    Canonicalization,
    Entity(EntityObservationVerifyError),
}

/// Independent expression lowering and entity/mechanism replay.
#[derive(Clone, Copy, Debug, Default)]
pub struct RelationExpressionVerifier;

impl RelationExpressionVerifier {
    /// Reconstruct the complete RKL definition, evaluate the expression through
    /// a separate reference path, and replay the downstream mechanism.
    ///
    /// # Errors
    ///
    /// Rejects definition mismatch, malformed bindings/evidence, capacity, or
    /// any downstream exact-replay mismatch.
    #[allow(clippy::too_many_arguments)]
    pub fn verify(
        &self,
        learned: &LearnedRelationExpressionGrouping,
        traces: &[RawRelationObservationTrace],
        expression_limits: &RelationExpressionLimits,
        entity_limits: EntityObservationLimits,
        observation_limits: ObservationPredicateLimits,
        predicate_limits: PredicateGroupingLimits,
        execution_limits: ExecLimits,
    ) -> Result<VerifiedRelationExpressionGrouping, RelationExpressionVerifyError> {
        if traces.len() < 2 {
            return Err(RelationExpressionVerifyError::InsufficientEvidence);
        }
        let experiences = traces
            .iter()
            .map(|trace| trace.episode)
            .collect::<BTreeSet<_>>();
        if experiences.len() != traces.len() {
            return Err(RelationExpressionVerifyError::DuplicateEpisode);
        }
        if learned
            .selected_bindings
            .keys()
            .copied()
            .collect::<BTreeSet<_>>()
            != experiences
        {
            return Err(RelationExpressionVerifyError::BindingMismatch);
        }
        let expected_definition = verifier_definition(&learned.expression, expression_limits)?;
        if learned.relation_definition != expected_definition {
            return Err(RelationExpressionVerifyError::RelationDefinition);
        }
        let lowered = traces
            .iter()
            .map(|trace| {
                let binding = learned
                    .selected_bindings
                    .get(&trace.episode)
                    .ok_or(RelationExpressionVerifyError::BindingMismatch)?;
                verifier_materialize(trace, &learned.expression, binding, expression_limits)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let receipt = EntityObservationVerifier
            .verify(
                &learned.entity_grouping,
                &lowered,
                entity_limits,
                observation_limits,
                predicate_limits,
                execution_limits,
            )
            .map_err(RelationExpressionVerifyError::Entity)?;
        let mechanism = learned
            .mechanism_hash()
            .map_err(|_| RelationExpressionVerifyError::Canonicalization)?;
        if receipt.experiences() != &experiences {
            return Err(RelationExpressionVerifyError::BindingMismatch);
        }
        Ok(VerifiedRelationExpressionGrouping {
            mechanism,
            experiences,
            replayed_groups: receipt.replayed_groups(),
        })
    }

    /// Verify exact replay and independently derive the expression-structure
    /// reduction obtained by reusing this source-independent program.
    ///
    /// # Errors
    ///
    /// Returns exact replay errors or rejects absent learning reduction.
    #[allow(clippy::too_many_arguments)]
    pub fn verify_reuse(
        &self,
        learned: &LearnedRelationExpressionGrouping,
        traces: &[RawRelationObservationTrace],
        expression_limits: &RelationExpressionLimits,
        entity_limits: EntityObservationLimits,
        observation_limits: ObservationPredicateLimits,
        predicate_limits: PredicateGroupingLimits,
        execution_limits: ExecLimits,
    ) -> Result<VerifiedRelationExpressionReuse, RelationExpressionVerifyError> {
        let verified = self.verify(
            learned,
            traces,
            expression_limits,
            entity_limits,
            observation_limits,
            predicate_limits,
            execution_limits,
        )?;
        let operator_count = u64::try_from(expression_limits.permitted_binary_operators.len())
            .map_err(|_| RelationExpressionVerifyError::Capacity)?;
        let exponent = u32::try_from(expression_limits.parameter_arity.saturating_sub(1))
            .map_err(|_| RelationExpressionVerifyError::Capacity)?;
        let expressions_without = operator_count
            .checked_pow(exponent)
            .ok_or(RelationExpressionVerifyError::Capacity)?;
        if expressions_without <= 1 || learned.expression_candidates != expressions_without {
            return Err(RelationExpressionVerifyError::InvalidEvidence);
        }
        Ok(VerifiedRelationExpressionReuse {
            mechanism: verified.mechanism,
            experiences: verified.experiences,
            replayed_groups: verified.replayed_groups,
            expressions_without,
            expressions_with: 1,
        })
    }
}

fn verifier_materialize(
    trace: &RawRelationObservationTrace,
    expression: &RelationKeyExpression,
    binding: &RelationExpressionBinding,
    limits: &RelationExpressionLimits,
) -> Result<RawEntityObservationTrace, RelationExpressionVerifyError> {
    if binding.attributes.len() != limits.parameter_arity
        || binding
            .attributes
            .iter()
            .copied()
            .collect::<BTreeSet<_>>()
            .len()
            != binding.attributes.len()
    {
        return Err(RelationExpressionVerifyError::InvalidBinding);
    }
    let attribute_count = trace
        .primitives
        .values()
        .next()
        .map_or(0, |primitive| primitive.attributes.len());
    if attribute_count < limits.parameter_arity
        || trace.primitives.is_empty()
        || trace.primitives.values().any(|primitive| {
            primitive.attributes.len() != attribute_count
                || primitive.attributes.iter().any(|value| {
                    value
                        .checked_abs()
                        .is_none_or(|magnitude| magnitude > limits.maximum_absolute_attribute)
                })
        })
        || binding
            .attributes
            .iter()
            .any(|attribute| *attribute >= attribute_count)
    {
        return Err(RelationExpressionVerifyError::InvalidEvidence);
    }
    let role = verifier_role(binding);
    let items = trace.primitives.iter().collect::<Vec<_>>();
    let mut relations = Vec::new();
    let mut evaluations = 0_u64;
    for left_index in 0..items.len() {
        for right_index in (left_index + 1)..items.len() {
            evaluations = evaluations
                .checked_add(1)
                .ok_or(RelationExpressionVerifyError::Capacity)?;
            if evaluations > limits.maximum_relation_evaluations {
                return Err(RelationExpressionVerifyError::Capacity);
            }
            let (left_id, left) = items[left_index];
            let (right_id, right) = items[right_index];
            if verifier_evaluate(expression, &left.attributes, binding)?
                == verifier_evaluate(expression, &right.attributes, binding)?
            {
                relations.push(RelationRecord {
                    role: role.clone(),
                    subject: left_id.clone(),
                    object: right_id.clone(),
                });
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
        .collect::<BTreeMap<EntityId, _>>();
    Ok(RawEntityObservationTrace {
        episode: trace.episode,
        evidence: RawEntityObservationEvidence {
            primitives,
            relations,
        },
        outputs: trace.outputs.clone(),
    })
}

fn verifier_evaluate(
    expression: &RelationKeyExpression,
    attributes: &[i64],
    binding: &RelationExpressionBinding,
) -> Result<i64, RelationExpressionVerifyError> {
    match expression {
        RelationKeyExpression::Parameter(parameter) => binding
            .attributes
            .get(usize::try_from(*parameter).map_err(|_| RelationExpressionVerifyError::Capacity)?)
            .and_then(|attribute| attributes.get(*attribute))
            .copied()
            .ok_or(RelationExpressionVerifyError::InvalidBinding),
        RelationKeyExpression::Binary {
            operator,
            left,
            right,
        } => {
            let left = verifier_evaluate(left, attributes, binding)?;
            let right = verifier_evaluate(right, attributes, binding)?;
            match operator {
                RelationKeyOperator::Add => left
                    .checked_add(right)
                    .ok_or(RelationExpressionVerifyError::RelationExecution),
                RelationKeyOperator::Subtract => left
                    .checked_sub(right)
                    .ok_or(RelationExpressionVerifyError::RelationExecution),
                RelationKeyOperator::Minimum => Ok(left.min(right)),
                RelationKeyOperator::Maximum => Ok(left.max(right)),
                _ => Err(RelationExpressionVerifyError::RelationDefinition),
            }
        }
    }
}

fn verifier_definition(
    expression: &RelationKeyExpression,
    limits: &RelationExpressionLimits,
) -> Result<Definition, RelationExpressionVerifyError> {
    let arity = usize::try_from(expression.parameter_count())
        .map_err(|_| RelationExpressionVerifyError::Capacity)?;
    if arity != limits.parameter_arity
        || expression.binary_node_count() > limits.maximum_binary_nodes
    {
        return Err(RelationExpressionVerifyError::RelationDefinition);
    }
    let left = (0..arity)
        .map(|index| VarName(format!("left_attribute_{index}")))
        .collect::<Vec<_>>();
    let right = (0..arity)
        .map(|index| VarName(format!("right_attribute_{index}")))
        .collect::<Vec<_>>();
    let ty = Type::Int(IntRange {
        lower: -limits.maximum_absolute_attribute,
        upper: limits.maximum_absolute_attribute,
    });
    Ok(Definition {
        name: "invented_relation_expression".to_owned(),
        parameters: left
            .iter()
            .chain(&right)
            .cloned()
            .map(|name| Binder {
                name,
                ty: ty.clone(),
            })
            .collect(),
        return_type: Type::Bool,
        body: Expr::Binary {
            op: BinaryOp::Equal,
            left: Box::new(verifier_lower(expression, &left)?),
            right: Box::new(verifier_lower(expression, &right)?),
        },
        recursion: None,
        proof_annotations: Vec::new(),
    })
}

fn verifier_lower(
    expression: &RelationKeyExpression,
    variables: &[VarName],
) -> Result<Expr, RelationExpressionVerifyError> {
    match expression {
        RelationKeyExpression::Parameter(parameter) => variables
            .get(usize::try_from(*parameter).map_err(|_| RelationExpressionVerifyError::Capacity)?)
            .cloned()
            .map(Expr::Var)
            .ok_or(RelationExpressionVerifyError::InvalidBinding),
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
                _ => return Err(RelationExpressionVerifyError::RelationDefinition),
            },
            left: Box::new(verifier_lower(left, variables)?),
            right: Box::new(verifier_lower(right, variables)?),
        }),
    }
}

fn verifier_role(binding: &RelationExpressionBinding) -> RelationRole {
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
