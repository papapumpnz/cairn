use crate::{
    EntityObservationError, EntityObservationInventor, EntityObservationLimits,
    LearnedEntityObservationGrouping, LearnedObservationPredicateGrouping,
    ObservationPredicateLimits, PredicateGroupingLimits, PrimitiveObservationTrace,
    RawEntityObservationEvidence, RawEntityObservationTrace, RawObservationPrimitive,
};
use cairn_core::{CanonicalEncode, CanonicalValue, EntityId, Hash32, RelationRecord, RelationRole};
use cairn_invent::InventionLimits;
use cairn_rkl::{
    BinaryOp, Binder, BoundChecker, Definition, Expr, IntRange, Type, TypeChecker, VarName,
};
use std::collections::{BTreeMap, BTreeSet};

type EpisodeRelationBindings = BTreeMap<Hash32, BTreeMap<RelationRole, RelationAttributeBinding>>;
type MaterializedRelationEpisodes = (Vec<RawEntityObservationTrace>, EpisodeRelationBindings, u64);

/// One uninterpreted primitive observation with anonymous scalar attributes.
/// Attribute positions are task-local bindings and do not enter reusable
/// mechanism identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RawAttributedObservationPrimitive {
    pub observation: i64,
    pub attributes: Vec<i64>,
}

/// One episode before any binary relation or entity boundary is supplied.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RawRelationObservationTrace {
    pub episode: Hash32,
    pub primitives: BTreeMap<EntityId, RawAttributedObservationPrimitive>,
    pub outputs: BTreeSet<i64>,
}

/// Exact limits for relation-program proposal and materialization.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RelationObservationLimits {
    pub maximum_primitives: usize,
    pub maximum_attributes_per_primitive: usize,
    pub maximum_relation_evaluations: u64,
    pub maximum_absolute_attribute: i64,
    pub maximum_key_arity: usize,
    pub enabled_associative_folds: RelationFoldMask,
}

impl Default for RelationObservationLimits {
    fn default() -> Self {
        Self {
            maximum_primitives: 16_384,
            maximum_attributes_per_primitive: 32,
            maximum_relation_evaluations: 4_000_000,
            maximum_absolute_attribute: 1_000_000_000,
            maximum_key_arity: 2,
            enabled_associative_folds: RelationFoldMask::NONE,
        }
    }
}

/// Explicit capability mask for bounded associative relation-key folds.
/// Raising the arity cap alone does not make a fold family reachable.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RelationFoldMask(u8);

impl RelationFoldMask {
    pub const NONE: Self = Self(0);
    pub const ADD: Self = Self(1);
    pub const MINIMUM: Self = Self(2);
    pub const MAXIMUM: Self = Self(4);
    pub const ALL: Self = Self(Self::ADD.0 | Self::MINIMUM.0 | Self::MAXIMUM.0);

    #[must_use]
    pub const fn contains(self, operator: RelationKeyOperator) -> bool {
        let required = match operator {
            RelationKeyOperator::Sum => Self::ADD.0,
            RelationKeyOperator::MinimumFold => Self::MINIMUM.0,
            RelationKeyOperator::MaximumFold => Self::MAXIMUM.0,
            _ => 0,
        };
        required != 0 && self.0 & required == required
    }

    #[must_use]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    #[must_use]
    pub const fn for_operator(operator: RelationKeyOperator) -> Option<Self> {
        match operator {
            RelationKeyOperator::Sum => Some(Self::ADD),
            RelationKeyOperator::MinimumFold => Some(Self::MINIMUM),
            RelationKeyOperator::MaximumFold => Some(Self::MAXIMUM),
            _ => None,
        }
    }
}

/// Source-independent structure of one scalar relation key. The resulting
/// binary relation connects primitives whose evaluated keys are equal.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RelationKeyOperator {
    Projection,
    Add,
    Subtract,
    Minimum,
    Maximum,
    /// A bounded, variable-arity associative addition fold.
    Sum,
    MinimumFold,
    MaximumFold,
}

