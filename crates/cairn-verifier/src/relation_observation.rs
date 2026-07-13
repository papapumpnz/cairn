use crate::{EntityObservationVerifier, EntityObservationVerifyError};
use cairn_core::{Hash32, RelationRecord, RelationRole};
use cairn_curriculum::RelationLanguageGrowthRequest;
use cairn_rkl::{
    ExecLimits, ReferenceExecutor, RuntimeEnvironment, TraceLevel, Type, TypeChecker,
    TypeEnvironment, Value,
};
use cairn_theory::{
    EntityObservationLimits, LearnedEntityObservationGrouping,
    LearnedRawRelationObservationGrouping, ObservationPredicateLimits, PredicateGroupingLimits,
    RawAttributedObservationPrimitive, RawEntityObservationEvidence, RawEntityObservationTrace,
    RawObservationPrimitive, RawRelationMechanism, RawRelationObservationInventor,
    RawRelationObservationTrace, RelationAttributeBinding, RelationFoldMask, RelationKeyOperator,
    RelationObservationLimits,
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedRawRelationObservationGrouping {
    mechanism: Hash32,
    experiences: BTreeSet<Hash32>,
    relation_executions: u64,
    replayed_groups: u64,
}

impl VerifiedRawRelationObservationGrouping {
    #[must_use]
    pub const fn mechanism(&self) -> Hash32 {
        self.mechanism
    }

    #[must_use]
    pub const fn experiences(&self) -> &BTreeSet<Hash32> {
        &self.experiences
    }

    #[must_use]
    pub const fn relation_executions(&self) -> u64 {
        self.relation_executions
    }

    #[must_use]
    pub const fn replayed_groups(&self) -> u64 {
        self.replayed_groups
    }
}

/// Opaque verifier evidence that one source-independent relation mechanism
/// replayed a disjoint cohort while reducing relation-structure search.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedRawRelationObservationReuse {
    mechanism: Hash32,
    experiences: BTreeSet<Hash32>,
    replayed_groups: u64,
    bindings_without: u64,
    bindings_with: u64,
    operator: RelationKeyOperator,
    key_arity: u32,
    minimum_attributes: u32,
    maximum_attributes: u32,
}

impl VerifiedRawRelationObservationReuse {
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
    pub const fn bindings_without(&self) -> u64 {
        self.bindings_without
    }
    #[must_use]
    pub const fn bindings_with(&self) -> u64 {
        self.bindings_with
    }
    #[must_use]
    pub const fn operator(&self) -> RelationKeyOperator {
        self.operator
    }
    #[must_use]
    pub const fn key_arity(&self) -> u32 {
        self.key_arity
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RawRelationSupportGeometry {
    pub operator: RelationKeyOperator,
    pub key_arity: u32,
    pub minimum_attributes: u32,
    pub maximum_attributes: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedRawRelationObservationPromotion {
    mechanism: Hash32,
    experiences: BTreeSet<Hash32>,
    verified_reuses: u32,
    bindings_saved: u64,
    support: RawRelationSupportGeometry,
}

impl VerifiedRawRelationObservationPromotion {
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
    pub const fn bindings_saved(&self) -> u64 {
        self.bindings_saved
    }
    #[must_use]
    pub const fn support(&self) -> RawRelationSupportGeometry {
        self.support
    }
}

/// Bounded policy for verifier-authorized relation-language growth.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RelationLanguageExtensionLimits {
    pub maximum_key_arity: u32,
}

impl Default for RelationLanguageExtensionLimits {
    fn default() -> Self {
        Self {
            maximum_key_arity: 8,
        }
    }
}

/// Opaque evidence that a proof-gated binary mechanism may expose its
/// associative fold family to later proposal search. It grants no answer or
/// promotion authority.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VerifiedRelationLanguageExtension {
    source_mechanism: Hash32,
    failure_core: Hash32,
    fold_operator: RelationKeyOperator,
    maximum_key_arity: u32,
}

impl VerifiedRelationLanguageExtension {
    #[must_use]
    pub const fn source_mechanism(&self) -> Hash32 {
        self.source_mechanism
    }

    #[must_use]
    pub const fn failure_core(&self) -> Hash32 {
        self.failure_core
    }

    #[must_use]
    pub const fn fold_operator(&self) -> RelationKeyOperator {
        self.fold_operator
    }

    #[must_use]
    pub const fn maximum_key_arity(&self) -> u32 {
        self.maximum_key_arity
    }

