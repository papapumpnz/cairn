use crate::{
    AttributedGroupingEntity, AttributedGroupingTrace, LearnedPredicateGroupingMerge,
    PredicateGroupingError, PredicateGroupingInventor, PredicateGroupingLimits,
};
use cairn_core::{CanonicalEncode, CanonicalValue, EntityId, Hash32};
use cairn_invent::InventionLimits;
use cairn_rkl::{
    BinaryOp, Binder, BoundChecker, Definition, ExecLimits, Expr, IntRange, ReferenceExecutor,
    RuntimeEnvironment, TraceLevel, Type, TypeChecker, TypeEnvironment, UnaryOp, Value, VarName,
};
use std::collections::{BTreeMap, BTreeSet};

/// One generic primitive carrying a bounded unordered collection of scalar
/// observations. Neither collection order nor derived relational features are
/// semantic.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PrimitiveObservationEntity {
    pub id: EntityId,
    pub value: i64,
    pub observations: Vec<i64>,
}

/// Backward-compatible name for the former arity-two evidence carrier.
pub type PrimitivePairEntity = PrimitiveObservationEntity;

/// One episode of primitive observations and complete unordered target
/// evidence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PrimitiveObservationTrace {
    pub episode: Hash32,
    pub entities: Vec<PrimitiveObservationEntity>,
    pub outputs: BTreeSet<i64>,
}

/// Backward-compatible name for the former arity-two trace carrier.
pub type PrimitivePairTrace = PrimitiveObservationTrace;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ObservationPredicateLimits {
    pub maximum_entities: usize,
    /// Maximum scalar observations accepted for any entity.
    pub maximum_observations: usize,
    /// Maximum observation-index permutations checked for invariance.
    pub maximum_observation_permutations: usize,
    /// Maximum number of jointly exposed learned scalar features.
    pub maximum_features: usize,
    /// Maximum structural cost of one typed feature expression.
    pub maximum_feature_cost: u16,
    /// Maximum typed expressions checked before a capacity terminal.
    pub maximum_feature_candidates: u64,
    /// Maximum feature collections tried with the downstream learner.
    pub maximum_feature_sets: usize,
}

impl Default for ObservationPredicateLimits {
    fn default() -> Self {
        Self {
            maximum_entities: 4_096,
            maximum_observations: 4,
            maximum_observation_permutations: 24,
            maximum_features: 2,
            maximum_feature_cost: 5,
            maximum_feature_candidates: 10_000,
            maximum_feature_sets: 10_000,
        }
    }
}

/// Learned observation-to-feature definitions composed with an executable
/// predicate/aggregate mechanism.
#[derive(Clone, Debug, PartialEq)]
pub struct LearnedObservationPredicateGrouping {
    pub observation_arity: usize,
    pub feature_definitions: Vec<Definition>,
    pub grouping: LearnedPredicateGroupingMerge,
    pub feature_candidates_evaluated: u64,
    pub feature_sets_evaluated: usize,
}