impl RelationKeyOperator {
    #[must_use]
    pub const fn accepts_arity(self, arity: usize) -> bool {
        match self {
            Self::Projection => arity == 1,
            Self::Add | Self::Subtract | Self::Minimum | Self::Maximum => arity == 2,
            Self::Sum | Self::MinimumFold | Self::MaximumFold => arity >= 3,
        }
    }

    #[must_use]
    pub const fn associative_fold(self) -> Option<Self> {
        match self {
            Self::Add => Some(Self::Sum),
            Self::Minimum => Some(Self::MinimumFold),
            Self::Maximum => Some(Self::MaximumFold),
            _ => None,
        }
    }
}

/// Task-local binding from relation-program parameter slots to anonymous
/// primitive attribute positions.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RelationAttributeBinding {
    pub operator: RelationKeyOperator,
    pub attributes: Vec<usize>,
}

/// An executable relation definition, task-local attribute bindings, and the
/// entity/mechanism theory selected by complete explanation.
#[derive(Clone, Debug, PartialEq)]
pub struct LearnedRawRelationObservationGrouping {
    pub relation_definition: Definition,
    pub selected_bindings: BTreeMap<Hash32, RelationAttributeBinding>,
    pub entity_grouping: LearnedEntityObservationGrouping,
    pub relation_evaluations: u64,
}

/// Source-independent relation structure and downstream executable mechanism.
/// It contains no episode identity, attribute index, generated relation role,
/// output, or acceptance authority.
#[derive(Clone, Debug, PartialEq)]
pub struct RawRelationMechanism {
    pub relation_definition: Definition,
    pub operator: RelationKeyOperator,
    pub key_arity: u32,
    pub observation_mechanism: LearnedObservationPredicateGrouping,
}

impl RawRelationMechanism {
    /// Canonical semantic identity of the executable composition.
    ///
    /// # Errors
    ///
    /// Returns canonical serialization failure.
    pub fn semantic_hash(&self) -> Result<Hash32, cairn_core::CanonicalError> {
        raw_relation_mechanism_hash(
            &self.relation_definition,
            self.operator,
            self.key_arity,
            &self.observation_mechanism,
        )
    }
}

impl LearnedRawRelationObservationGrouping {
    /// Source-independent identity over executable relation and downstream
    /// mechanism. Attribute positions and episode relation roles are excluded.
    ///
    /// # Errors
    ///
    /// Returns canonical serialization failure.
    pub fn mechanism_hash(&self) -> Result<Hash32, cairn_core::CanonicalError> {
        let structures = self
            .selected_bindings
            .values()
            .map(|binding| (binding.operator, binding.attributes.len()))
            .collect::<BTreeSet<_>>();
        let (operator, key_arity) = structures
            .first()
            .copied()
            .filter(|_| structures.len() == 1)
            .ok_or(cairn_core::CanonicalError::NonCanonical)?;
        raw_relation_mechanism_hash(
            &self.relation_definition,
            operator,
            u32::try_from(key_arity).map_err(|_| cairn_core::CanonicalError::IntegerOutOfRange)?,
            &self.entity_grouping.mechanism,
        )
    }

    #[must_use]
    pub const fn mechanism(&self) -> &LearnedObservationPredicateGrouping {
        &self.entity_grouping.mechanism
    }

    /// Remove task-local bindings and evidence lineage, retaining only the
    /// executable proposal structure.
    ///
    /// # Errors
    ///
    /// Rejects missing or structurally inconsistent selected bindings.
    pub fn source_independent(&self) -> Result<RawRelationMechanism, RelationObservationError> {
        let structures = self
            .selected_bindings
            .values()
            .map(|binding| (binding.operator, binding.attributes.len()))
            .collect::<BTreeSet<_>>();
        if structures.len() != 1 {
            return Err(RelationObservationError::InvalidBinding);
        }
        let (operator, key_arity) = structures
            .first()
            .copied()
            .ok_or(RelationObservationError::InvalidBinding)?;
        Ok(RawRelationMechanism {
            relation_definition: self.relation_definition.clone(),
            operator,
            key_arity: u32::try_from(key_arity).map_err(|_| RelationObservationError::Capacity)?,
            observation_mechanism: self.entity_grouping.mechanism.clone(),
        })
    }
}