    /// Apply only proposal-language reachability to existing resource limits.
    /// Existing caps may grow to the verified arity but never beyond it.
    ///
    /// # Errors
    ///
    /// Rejects platform-size overflow or an invalid fold certificate.
    pub fn apply(
        &self,
        mut limits: RelationObservationLimits,
    ) -> Result<RelationObservationLimits, RelationLanguageExtensionError> {
        let maximum_key_arity = usize::try_from(self.maximum_key_arity)
            .map_err(|_| RelationLanguageExtensionError::Capacity)?;
        let mask = RelationFoldMask::for_operator(self.fold_operator)
            .ok_or(RelationLanguageExtensionError::UnsupportedBase)?;
        limits.maximum_key_arity = limits.maximum_key_arity.max(maximum_key_arity);
        limits.enabled_associative_folds = limits.enabled_associative_folds.union(mask);
        Ok(limits)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RelationLanguageExtensionError {
    InvalidPromotion,
    UnsupportedBase,
    InvalidRequestedArity,
    Capacity,
}

/// Separate authority that converts verified reuse plus a content-free
/// structural failure into bounded proposal-language reachability.
#[derive(Clone, Copy, Debug, Default)]
pub struct RelationLanguageExtensionConsolidator;

impl RelationLanguageExtensionConsolidator {
    /// Certify only associative closure already justified by a persistent
    /// binary mechanism and an independently identified dependency-arity gap.
    ///
    /// # Errors
    ///
    /// Rejects mismatched promotion, non-associative base operations, arities
    /// below three, or the declared safety cap.
    pub fn consolidate(
        &self,
        mechanism: &RawRelationMechanism,
        promotion: &VerifiedRawRelationObservationPromotion,
        request: &RelationLanguageGrowthRequest,
        limits: RelationLanguageExtensionLimits,
    ) -> Result<VerifiedRelationLanguageExtension, RelationLanguageExtensionError> {
        let mechanism_id = mechanism
            .semantic_hash()
            .map_err(|_| RelationLanguageExtensionError::InvalidPromotion)?;
        if promotion.mechanism() != mechanism_id
            || promotion.verified_reuses() < 2
            || promotion.support().operator != mechanism.operator
            || promotion.support().key_arity != mechanism.key_arity
            || mechanism.key_arity != 2
        {
            return Err(RelationLanguageExtensionError::InvalidPromotion);
        }
        let fold_operator = mechanism
            .operator
            .associative_fold()
            .ok_or(RelationLanguageExtensionError::UnsupportedBase)?;
        let requested_arity = request.minimum_dependency_arity_lower_bound();
        if requested_arity < 3 || requested_arity > limits.maximum_key_arity {
            return Err(RelationLanguageExtensionError::InvalidRequestedArity);
        }
        Ok(VerifiedRelationLanguageExtension {
            source_mechanism: mechanism_id,
            failure_core: request.failure_core(),
            fold_operator,
            maximum_key_arity: requested_arity,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RawRelationObservationPromotionError {
    Verification(RawRelationObservationVerifyError),
    InvalidMechanism,
    NoLearningReduction,
    InsufficientIndependentEvidence,
    MixedMechanism,
    OverlappingExperience,
    Capacity,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RawRelationObservationVerifyError {
    InsufficientEvidence,
    DuplicateEpisode,
    BindingMismatch,
    InvalidEvidence,
    InvalidBinding,
    Capacity,
    RelationDefinition,
    RelationExecution,
    Canonicalization,
    Entity(EntityObservationVerifyError),
}

/// Independent RKL execution of an invented primitive relation followed by
/// independent entity reconstruction and complete mechanism replay.
#[derive(Clone, Copy, Debug, Default)]
pub struct RawRelationObservationVerifier;

impl RawRelationObservationVerifier {
    /// Replay the learned relation definition directly on raw attributes and
    /// verify the resulting entity/mechanism theory through the independent
    /// entity verifier.
    ///
    /// # Errors
    ///
    /// Rejects malformed bindings, invalid RKL, resource caps, or any replay
    /// mismatch.
    #[allow(clippy::too_many_arguments)] // Keeps independent resource limits explicit at trust boundary.
    pub fn verify(
        &self,
        learned: &LearnedRawRelationObservationGrouping,
        traces: &[RawRelationObservationTrace],
        relation_limits: RelationObservationLimits,
        entity_limits: EntityObservationLimits,
        observation_limits: ObservationPredicateLimits,
        predicate_limits: PredicateGroupingLimits,
        execution_limits: ExecLimits,
    ) -> Result<VerifiedRawRelationObservationGrouping, RawRelationObservationVerifyError> {
        if traces.len() < 2 {
            return Err(RawRelationObservationVerifyError::InsufficientEvidence);
        }
        let experiences = traces
            .iter()
            .map(|trace| trace.episode)
            .collect::<BTreeSet<_>>();
        if experiences.len() != traces.len() {
            return Err(RawRelationObservationVerifyError::DuplicateEpisode);
        }
        if learned
            .selected_bindings
            .keys()
            .copied()
            .collect::<BTreeSet<_>>()
            != experiences
        {
            return Err(RawRelationObservationVerifyError::BindingMismatch);
        }
        if learned.relation_definition.return_type != Type::Bool {
            return Err(RawRelationObservationVerifyError::RelationDefinition);
        }
        let mut type_environment = TypeEnvironment::new();
        for parameter in &learned.relation_definition.parameters {
            type_environment.insert(parameter.name.clone(), parameter.ty.clone());
        }
        let relation_expression = TypeChecker::new(10_000)
            .check(&learned.relation_definition.body, &type_environment)
            .map_err(|_| RawRelationObservationVerifyError::RelationDefinition)?;
        if relation_expression.ty() != &Type::Bool {
            return Err(RawRelationObservationVerifyError::RelationDefinition);
        }
        let executor = ReferenceExecutor::without_definitions(execution_limits, TraceLevel::Off);
        let mut relation_executions = 0_u64;
        let mut selected_relations = BTreeMap::new();
        let materialized = traces
            .iter()
            .map(|trace| {
                let binding = learned
                    .selected_bindings
                    .get(&trace.episode)
                    .cloned()
                    .ok_or(RawRelationObservationVerifyError::BindingMismatch)?;
                if learned.relation_definition.parameters.len() != binding.attributes.len() * 2 {
                    return Err(RawRelationObservationVerifyError::RelationDefinition);
                }
                let role = RelationRole("cairn.verifier-local.relation".to_owned());
                let replay = verifier_materialize(
                    trace,
                    &binding,
                    &learned.relation_definition.parameters,
                    &relation_expression,
                    &executor,
                    relation_limits,
                    &mut relation_executions,
                    &role,
                )?;
                selected_relations.insert(trace.episode, role);
                Ok(replay)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let entity_grouping = LearnedEntityObservationGrouping {
            selected_relations,
            mechanism: learned.entity_grouping.mechanism.clone(),
            binding_sets_evaluated: learned.entity_grouping.binding_sets_evaluated,
        };
        let entity_receipt = EntityObservationVerifier
            .verify(
                &entity_grouping,
                &materialized,
                entity_limits,
                observation_limits,
                predicate_limits,
                execution_limits,
            )
            .map_err(RawRelationObservationVerifyError::Entity)?;
        let mechanism = learned
            .mechanism_hash()
            .map_err(|_| RawRelationObservationVerifyError::Canonicalization)?;
        Ok(VerifiedRawRelationObservationGrouping {
            mechanism,
            experiences,
            relation_executions,
            replayed_groups: entity_receipt.replayed_groups(),
        })
    }

    /// Verify exact replay and derive search-reduction evidence rather than
    /// accepting caller-authored learning metrics.
    ///
    /// # Errors
    ///
    /// Rejects replay failure, inconsistent mechanism structure, no strict
    /// binding reduction, or numeric capacity.
    #[allow(clippy::too_many_arguments)]
    pub fn verify_reuse(
        &self,
        learned: &LearnedRawRelationObservationGrouping,
        traces: &[RawRelationObservationTrace],
        relation_limits: RelationObservationLimits,
        entity_limits: EntityObservationLimits,
        observation_limits: ObservationPredicateLimits,
        predicate_limits: PredicateGroupingLimits,
        execution_limits: ExecLimits,
    ) -> Result<VerifiedRawRelationObservationReuse, RawRelationObservationPromotionError> {
        let verified = self
            .verify(
                learned,
                traces,
                relation_limits,
                entity_limits,
                observation_limits,
                predicate_limits,
                execution_limits,
            )
            .map_err(RawRelationObservationPromotionError::Verification)?;
        let mechanism = learned
            .source_independent()
            .map_err(|_| RawRelationObservationPromotionError::InvalidMechanism)?;
        let mechanism_hash = mechanism
            .semantic_hash()
            .map_err(|_| RawRelationObservationPromotionError::InvalidMechanism)?;
        if mechanism_hash != verified.mechanism() {
            return Err(RawRelationObservationPromotionError::InvalidMechanism);
        }
        let mut bindings_without = 0_u64;
        let mut bindings_with = 0_u64;
        let mut minimum_attributes = u32::MAX;
        let mut maximum_attributes = 0_u32;
        for trace in traces {
            let bindings = RawRelationObservationInventor
                .candidate_bindings(trace, relation_limits)
                .map_err(|_| RawRelationObservationPromotionError::Capacity)?;
            bindings_without = bindings_without
                .checked_add(
                    u64::try_from(bindings.len())
                        .map_err(|_| RawRelationObservationPromotionError::Capacity)?,
                )
                .ok_or(RawRelationObservationPromotionError::Capacity)?;
            bindings_with = bindings_with
                .checked_add(
                    u64::try_from(
                        bindings
                            .iter()
                            .filter(|binding| {
                                binding.operator == mechanism.operator
                                    && u32::try_from(binding.attributes.len())
                                        .is_ok_and(|arity| arity == mechanism.key_arity)
                            })
                            .count(),
                    )
                    .map_err(|_| RawRelationObservationPromotionError::Capacity)?,
                )
                .ok_or(RawRelationObservationPromotionError::Capacity)?;
            let attributes = trace
                .primitives
                .values()
                .next()
                .map_or(0, |primitive| primitive.attributes.len());
            let attributes = u32::try_from(attributes)
                .map_err(|_| RawRelationObservationPromotionError::Capacity)?;
            minimum_attributes = minimum_attributes.min(attributes);
            maximum_attributes = maximum_attributes.max(attributes);
        }
        if bindings_without <= bindings_with || bindings_with == 0 {
            return Err(RawRelationObservationPromotionError::NoLearningReduction);
        }
        Ok(VerifiedRawRelationObservationReuse {
            mechanism: mechanism_hash,
            experiences: verified.experiences,
            replayed_groups: verified.replayed_groups,
            bindings_without,
            bindings_with,
            operator: mechanism.operator,
            key_arity: mechanism.key_arity,
            minimum_attributes,
            maximum_attributes,
        })
    }
}

/// Separate consolidation authority for promotion of complete relation
/// programs. It consumes only opaque independent verifier receipts.
#[derive(Clone, Copy, Debug, Default)]
pub struct RawRelationObservationPromotionConsolidator;

impl RawRelationObservationPromotionConsolidator {
    /// Require two disjoint exact reuses of one mechanism with strict search
    /// reduction and issue a complete-ledger promotion certificate.
    ///
    /// # Errors
    ///
    /// Rejects insufficient, overlapping, mixed, or non-reducing evidence.
    pub fn consolidate(
        &self,
        receipts: &[VerifiedRawRelationObservationReuse],
    ) -> Result<VerifiedRawRelationObservationPromotion, RawRelationObservationPromotionError> {
        if receipts.len() < 2 {
            return Err(RawRelationObservationPromotionError::InsufficientIndependentEvidence);
        }
        let mechanism = receipts[0].mechanism;
        let operator = receipts[0].operator;
        let key_arity = receipts[0].key_arity;
        let mut experiences = BTreeSet::new();
        let mut bindings_saved = 0_u64;
        let mut minimum_attributes = u32::MAX;
        let mut maximum_attributes = 0_u32;
        for receipt in receipts {
            if receipt.mechanism != mechanism
                || receipt.operator != operator
                || receipt.key_arity != key_arity
            {
                return Err(RawRelationObservationPromotionError::MixedMechanism);
            }
            if receipt.bindings_without <= receipt.bindings_with {
                return Err(RawRelationObservationPromotionError::NoLearningReduction);
            }
            if !experiences.is_disjoint(&receipt.experiences) {
                return Err(RawRelationObservationPromotionError::OverlappingExperience);
            }
            experiences.extend(&receipt.experiences);
            bindings_saved = bindings_saved
                .checked_add(receipt.bindings_without - receipt.bindings_with)
                .ok_or(RawRelationObservationPromotionError::Capacity)?;
            minimum_attributes = minimum_attributes.min(receipt.minimum_attributes);
            maximum_attributes = maximum_attributes.max(receipt.maximum_attributes);
        }
        Ok(VerifiedRawRelationObservationPromotion {
            mechanism,
            experiences,
            verified_reuses: u32::try_from(receipts.len())
                .map_err(|_| RawRelationObservationPromotionError::Capacity)?,
            bindings_saved,
            support: RawRelationSupportGeometry {
                operator,
                key_arity,
                minimum_attributes,
                maximum_attributes,
            },
        })
    }
}

#[allow(clippy::too_many_arguments)]
fn verifier_materialize(
    trace: &RawRelationObservationTrace,
    binding: &RelationAttributeBinding,
    parameters: &[cairn_rkl::Binder],
    expression: &cairn_rkl::TypedExpr,
    executor: &ReferenceExecutor<'_>,
    limits: RelationObservationLimits,
    relation_executions: &mut u64,
    role: &RelationRole,
) -> Result<RawEntityObservationTrace, RawRelationObservationVerifyError> {
    verifier_validate(trace, binding, limits)?;
    let items = trace.primitives.iter().collect::<Vec<_>>();
    let mut relations = Vec::new();
    for left_index in 0..items.len() {
        for right_index in (left_index + 1)..items.len() {
            *relation_executions = relation_executions
                .checked_add(1)
                .ok_or(RawRelationObservationVerifyError::Capacity)?;
            if *relation_executions > limits.maximum_relation_evaluations {
                return Err(RawRelationObservationVerifyError::Capacity);
            }
            let (left_id, left) = items[left_index];
            let (right_id, right) = items[right_index];
            if execute_relation(
                parameters,
                expression,
                executor,
                &left.attributes,
                &right.attributes,
                binding,
            )? {
                relations.push(RelationRecord {
                    role: role.clone(),
                    subject: left_id.clone(),
                    object: right_id.clone(),
                });
            }
        }
    }
    Ok(RawEntityObservationTrace {
        episode: trace.episode,
        evidence: RawEntityObservationEvidence {
            primitives: trace
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
                .collect(),
            relations,
        },
        outputs: trace.outputs.clone(),
    })
}

fn verifier_validate(
    trace: &RawRelationObservationTrace,
    binding: &RelationAttributeBinding,
    limits: RelationObservationLimits,
) -> Result<(), RawRelationObservationVerifyError> {
    if trace.primitives.is_empty() {
        return Err(RawRelationObservationVerifyError::InvalidEvidence);
    }
    if trace.primitives.len() > limits.maximum_primitives
        || limits.maximum_attributes_per_primitive == 0
        || limits.maximum_relation_evaluations == 0
        || limits.maximum_absolute_attribute <= 0
    {
        return Err(RawRelationObservationVerifyError::Capacity);
    }
    let attribute_count = trace
        .primitives
        .values()
        .next()
        .map_or(0, |primitive| primitive.attributes.len());
    if attribute_count == 0
        || attribute_count > limits.maximum_attributes_per_primitive
        || trace
            .primitives
            .values()
            .any(|primitive: &RawAttributedObservationPrimitive| {
                primitive.attributes.len() != attribute_count
                    || primitive.attributes.iter().any(|attribute| {
                        attribute
                            .checked_abs()
                            .is_none_or(|magnitude| magnitude > limits.maximum_absolute_attribute)
                    })
            })
    {
        return Err(RawRelationObservationVerifyError::InvalidEvidence);
    }
    if !binding.operator.accepts_arity(binding.attributes.len())
        || binding
            .attributes
            .iter()
            .any(|attribute| *attribute >= attribute_count)
    {
        return Err(RawRelationObservationVerifyError::InvalidBinding);
    }
    Ok(())
}

fn execute_relation(
    parameters: &[cairn_rkl::Binder],
    expression: &cairn_rkl::TypedExpr,
    executor: &ReferenceExecutor<'_>,
    left: &[i64],
    right: &[i64],
    binding: &RelationAttributeBinding,
) -> Result<bool, RawRelationObservationVerifyError> {
    let mut environment = RuntimeEnvironment::new();
    for (index, attribute) in binding.attributes.iter().copied().enumerate() {
        environment.insert(parameters[index].name.clone(), Value::Int(left[attribute]));
        environment.insert(
            parameters[binding.attributes.len() + index].name.clone(),
            Value::Int(right[attribute]),
        );
    }
    match executor
        .execute(expression, &environment)
        .map_err(|_| RawRelationObservationVerifyError::RelationExecution)?
        .result
    {
        Value::Bool(related) => Ok(related),
        _ => Err(RawRelationObservationVerifyError::RelationExecution),
    }
}
