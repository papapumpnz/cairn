use crate::{GroupMergeError, GroupMergeInventor, GroupMergeTrace, LearnedGroupMerge};
use cairn_core::{CanonicalEncode, CanonicalValue, EntityId, Hash32};
use cairn_invent::InventionLimits;
use cairn_rkl::{
    BinaryOp, Binder, BoundChecker, Definition, ExecLimits, Expr, IntRange, ReferenceExecutor,
    RuntimeEnvironment, TraceLevel, Type, TypeChecker, TypeEnvironment, Value, VarName,
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AttributedGroupingEntity {
    pub id: EntityId,
    pub value: i64,
    pub features: Vec<i64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AttributedGroupingTrace {
    pub episode: Hash32,
    pub entities: Vec<AttributedGroupingEntity>,
    pub outputs: BTreeSet<i64>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PredicateGroupingLimits {
    pub maximum_entities: usize,
    pub maximum_features: usize,
    pub maximum_relation_candidates: usize,
}

impl Default for PredicateGroupingLimits {
    fn default() -> Self {
        Self {
            maximum_entities: 4_096,
            maximum_features: 32,
            maximum_relation_candidates: 256,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct LearnedPredicateGroupingMerge {
    pub relation: Definition,
    pub merge: LearnedGroupMerge,
    pub relation_candidates_evaluated: usize,
}

impl LearnedPredicateGroupingMerge {
    /// Canonical identity over executable relation and merge definitions.
    ///
    /// # Errors
    ///
    /// Returns canonical encoding failure.
    pub fn semantic_hash(&self) -> Result<Hash32, cairn_core::CanonicalError> {
        CanonicalValue::Array(vec![
            CanonicalValue::Text("cairn.predicate-grouping-merge.v1".to_owned()),
            CanonicalValue::Bytes(self.relation.encode_canonical()?),
            CanonicalValue::Bytes(
                self.merge
                    .constructor
                    .proposal
                    .definition
                    .encode_canonical()?,
            ),
        ])
        .encode_canonical()
        .map(|bytes| Hash32::digest(&bytes))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PredicateGroupingError {
    InsufficientEpisodes,
    DuplicateEpisode,
    InvalidEntities,
    FeatureMismatch,
    Capacity,
    NoExactTheory,
    Ambiguous,
    RelationExecution,
    Merge(GroupMergeError),
}

#[derive(Clone, Copy, Debug, Default)]
pub struct PredicateGroupingInventor;

impl PredicateGroupingInventor {
    /// Jointly invent an executable pairwise equality relation, its connected
    /// collections, and an aggregate explaining complete unordered outputs.
    ///
    /// No pair labels, relation edges, partitions, or selected feature are
    /// supplied. Enumeration is bounded to the typed pairwise-relation hole.
    ///
    /// # Errors
    ///
    /// Rejects malformed/capped evidence, no exact theory, or equal-cost
    /// semantic ambiguity.
    pub fn invent(
        &self,
        traces: &[AttributedGroupingTrace],
        limits: PredicateGroupingLimits,
        invention_limits: InventionLimits,
    ) -> Result<LearnedPredicateGroupingMerge, PredicateGroupingError> {
        let feature_count = validate_traces(traces, limits)?;
        let candidate_count = feature_count.min(limits.maximum_relation_candidates);
        if candidate_count < feature_count {
            return Err(PredicateGroupingError::Capacity);
        }
        let mut exact = Vec::new();
        for feature in 0..feature_count {
            let definition = equality_definition(feature, feature_count)?;
            let merge_traces = traces
                .iter()
                .map(|trace| {
                    Ok(GroupMergeTrace {
                        episode: trace.episode,
                        groups: execute_groups(&definition, &trace.entities)?,
                        outputs: trace.outputs.clone(),
                    })
                })
                .collect::<Result<Vec<_>, PredicateGroupingError>>()?;
            if let Ok(merge) = GroupMergeInventor.invent(&merge_traces, invention_limits) {
                let cost = merge.constructor.proposal.structural_cost.saturating_add(3);
                exact.push((cost, definition, merge));
            }
        }
        exact.sort_by(|left, right| {
            left.0.cmp(&right.0).then_with(|| {
                let left = left.1.encode_canonical().unwrap_or_default();
                let right = right.1.encode_canonical().unwrap_or_default();
                left.cmp(&right)
            })
        });
        if exact.len() > 1 && exact[0].0 == exact[1].0 {
            return Err(PredicateGroupingError::Ambiguous);
        }
        let Some((_, relation, merge)) = exact.into_iter().next() else {
            return Err(PredicateGroupingError::NoExactTheory);
        };
        Ok(LearnedPredicateGroupingMerge {
            relation,
            merge,
            relation_candidates_evaluated: candidate_count,
        })
    }

    /// Execute a learned pairwise relation, form components, and aggregate.
    ///
    /// # Errors
    ///
    /// Returns malformed evidence, relation execution, or merge failure.
    pub fn predict(
        &self,
        learned: &LearnedPredicateGroupingMerge,
        entities: &[AttributedGroupingEntity],
        limits: PredicateGroupingLimits,
    ) -> Result<BTreeSet<i64>, PredicateGroupingError> {
        validate_entities(entities, limits, learned.relation.parameters.len() / 2)?;
        let groups = execute_groups(&learned.relation, entities)?;
        GroupMergeInventor
            .merge(&learned.merge, &groups)
            .map_err(PredicateGroupingError::Merge)
    }
}

fn validate_traces(
    traces: &[AttributedGroupingTrace],
    limits: PredicateGroupingLimits,
) -> Result<usize, PredicateGroupingError> {
    if traces.len() < 2 {
        return Err(PredicateGroupingError::InsufficientEpisodes);
    }
    let episodes = traces
        .iter()
        .map(|trace| trace.episode)
        .collect::<BTreeSet<_>>();
    if episodes.len() != traces.len() {
        return Err(PredicateGroupingError::DuplicateEpisode);
    }
    let feature_count = traces
        .first()
        .and_then(|trace| trace.entities.first())
        .map(|entity| entity.features.len())
        .ok_or(PredicateGroupingError::InvalidEntities)?;
    for trace in traces {
        validate_entities(&trace.entities, limits, feature_count)?;
    }
    Ok(feature_count)
}

fn validate_entities(
    entities: &[AttributedGroupingEntity],
    limits: PredicateGroupingLimits,
    feature_count: usize,
) -> Result<(), PredicateGroupingError> {
    if entities.len() < 2 || feature_count == 0 {
        return Err(PredicateGroupingError::InvalidEntities);
    }
    if entities.len() > limits.maximum_entities || feature_count > limits.maximum_features {
        return Err(PredicateGroupingError::Capacity);
    }
    let ids = entities
        .iter()
        .map(|entity| &entity.id)
        .collect::<BTreeSet<_>>();
    if ids.len() != entities.len() {
        return Err(PredicateGroupingError::InvalidEntities);
    }
    if entities
        .iter()
        .any(|entity| entity.features.len() != feature_count)
    {
        return Err(PredicateGroupingError::FeatureMismatch);
    }
    Ok(())
}

fn equality_definition(
    selected: usize,
    feature_count: usize,
) -> Result<Definition, PredicateGroupingError> {
    let parameters = (0..feature_count)
        .map(|feature| Binder {
            name: VarName(format!("left_feature_{feature}")),
            ty: integer_type(),
        })
        .chain((0..feature_count).map(|feature| Binder {
            name: VarName(format!("right_feature_{feature}")),
            ty: integer_type(),
        }))
        .collect::<Vec<_>>();
    let definition = Definition {
        name: "invented_pair_relation".to_owned(),
        parameters,
        return_type: Type::Bool,
        body: Expr::Binary {
            op: BinaryOp::Equal,
            left: Box::new(Expr::Var(VarName(format!("left_feature_{selected}")))),
            right: Box::new(Expr::Var(VarName(format!("right_feature_{selected}")))),
        },
        recursion: None,
        proof_annotations: Vec::new(),
    };
    TypeChecker::new(10_000)
        .check_definition(&definition)
        .map_err(|_| PredicateGroupingError::RelationExecution)?;
    Ok(definition)
}

fn execute_groups(
    definition: &Definition,
    entities: &[AttributedGroupingEntity],
) -> Result<Vec<BTreeSet<i64>>, PredicateGroupingError> {
    let mut type_environment = TypeEnvironment::new();
    for parameter in &definition.parameters {
        type_environment.insert(parameter.name.clone(), parameter.ty.clone());
    }
    let typed_expression = TypeChecker::new(10_000)
        .check(&definition.body, &type_environment)
        .map_err(|_| PredicateGroupingError::RelationExecution)?;
    let executor = ReferenceExecutor::without_definitions(ExecLimits::default(), TraceLevel::Off);
    let feature_count = definition.parameters.len() / 2;
    let mut adjacency = (0..entities.len())
        .map(|index| (index, BTreeSet::new()))
        .collect::<BTreeMap<_, _>>();
    for left in 0..entities.len() {
        for right in (left + 1)..entities.len() {
            let mut environment = RuntimeEnvironment::new();
            for feature in 0..feature_count {
                environment.insert(
                    VarName(format!("left_feature_{feature}")),
                    Value::Int(entities[left].features[feature]),
                );
                environment.insert(
                    VarName(format!("right_feature_{feature}")),
                    Value::Int(entities[right].features[feature]),
                );
            }
            let Value::Bool(related) = executor
                .execute(&typed_expression, &environment)
                .map_err(|_| PredicateGroupingError::RelationExecution)?
                .result
            else {
                return Err(PredicateGroupingError::RelationExecution);
            };
            if related {
                adjacency
                    .get_mut(&left)
                    .ok_or(PredicateGroupingError::RelationExecution)?
                    .insert(right);
                adjacency
                    .get_mut(&right)
                    .ok_or(PredicateGroupingError::RelationExecution)?
                    .insert(left);
            }
        }
    }
    let mut unseen = (0..entities.len()).collect::<BTreeSet<_>>();
    let mut groups = Vec::new();
    while let Some(start) = unseen.pop_first() {
        let mut frontier = BTreeSet::from([start]);
        let mut values = BTreeSet::new();
        while let Some(entity) = frontier.pop_first() {
            values.insert(entities[entity].value);
            frontier.extend(
                adjacency[&entity]
                    .iter()
                    .filter(|neighbor| unseen.remove(*neighbor))
                    .copied(),
            );
        }
        groups.push(values);
    }
    Ok(groups)
}

fn integer_type() -> Type {
    Type::Int(IntRange {
        lower: -1_000_000,
        upper: 1_000_000,
    })
}