fn raw_relation_mechanism_hash(
    relation_definition: &Definition,
    operator: RelationKeyOperator,
    key_arity: u32,
    observation_mechanism: &LearnedObservationPredicateGrouping,
) -> Result<Hash32, cairn_core::CanonicalError> {
    CanonicalValue::Array(vec![
        CanonicalValue::Text("cairn.raw-relation-observation.v2".to_owned()),
        CanonicalValue::Text(key_operator_name(operator).to_owned()),
        CanonicalValue::Unsigned(u64::from(key_arity)),
        CanonicalValue::Bytes(relation_definition.encode_canonical()?),
        CanonicalValue::Bytes(observation_mechanism.semantic_hash()?.as_bytes().to_vec()),
    ])
    .encode_canonical()
    .map(|bytes| Hash32::digest(&bytes))
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RelationObservationError {
    InsufficientEpisodes,
    DuplicateEpisode,
    EmptyEvidence,
    InvalidAttributes,
    Capacity,
    InvalidBinding,
    RelationDefinition,
    RelationExecution,
    Entity(EntityObservationError),
}

/// Reference learner that constructs a binary relation from the minimal RKL
/// equality substrate, binds it to anonymous attributes, and selects the
/// binding only through downstream executable explanation.
#[derive(Clone, Copy, Debug, Default)]
pub struct RawRelationObservationInventor;

impl RawRelationObservationInventor {
    /// Enumerate the bounded, deterministic task-local bindings currently
    /// expressible by the relation-key grammar.
    ///
    /// # Errors
    ///
    /// Rejects malformed or over-cap raw evidence.
    pub fn candidate_bindings(
        &self,
        trace: &RawRelationObservationTrace,
        limits: RelationObservationLimits,
    ) -> Result<Vec<RelationAttributeBinding>, RelationObservationError> {
        validate_trace(trace, limits)?;
        let attribute_count = trace
            .primitives
            .values()
            .next()
            .ok_or(RelationObservationError::EmptyEvidence)?
            .attributes
            .len();
        Ok(candidate_bindings(
            attribute_count,
            limits.maximum_key_arity,
            limits.enabled_associative_folds,
        ))
    }

    /// Jointly invent a relation binding, entity interpretation, observation
    /// features, selector, and aggregate.
    ///
    /// # Errors
    ///
    /// Returns typed malformed-evidence, capacity, ambiguity, or no-theory
    /// failures from the bounded reference search.
    pub fn invent(
        &self,
        traces: &[RawRelationObservationTrace],
        relation_limits: RelationObservationLimits,
        entity_limits: EntityObservationLimits,
        observation_limits: ObservationPredicateLimits,
        predicate_limits: PredicateGroupingLimits,
        invention_limits: InventionLimits,
    ) -> Result<LearnedRawRelationObservationGrouping, RelationObservationError> {
        validate_traces(traces, relation_limits)?;
        let (materialized, candidates, relation_evaluations) =
            materialize_all(traces, relation_limits)?;
        let phases = [
            BTreeSet::from([RelationKeyOperator::Projection]),
            BTreeSet::from([
                RelationKeyOperator::Add,
                RelationKeyOperator::Subtract,
                RelationKeyOperator::Minimum,
                RelationKeyOperator::Maximum,
            ]),
            BTreeSet::from([
                RelationKeyOperator::Sum,
                RelationKeyOperator::MinimumFold,
                RelationKeyOperator::MaximumFold,
            ]),
        ];
        let mut selected = None;
        for operators in phases {
            let (phase_traces, phase_candidates) =
                filter_materialized(&materialized, &candidates, &operators);
            match EntityObservationInventor.invent(
                &phase_traces,
                entity_limits,
                observation_limits,
                predicate_limits,
                invention_limits,
            ) {
                Ok(entity_grouping) => {
                    selected = Some((entity_grouping, phase_candidates));
                    break;
                }
                Err(
                    EntityObservationError::NoExactTheory | EntityObservationError::NoRelations,
                ) => {}
                Err(error) => return Err(RelationObservationError::Entity(error)),
            }
        }
        let (entity_grouping, selected_candidates) = selected.ok_or(
            RelationObservationError::Entity(EntityObservationError::NoExactTheory),
        )?;
        let selected_bindings = entity_grouping
            .selected_relations
            .iter()
            .map(|(episode, role)| {
                selected_candidates
                    .get(episode)
                    .and_then(|bindings| bindings.get(role))
                    .cloned()
                    .map(|binding| (*episode, binding))
                    .ok_or(RelationObservationError::InvalidBinding)
            })
            .collect::<Result<BTreeMap<_, _>, _>>()?;
        let structures = selected_bindings
            .values()
            .map(|binding| (binding.operator, binding.attributes.len()))
            .collect::<BTreeSet<_>>();
        if structures.len() != 1 {
            return Err(RelationObservationError::InvalidBinding);
        }
        let (operator, key_arity) = structures
            .first()
            .copied()
            .ok_or(RelationObservationError::InvalidBinding)?;
        let relation_definition = equality_definition(operator, key_arity, relation_limits)?;
        Ok(LearnedRawRelationObservationGrouping {
            relation_definition,
            selected_bindings,
            entity_grouping,
            relation_evaluations,
        })
    }

    /// Materialize one selected task-local attribute binding as the generic
    /// entity-observation reference trace.
    ///
    /// # Errors
    ///
    /// Rejects malformed evidence, invalid attribute bindings, or capacity.
    pub fn lower(
        &self,
        trace: &RawRelationObservationTrace,
        binding: &RelationAttributeBinding,
        relation_limits: RelationObservationLimits,
        entity_limits: EntityObservationLimits,
        observation_limits: ObservationPredicateLimits,
    ) -> Result<PrimitiveObservationTrace, RelationObservationError> {
        validate_trace(trace, relation_limits)?;
        let (materialized, _, _) = materialize_trace(trace, relation_limits)?;
        EntityObservationInventor
            .lower(
                &materialized,
                &role_for_binding(binding),
                entity_limits,
                observation_limits,
            )
            .map_err(RelationObservationError::Entity)
    }

    /// Execute a learned source-independent mechanism under one explicit
    /// current-task attribute binding as an untrusted proposal.
    ///
    /// # Errors
    ///
    /// Rejects malformed evidence, invalid bindings, or downstream execution.
    #[allow(clippy::too_many_arguments)]
    pub fn predict(
        &self,
        mechanism: &LearnedObservationPredicateGrouping,
        trace: &RawRelationObservationTrace,
        binding: &RelationAttributeBinding,
        relation_limits: RelationObservationLimits,
        entity_limits: EntityObservationLimits,
        observation_limits: ObservationPredicateLimits,
        predicate_limits: PredicateGroupingLimits,
    ) -> Result<BTreeSet<i64>, RelationObservationError> {
        validate_trace(trace, relation_limits)?;
        let (materialized, _, _) = materialize_trace(trace, relation_limits)?;
        EntityObservationInventor
            .predict(
                mechanism,
                &materialized,
                &role_for_binding(binding),
                entity_limits,
                observation_limits,
                predicate_limits,
            )
            .map_err(RelationObservationError::Entity)
    }
}

fn validate_traces(
    traces: &[RawRelationObservationTrace],
    limits: RelationObservationLimits,
) -> Result<(), RelationObservationError> {
    if traces.len() < 2 {
        return Err(RelationObservationError::InsufficientEpisodes);
    }
    let episodes = traces
        .iter()
        .map(|trace| trace.episode)
        .collect::<BTreeSet<_>>();
    if episodes.len() != traces.len() {
        return Err(RelationObservationError::DuplicateEpisode);
    }
    for trace in traces {
        validate_trace(trace, limits)?;
    }
    Ok(())
}

fn validate_trace(
    trace: &RawRelationObservationTrace,
    limits: RelationObservationLimits,
) -> Result<(), RelationObservationError> {
    if trace.primitives.is_empty() {
        return Err(RelationObservationError::EmptyEvidence);
    }
    if trace.primitives.len() > limits.maximum_primitives
        || limits.maximum_attributes_per_primitive == 0
        || limits.maximum_relation_evaluations == 0
        || limits.maximum_absolute_attribute <= 0
        || limits.maximum_key_arity == 0
        || limits.maximum_key_arity > limits.maximum_attributes_per_primitive
    {
        return Err(RelationObservationError::Capacity);
    }
    let Some(attribute_count) = trace
        .primitives
        .values()
        .next()
        .map(|primitive| primitive.attributes.len())
    else {
        return Err(RelationObservationError::EmptyEvidence);
    };
    if attribute_count == 0 || attribute_count > limits.maximum_attributes_per_primitive {
        return Err(RelationObservationError::InvalidAttributes);
    }
    if trace.primitives.values().any(|primitive| {
        primitive.attributes.len() != attribute_count
            || primitive.attributes.iter().any(|attribute| {
                attribute
                    .checked_abs()
                    .is_none_or(|magnitude| magnitude > limits.maximum_absolute_attribute)
            })
    }) {
        return Err(RelationObservationError::InvalidAttributes);
    }
    Ok(())
}

fn equality_definition(
    operator: RelationKeyOperator,
    key_arity: usize,
    limits: RelationObservationLimits,
) -> Result<Definition, RelationObservationError> {
    if !operator.accepts_arity(key_arity) {
        return Err(RelationObservationError::InvalidBinding);
    }
    let left = (0..key_arity)
        .map(|index| VarName(format!("left_attribute_{index}")))
        .collect::<Vec<_>>();
    let right = (0..key_arity)
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
    let key_expression = |variables: &[VarName]| match operator {
        RelationKeyOperator::Projection => Expr::Var(variables[0].clone()),
        RelationKeyOperator::Add => binary_key(BinaryOp::Add, variables),
        RelationKeyOperator::Subtract => binary_key(BinaryOp::Subtract, variables),
        RelationKeyOperator::Minimum => binary_key(BinaryOp::Minimum, variables),
        RelationKeyOperator::Maximum => binary_key(BinaryOp::Maximum, variables),
        RelationKeyOperator::Sum => fold_key(BinaryOp::Add, variables),
        RelationKeyOperator::MinimumFold => fold_key(BinaryOp::Minimum, variables),
        RelationKeyOperator::MaximumFold => fold_key(BinaryOp::Maximum, variables),
    };
    let definition = Definition {
        name: "invented_primitive_relation".to_owned(),
        parameters,
        return_type: Type::Bool,
        body: Expr::Binary {
            op: BinaryOp::Equal,
            left: Box::new(key_expression(&left)),
            right: Box::new(key_expression(&right)),
        },
        recursion: None,
        proof_annotations: Vec::new(),
    };
    TypeChecker::new(128)
        .check_definition(&definition)
        .map_err(|_| RelationObservationError::RelationDefinition)?;
    Ok(definition)
}

fn binary_key(operation: BinaryOp, variables: &[VarName]) -> Expr {
    Expr::Binary {
        op: operation,
        left: Box::new(Expr::Var(variables[0].clone())),
        right: Box::new(Expr::Var(variables[1].clone())),
    }
}

fn fold_key(operation: BinaryOp, variables: &[VarName]) -> Expr {
    variables[1..]
        .iter()
        .cloned()
        .fold(Expr::Var(variables[0].clone()), |left, right| {
            Expr::Binary {
                op: operation,
                left: Box::new(left),
                right: Box::new(Expr::Var(right)),
            }
        })
}

fn materialize_all(
    traces: &[RawRelationObservationTrace],
    limits: RelationObservationLimits,
) -> Result<MaterializedRelationEpisodes, RelationObservationError> {
    let mut evaluations = 0_u64;
    let mut candidates = BTreeMap::new();
    let materialized = traces
        .iter()
        .map(|trace| {
            let (materialized, bindings, work) = materialize_trace(trace, limits)?;
            evaluations = evaluations
                .checked_add(work)
                .ok_or(RelationObservationError::Capacity)?;
            if evaluations > limits.maximum_relation_evaluations {
                return Err(RelationObservationError::Capacity);
            }
            candidates.insert(trace.episode, bindings);
            Ok(materialized)
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok((materialized, candidates, evaluations))
}

fn filter_materialized(
    traces: &[RawEntityObservationTrace],
    candidates: &BTreeMap<Hash32, BTreeMap<RelationRole, RelationAttributeBinding>>,
    operators: &BTreeSet<RelationKeyOperator>,
) -> (
    Vec<RawEntityObservationTrace>,
    BTreeMap<Hash32, BTreeMap<RelationRole, RelationAttributeBinding>>,
) {
    let filtered_candidates = candidates
        .iter()
        .map(|(episode, bindings)| {
            (
                *episode,
                bindings
                    .iter()
                    .filter(|(_, binding)| operators.contains(&binding.operator))
                    .map(|(role, binding)| (role.clone(), binding.clone()))
                    .collect::<BTreeMap<_, _>>(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let filtered_traces = traces
        .iter()
        .cloned()
        .map(|mut trace| {
            trace.evidence.relations.retain(|relation| {
                filtered_candidates
                    .get(&trace.episode)
                    .is_some_and(|bindings| bindings.contains_key(&relation.role))
            });
            trace
        })
        .collect();
    (filtered_traces, filtered_candidates)
}

fn materialize_trace(
    trace: &RawRelationObservationTrace,
    limits: RelationObservationLimits,
) -> Result<
    (
        RawEntityObservationTrace,
        BTreeMap<RelationRole, RelationAttributeBinding>,
        u64,
    ),
    RelationObservationError,
> {
    validate_trace(trace, limits)?;
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
        .collect::<BTreeMap<_, _>>();
    let attribute_count = trace
        .primitives
        .values()
        .next()
        .ok_or(RelationObservationError::EmptyEvidence)?
        .attributes
        .len();
    let items = trace.primitives.iter().collect::<Vec<_>>();
    let mut relations = Vec::new();
    let mut evaluations = 0_u64;
    let candidates = candidate_bindings(
        attribute_count,
        limits.maximum_key_arity,
        limits.enabled_associative_folds,
    );
    let mut role_bindings = BTreeMap::new();
    for binding in candidates {
        let role = role_for_binding(&binding);
        role_bindings.insert(role.clone(), binding.clone());
        for left_index in 0..items.len() {
            for right_index in (left_index + 1)..items.len() {
                evaluations = evaluations
                    .checked_add(1)
                    .ok_or(RelationObservationError::Capacity)?;
                if evaluations > limits.maximum_relation_evaluations {
                    return Err(RelationObservationError::Capacity);
                }
                let (left_id, left) = items[left_index];
                let (right_id, right) = items[right_index];
                let left_key = evaluate_key(&left.attributes, &binding)?;
                let right_key = evaluate_key(&right.attributes, &binding)?;
                if left_key == right_key {
                    relations.push(RelationRecord {
                        role: role.clone(),
                        subject: left_id.clone(),
                        object: right_id.clone(),
                    });
                }
            }
        }
    }
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

fn candidate_bindings(
    attribute_count: usize,
    maximum_key_arity: usize,
    enabled_associative_folds: RelationFoldMask,
) -> Vec<RelationAttributeBinding> {
    let mut output = (0..attribute_count)
        .map(|attribute| RelationAttributeBinding {
            operator: RelationKeyOperator::Projection,
            attributes: vec![attribute],
        })
        .collect::<Vec<_>>();
    for left in 0..attribute_count {
        for right in (left + 1)..attribute_count {
            for operator in [
                RelationKeyOperator::Add,
                RelationKeyOperator::Subtract,
                RelationKeyOperator::Minimum,
                RelationKeyOperator::Maximum,
            ] {
                output.push(RelationAttributeBinding {
                    operator,
                    attributes: vec![left, right],
                });
            }
        }
    }
    for arity in 3..=maximum_key_arity.min(attribute_count) {
        let mut combinations = Vec::new();
        collect_attribute_combinations(
            attribute_count,
            arity,
            0,
            &mut Vec::new(),
            &mut combinations,
        );
        for attributes in combinations {
            for operator in [
                RelationKeyOperator::Sum,
                RelationKeyOperator::MinimumFold,
                RelationKeyOperator::MaximumFold,
            ] {
                if enabled_associative_folds.contains(operator) {
                    output.push(RelationAttributeBinding {
                        operator,
                        attributes: attributes.clone(),
                    });
                }
            }
        }
    }
    output
}

fn collect_attribute_combinations(
    attribute_count: usize,
    arity: usize,
    next: usize,
    current: &mut Vec<usize>,
    output: &mut Vec<Vec<usize>>,
) {
    if current.len() == arity {
        output.push(current.clone());
        return;
    }
    let remaining = arity - current.len();
    for attribute in next..=attribute_count - remaining {
        current.push(attribute);
        collect_attribute_combinations(attribute_count, arity, attribute + 1, current, output);
        current.pop();
    }
}

fn role_for_binding(binding: &RelationAttributeBinding) -> RelationRole {
    let operator = key_operator_name(binding.operator);
    RelationRole(format!(
        "cairn.task-local.key.{operator}.{}",
        binding
            .attributes
            .iter()
            .map(usize::to_string)
            .collect::<Vec<_>>()
            .join(".")
    ))
}

const fn key_operator_name(operator: RelationKeyOperator) -> &'static str {
    match operator {
        RelationKeyOperator::Projection => "project",
        RelationKeyOperator::Add => "add",
        RelationKeyOperator::Subtract => "subtract",
        RelationKeyOperator::Minimum => "minimum",
        RelationKeyOperator::Maximum => "maximum",
        RelationKeyOperator::Sum => "sum",
        RelationKeyOperator::MinimumFold => "minimum-fold",
        RelationKeyOperator::MaximumFold => "maximum-fold",
    }
}

fn evaluate_key(
    attributes: &[i64],
    binding: &RelationAttributeBinding,
) -> Result<i64, RelationObservationError> {
    if !binding.operator.accepts_arity(binding.attributes.len())
        || binding
            .attributes
            .iter()
            .any(|attribute| *attribute >= attributes.len())
    {
        return Err(RelationObservationError::InvalidBinding);
    }
    let left = attributes[binding.attributes[0]];
    match binding.operator {
        RelationKeyOperator::Projection => Ok(left),
        RelationKeyOperator::Add => left
            .checked_add(attributes[binding.attributes[1]])
            .ok_or(RelationObservationError::RelationExecution),
        RelationKeyOperator::Subtract => left
            .checked_sub(attributes[binding.attributes[1]])
            .ok_or(RelationObservationError::RelationExecution),
        RelationKeyOperator::Minimum => Ok(left.min(attributes[binding.attributes[1]])),
        RelationKeyOperator::Maximum => Ok(left.max(attributes[binding.attributes[1]])),
        RelationKeyOperator::Sum => binding.attributes[1..].iter().try_fold(left, |sum, index| {
            sum.checked_add(attributes[*index])
                .ok_or(RelationObservationError::RelationExecution)
        }),
        RelationKeyOperator::MinimumFold => Ok(binding.attributes[1..]
            .iter()
            .fold(left, |minimum, index| minimum.min(attributes[*index]))),
        RelationKeyOperator::MaximumFold => Ok(binding.attributes[1..]
            .iter()
            .fold(left, |maximum, index| maximum.max(attributes[*index]))),
    }
}