impl LearnedObservationPredicateGrouping {
    /// Identity over both derived-feature RKL bodies and the downstream learned
    /// relation/aggregate.
    ///
    /// # Errors
    ///
    /// Returns canonical encoding failure.
    pub fn semantic_hash(&self) -> Result<Hash32, cairn_core::CanonicalError> {
        CanonicalValue::Array(vec![
            CanonicalValue::Text("cairn.observation-predicate-grouping.v3".to_owned()),
            CanonicalValue::Unsigned(
                u64::try_from(self.observation_arity)
                    .map_err(|_| cairn_core::CanonicalError::IntegerOutOfRange)?,
            ),
            CanonicalValue::Array(
                self.feature_definitions
                    .iter()
                    .map(|definition| definition.encode_canonical().map(CanonicalValue::Bytes))
                    .collect::<Result<Vec<_>, _>>()?,
            ),
            CanonicalValue::Bytes(self.grouping.semantic_hash()?.as_bytes().to_vec()),
        ])
        .encode_canonical()
        .map(|bytes| Hash32::digest(&bytes))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ObservationPredicateError {
    InsufficientEpisodes,
    DuplicateEpisode,
    InvalidEntities,
    Capacity,
    FeatureExecution,
    NoExactTheory,
    Ambiguous,
    Grouping(PredicateGroupingError),
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ObservationPredicateInventor;

impl ObservationPredicateInventor {
    /// Jointly select executable observation-feature definitions and invent the
    /// downstream pair predicate/aggregate from complete outputs.
    ///
    /// Feature programs are enumerated by structural cost from the universal
    /// integer RKL substrate. The learner jointly chooses one or more programs
    /// and an executable predicate/aggregate explanation. No feature template,
    /// interval, endpoint, containment, edge, or pair label is supplied.
    ///
    /// # Errors
    ///
    /// Rejects malformed/capped evidence, no exact explanation, or semantic
    /// ambiguity.
    pub fn invent(
        &self,
        traces: &[PrimitivePairTrace],
        limits: ObservationPredicateLimits,
        mut predicate_limits: PredicateGroupingLimits,
        invention_limits: InventionLimits,
    ) -> Result<LearnedObservationPredicateGrouping, ObservationPredicateError> {
        validate_primitive_traces(traces, limits)?;
        let observation_arity = traces[0].entities[0].observations.len();
        predicate_limits.include_order_comparisons = true;
        let search = feature_programs(traces, limits, invention_limits)?;
        let feature_sets = feature_sets(&search.programs, limits)?;
        let mut exact = BTreeMap::new();
        let mut exact_cost = None;
        let mut feature_sets_evaluated = 0_usize;
        for feature_set in feature_sets {
            if exact_cost.is_some_and(|cost| feature_set.cost > cost) {
                break;
            }
            feature_sets_evaluated = feature_sets_evaluated.saturating_add(1);
            let definitions =
                feature_definitions(&search.programs, &feature_set.indices, observation_arity)?;
            let attributed = lower_traces(traces, &definitions)?;
            if let Ok(grouping) =
                PredicateGroupingInventor.invent(&attributed, predicate_limits, invention_limits)
            {
                let learned = LearnedObservationPredicateGrouping {
                    observation_arity,
                    feature_definitions: definitions,
                    grouping,
                    feature_candidates_evaluated: search.candidates_evaluated,
                    feature_sets_evaluated,
                };
                let identity = learned
                    .semantic_hash()
                    .map_err(|_| ObservationPredicateError::FeatureExecution)?;
                exact_cost = Some(feature_set.cost);
                exact.entry(identity).or_insert(learned);
            }
        }
        if exact.len() > 1 {
            return Err(ObservationPredicateError::Ambiguous);
        }
        let Some((_, mut learned)) = exact.pop_first() else {
            return Err(ObservationPredicateError::NoExactTheory);
        };
        learned.feature_sets_evaluated = feature_sets_evaluated;
        Ok(learned)
    }

    /// Derive current features through learned RKL and execute the learned
    /// relation/aggregate as an untrusted prediction.
    ///
    /// # Errors
    ///
    /// Returns malformed/capped evidence, feature execution, or grouping
    /// execution failure.
    pub fn predict(
        &self,
        learned: &LearnedObservationPredicateGrouping,
        entities: &[PrimitivePairEntity],
        limits: ObservationPredicateLimits,
        predicate_limits: PredicateGroupingLimits,
    ) -> Result<BTreeSet<i64>, ObservationPredicateError> {
        validate_primitive_entities(entities, limits)?;
        let attributed = lower_entities(entities, &learned.feature_definitions)?;
        PredicateGroupingInventor
            .predict(&learned.grouping, &attributed, predicate_limits)
            .map_err(ObservationPredicateError::Grouping)
    }
}

fn validate_primitive_traces(
    traces: &[PrimitivePairTrace],
    limits: ObservationPredicateLimits,
) -> Result<(), ObservationPredicateError> {
    if traces.len() < 2 {
        return Err(ObservationPredicateError::InsufficientEpisodes);
    }
    let episodes = traces
        .iter()
        .map(|trace| trace.episode)
        .collect::<BTreeSet<_>>();
    if episodes.len() != traces.len() {
        return Err(ObservationPredicateError::DuplicateEpisode);
    }
    let mut arity = None;
    for trace in traces {
        validate_primitive_entities(&trace.entities, limits)?;
        let trace_arity = trace.entities[0].observations.len();
        if arity
            .replace(trace_arity)
            .is_some_and(|prior| prior != trace_arity)
        {
            return Err(ObservationPredicateError::InvalidEntities);
        }
    }
    Ok(())
}

fn validate_primitive_entities(
    entities: &[PrimitivePairEntity],
    limits: ObservationPredicateLimits,
) -> Result<(), ObservationPredicateError> {
    if entities.len() < 2 {
        return Err(ObservationPredicateError::InvalidEntities);
    }
    if entities.len() > limits.maximum_entities {
        return Err(ObservationPredicateError::Capacity);
    }
    let arity = entities[0].observations.len();
    if arity < 2 {
        return Err(ObservationPredicateError::InvalidEntities);
    }
    if arity > limits.maximum_observations {
        return Err(ObservationPredicateError::Capacity);
    }
    if entities
        .iter()
        .any(|entity| entity.observations.len() != arity)
    {
        return Err(ObservationPredicateError::InvalidEntities);
    }
    let ids = entities
        .iter()
        .map(|entity| &entity.id)
        .collect::<BTreeSet<_>>();
    if ids.len() != entities.len() {
        return Err(ObservationPredicateError::InvalidEntities);
    }
    Ok(())
}

fn lower_traces(
    traces: &[PrimitivePairTrace],
    definitions: &[Definition],
) -> Result<Vec<AttributedGroupingTrace>, ObservationPredicateError> {
    traces
        .iter()
        .map(|trace| {
            Ok(AttributedGroupingTrace {
                episode: trace.episode,
                entities: lower_entities(&trace.entities, definitions)?,
                outputs: trace.outputs.clone(),
            })
        })
        .collect()
}

fn lower_entities(
    entities: &[PrimitivePairEntity],
    definitions: &[Definition],
) -> Result<Vec<AttributedGroupingEntity>, ObservationPredicateError> {
    let checked = definitions
        .iter()
        .map(|definition| {
            let mut environment = TypeEnvironment::new();
            for parameter in &definition.parameters {
                environment.insert(parameter.name.clone(), parameter.ty.clone());
            }
            TypeChecker::new(10_000)
                .check(&definition.body, &environment)
                .map_err(|_| ObservationPredicateError::FeatureExecution)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let executor = ReferenceExecutor::without_definitions(ExecLimits::default(), TraceLevel::Off);
    entities
        .iter()
        .map(|entity| {
            if definitions
                .iter()
                .any(|definition| definition.parameters.len() != entity.observations.len())
            {
                return Err(ObservationPredicateError::FeatureExecution);
            }
            let mut environment = RuntimeEnvironment::new();
            for (index, observation) in entity.observations.iter().copied().enumerate() {
                environment.insert(
                    VarName(format!("observation_{index}")),
                    Value::Int(observation),
                );
            }
            let features = checked
                .iter()
                .map(|expression| {
                    match executor
                        .execute(expression, &environment)
                        .map_err(|_| ObservationPredicateError::FeatureExecution)?
                        .result
                    {
                        Value::Int(value) => Ok(value),
                        _ => Err(ObservationPredicateError::FeatureExecution),
                    }
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(AttributedGroupingEntity {
                id: entity.id.clone(),
                value: entity.value,
                features,
            })
        })
        .collect()
}

#[derive(Clone, Debug)]
struct FeatureProgram {
    expression: Expr,
    return_type: Type,
    cost: u16,
    permuted_behaviors: Vec<Vec<i64>>,
    canonical: Vec<u8>,
}

#[derive(Clone, Debug)]
struct FeatureProgramSearch {
    programs: Vec<FeatureProgram>,
    candidates_evaluated: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct FeatureSet {
    cost: u16,
    indices: Vec<usize>,
}

#[allow(clippy::too_many_lines)] // Keeps one deterministic cost-ordered grammar traversal auditable.
fn feature_programs(
    traces: &[PrimitivePairTrace],
    limits: ObservationPredicateLimits,
    invention_limits: InventionLimits,
) -> Result<FeatureProgramSearch, ObservationPredicateError> {
    validate_feature_limits(limits)?;
    let observation_arity = traces[0].entities[0].observations.len();
    let permutations =
        observation_permutations(observation_arity, limits.maximum_observation_permutations)?;
    let environments = permutations
        .iter()
        .map(|permutation| observation_environments(traces, permutation))
        .collect::<Vec<_>>();
    let mut type_environment = TypeEnvironment::new();
    for parameter in feature_parameters(observation_arity) {
        type_environment.insert(parameter.name, parameter.ty);
    }
    let checker = TypeChecker::new(invention_limits.maximum_expression_nodes);
    let executor =
        ReferenceExecutor::without_definitions(invention_limits.execution, TraceLevel::Off);
    let mut by_cost = vec![Vec::new(); usize::from(limits.maximum_feature_cost) + 1];
    let mut behaviors = BTreeSet::new();
    let mut candidates_evaluated = 0_u64;
    for expression in
        (0..observation_arity).map(|index| Expr::Var(VarName(format!("observation_{index}"))))
    {
        if let Some(candidate) = consider_feature_program(
            expression,
            1,
            &checker,
            &type_environment,
            &executor,
            &environments,
            observation_arity,
            limits,
            &mut candidates_evaluated,
            &mut behaviors,
        )? {
            by_cost[1].push(candidate);
        }
    }
    for cost in 2..=limits.maximum_feature_cost {
        let prior = by_cost[usize::from(cost - 1)].clone();
        for candidate in prior {
            if let Some(candidate) = consider_feature_program(
                Expr::Unary {
                    op: UnaryOp::Negate,
                    argument: Box::new(candidate.expression),
                },
                cost,
                &checker,
                &type_environment,
                &executor,
                &environments,
                observation_arity,
                limits,
                &mut candidates_evaluated,
                &mut behaviors,
            )? {
                by_cost[usize::from(cost)].push(candidate);
            }
        }
        for left_cost in 1..cost.saturating_sub(1) {
            let right_cost = cost - 1 - left_cost;
            let left = by_cost[usize::from(left_cost)].clone();
            let right = by_cost[usize::from(right_cost)].clone();
            for left in &left {
                for right in &right {
                    for operation in [
                        BinaryOp::Minimum,
                        BinaryOp::Maximum,
                        BinaryOp::Add,
                        BinaryOp::Subtract,
                        BinaryOp::Multiply,
                    ] {
                        if operation.is_commutative() && left.canonical > right.canonical {
                            continue;
                        }
                        if let Some(candidate) = consider_feature_program(
                            Expr::Binary {
                                op: operation,
                                left: Box::new(left.expression.clone()),
                                right: Box::new(right.expression.clone()),
                            },
                            cost,
                            &checker,
                            &type_environment,
                            &executor,
                            &environments,
                            observation_arity,
                            limits,
                            &mut candidates_evaluated,
                            &mut behaviors,
                        )? {
                            by_cost[usize::from(cost)].push(candidate);
                        }
                    }
                }
            }
        }
    }
    let mut programs = by_cost.into_iter().flatten().collect::<Vec<_>>();
    sort_feature_programs(&mut programs);
    Ok(FeatureProgramSearch {
        programs,
        candidates_evaluated,
    })
}

fn sort_feature_programs(programs: &mut [FeatureProgram]) {
    programs.sort_by(|left, right| {
        left.cost
            .cmp(&right.cost)
            .then_with(|| left.canonical.cmp(&right.canonical))
    });
}

fn validate_feature_limits(
    limits: ObservationPredicateLimits,
) -> Result<(), ObservationPredicateError> {
    if limits.maximum_features == 0
        || limits.maximum_observations < 2
        || limits.maximum_observation_permutations == 0
        || limits.maximum_feature_cost == 0
        || limits.maximum_feature_candidates == 0
        || limits.maximum_feature_sets == 0
    {
        return Err(ObservationPredicateError::Capacity);
    }
    Ok(())
}

fn observation_environments(
    traces: &[PrimitivePairTrace],
    permutation: &[usize],
) -> Vec<RuntimeEnvironment> {
    traces
        .iter()
        .flat_map(|trace| trace.entities.iter())
        .map(|entity| {
            let mut environment = RuntimeEnvironment::new();
            for (target, source) in permutation.iter().copied().enumerate() {
                environment.insert(
                    VarName(format!("observation_{target}")),
                    Value::Int(entity.observations[source]),
                );
            }
            environment
        })
        .collect()
}

fn observation_permutations(
    arity: usize,
    maximum: usize,
) -> Result<Vec<Vec<usize>>, ObservationPredicateError> {
    fn extend(
        arity: usize,
        maximum: usize,
        current: &mut Vec<usize>,
        output: &mut Vec<Vec<usize>>,
    ) -> Result<(), ObservationPredicateError> {
        if current.len() == arity {
            if output.len() >= maximum {
                return Err(ObservationPredicateError::Capacity);
            }
            output.push(current.clone());
            return Ok(());
        }
        for index in 0..arity {
            if current.contains(&index) {
                continue;
            }
            current.push(index);
            extend(arity, maximum, current, output)?;
            current.pop();
        }
        Ok(())
    }
    let mut output = Vec::new();
    extend(arity, maximum, &mut Vec::new(), &mut output)?;
    Ok(output)
}

#[allow(clippy::too_many_arguments)]
fn consider_feature_program(
    expression: Expr,
    cost: u16,
    checker: &TypeChecker,
    type_environment: &TypeEnvironment,
    executor: &ReferenceExecutor,
    environments: &[Vec<RuntimeEnvironment>],
    observation_arity: usize,
    limits: ObservationPredicateLimits,
    candidates_evaluated: &mut u64,
    behaviors: &mut BTreeSet<Vec<Vec<i64>>>,
) -> Result<Option<FeatureProgram>, ObservationPredicateError> {
    *candidates_evaluated = candidates_evaluated.saturating_add(1);
    if *candidates_evaluated > limits.maximum_feature_candidates {
        return Err(ObservationPredicateError::Capacity);
    }
    let Ok(typed) = checker.check(&expression, type_environment) else {
        return Ok(None);
    };
    if !matches!(typed.ty(), Type::Int(_)) {
        return Ok(None);
    }
    let permuted_behaviors = environments
        .iter()
        .map(|permutation| {
            permutation
                .iter()
                .map(|environment| match executor.execute(&typed, environment) {
                    Ok(trace) => match trace.result {
                        Value::Int(value) => Ok(value),
                        _ => Err(ObservationPredicateError::FeatureExecution),
                    },
                    Err(_) => Err(ObservationPredicateError::FeatureExecution),
                })
                .collect::<Result<Vec<_>, _>>()
        })
        .collect::<Result<Vec<_>, _>>()?;
    if !behaviors.insert(permuted_behaviors.clone()) {
        return Ok(None);
    }
    let canonical = Definition {
        name: "feature_candidate".to_owned(),
        parameters: feature_parameters(observation_arity),
        return_type: typed.ty().clone(),
        body: expression.clone(),
        recursion: None,
        proof_annotations: Vec::new(),
    }
    .encode_canonical()
    .map_err(|_| ObservationPredicateError::FeatureExecution)?;
    Ok(Some(FeatureProgram {
        expression,
        return_type: typed.ty().clone(),
        cost,
        permuted_behaviors,
        canonical,
    }))
}

fn feature_sets(
    programs: &[FeatureProgram],
    limits: ObservationPredicateLimits,
) -> Result<Vec<FeatureSet>, ObservationPredicateError> {
    let mut sets = Vec::new();
    for width in 1..=limits.maximum_features.min(programs.len()) {
        enumerate_feature_sets(
            programs,
            width,
            0,
            &mut Vec::new(),
            &mut sets,
            limits.maximum_feature_sets,
        )?;
    }
    sets.sort_by(|left, right| {
        left.cost
            .cmp(&right.cost)
            .then_with(|| left.indices.cmp(&right.indices))
    });
    Ok(sets)
}

fn enumerate_feature_sets(
    programs: &[FeatureProgram],
    width: usize,
    start: usize,
    current: &mut Vec<usize>,
    output: &mut Vec<FeatureSet>,
    maximum_sets: usize,
) -> Result<(), ObservationPredicateError> {
    if current.len() == width {
        if output.len() >= maximum_sets {
            return Err(ObservationPredicateError::Capacity);
        }
        if !feature_set_is_unordered(programs, current) {
            return Ok(());
        }
        let cost = current.iter().try_fold(0_u16, |total, index| {
            total.checked_add(programs[*index].cost)
        });
        output.push(FeatureSet {
            cost: cost.ok_or(ObservationPredicateError::Capacity)?,
            indices: current.clone(),
        });
        return Ok(());
    }
    let remaining = width - current.len();
    let Some(last_start) = programs.len().checked_sub(remaining) else {
        return Ok(());
    };
    for index in start..=last_start {
        current.push(index);
        enumerate_feature_sets(programs, width, index + 1, current, output, maximum_sets)?;
        current.pop();
    }
    Ok(())
}

fn feature_set_is_unordered(programs: &[FeatureProgram], selected: &[usize]) -> bool {
    let Some(permutation_count) = selected
        .first()
        .map(|index| programs[*index].permuted_behaviors.len())
    else {
        return false;
    };
    let mut ordinary = selected
        .iter()
        .map(|index| programs[*index].permuted_behaviors[0].clone())
        .collect::<Vec<_>>();
    ordinary.sort();
    (1..permutation_count).all(|permutation| {
        let mut permuted = selected
            .iter()
            .map(|index| programs[*index].permuted_behaviors[permutation].clone())
            .collect::<Vec<_>>();
        permuted.sort();
        ordinary == permuted
    })
}

fn feature_definitions(
    programs: &[FeatureProgram],
    selected: &[usize],
    observation_arity: usize,
) -> Result<Vec<Definition>, ObservationPredicateError> {
    selected
        .iter()
        .enumerate()
        .map(|(feature, index)| {
            let program = programs
                .get(*index)
                .ok_or(ObservationPredicateError::Capacity)?;
            debug_assert!(program
                .permuted_behaviors
                .first()
                .is_some_and(|behavior| !behavior.is_empty()));
            let definition = Definition {
                name: format!("invented_observation_feature_{feature}"),
                parameters: feature_parameters(observation_arity),
                return_type: program.return_type.clone(),
                body: program.expression.clone(),
                recursion: None,
                proof_annotations: Vec::new(),
            };
            TypeChecker::new(10_000)
                .check_definition(&definition)
                .map_err(|_| ObservationPredicateError::FeatureExecution)?;
            Ok(definition)
        })
        .collect()
}

fn feature_parameters(observation_arity: usize) -> Vec<Binder> {
    (0..observation_arity)
        .map(|index| Binder {
            name: VarName(format!("observation_{index}")),
            ty: integer_type(),
        })
        .collect()
}

fn integer_type() -> Type {
    Type::Int(IntRange {
        lower: -1_000_000,
        upper: 1_000_000,
    })
}
