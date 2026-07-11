//! Environment-neutral induction of theory components from binding evidence.

use cairn_invent::{
    BindingExample, DefinitionInventor, InventionError, InventionLimits, InventionOutcome,
    InventionProblem,
};
use cairn_rkl::{
    BinaryOp, Binder, Definition, Expr, RuntimeEnvironment, Schedule, Type, Value, VarName,
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq)]
pub struct PredicateColumn {
    pub expression: Expr,
    pub true_rows: BTreeSet<u32>,
    pub structural_cost: u16,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BindingTable {
    pub positive_rows: BTreeSet<u32>,
    pub negative_rows: BTreeSet<u32>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GuardHypothesis {
    pub expression: Expr,
    pub predicates: Vec<usize>,
    pub structural_cost: u16,
    pub candidates_evaluated: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GuardLimits {
    pub maximum_conjuncts: usize,
    pub maximum_candidates: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GuardFailure {
    EmptyCapacity,
    InconsistentRows,
    Unrealizable,
    CandidateCapacity { maximum: u64 },
}

#[derive(Clone, Copy, Debug, Default)]
pub struct GuardInducer;

impl GuardInducer {
    /// Find a minimum-cost conjunction separating all positive and negative
    /// bindings. Predicate meanings remain supplied as generic RKL.
    ///
    /// # Errors
    ///
    /// Distinguishes malformed, capped, and unrealizable separation.
    pub fn induce(
        &self,
        table: &BindingTable,
        predicates: &[PredicateColumn],
        limits: GuardLimits,
    ) -> Result<GuardHypothesis, GuardFailure> {
        if limits.maximum_conjuncts == 0 || limits.maximum_candidates == 0 {
            return Err(GuardFailure::EmptyCapacity);
        }
        if !table.positive_rows.is_disjoint(&table.negative_rows) {
            return Err(GuardFailure::InconsistentRows);
        }
        let mut candidates = predicates
            .iter()
            .enumerate()
            .filter(|(_, predicate)| table.positive_rows.is_subset(&predicate.true_rows))
            .collect::<Vec<_>>();
        candidates.sort_by_key(|(index, predicate)| (predicate.structural_cost, *index));
        let mut evaluated = 0_u64;
        for width in 1..=limits.maximum_conjuncts.min(candidates.len()) {
            let mut selection = Vec::new();
            if let Some(indices) = search_conjunction(
                &candidates,
                &table.negative_rows,
                width,
                0,
                &mut selection,
                &mut evaluated,
                limits.maximum_candidates,
            )? {
                let expression = indices.iter().skip(1).fold(
                    predicates[indices[0]].expression.clone(),
                    |left, index| Expr::Binary {
                        op: BinaryOp::And,
                        left: Box::new(left),
                        right: Box::new(predicates[*index].expression.clone()),
                    },
                );
                let structural_cost = indices.iter().fold(0_u16, |cost, index| {
                    cost.saturating_add(predicates[*index].structural_cost)
                });
                return Ok(GuardHypothesis {
                    expression,
                    predicates: indices,
                    structural_cost,
                    candidates_evaluated: evaluated,
                });
            }
        }
        Err(GuardFailure::Unrealizable)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContrastExample {
    pub features: BTreeMap<VarName, Value>,
    pub selected: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ContrastError {
    EmptyExamples,
    MissingPositive,
    MissingNegative,
    FeatureMismatch,
    Invention(InventionError),
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ContrastPredicateInventor;

impl ContrastPredicateInventor {
    /// Invent an executable Boolean definition that separates selected from
    /// rejected bindings using only typed feature values and generic RKL.
    ///
    /// # Errors
    ///
    /// Rejects empty/one-class evidence, inconsistent feature schemas, and
    /// bounded invention failure.
    pub fn invent(
        &self,
        name: &str,
        features: &[(VarName, Type)],
        examples: &[ContrastExample],
        literals: Vec<Value>,
        limits: InventionLimits,
    ) -> Result<InventionOutcome, ContrastError> {
        if examples.is_empty() {
            return Err(ContrastError::EmptyExamples);
        }
        if !examples.iter().any(|example| example.selected) {
            return Err(ContrastError::MissingPositive);
        }
        if !examples.iter().any(|example| !example.selected) {
            return Err(ContrastError::MissingNegative);
        }
        let names = features
            .iter()
            .map(|(name, _)| name)
            .collect::<BTreeSet<_>>();
        if examples.iter().any(|example| {
            example.features.len() != features.len()
                || example.features.keys().collect::<BTreeSet<_>>() != names
        }) {
            return Err(ContrastError::FeatureMismatch);
        }
        let demonstrations = examples
            .iter()
            .map(|example| {
                let mut inputs = RuntimeEnvironment::new();
                for (name, value) in &example.features {
                    inputs.insert(name.clone(), value.clone());
                }
                BindingExample {
                    inputs,
                    output: Value::Bool(example.selected),
                }
            })
            .collect();
        DefinitionInventor
            .invent(&InventionProblem {
                definition_name: name.to_owned(),
                parameters: features
                    .iter()
                    .map(|(name, ty)| Binder {
                        name: name.clone(),
                        ty: ty.clone(),
                    })
                    .collect(),
                return_type: Type::Bool,
                literals,
                demonstrations,
                limits,
            })
            .map_err(ContrastError::Invention)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RelationExample {
    pub bindings: BTreeMap<VarName, i64>,
    pub related: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct InventedRelation {
    pub parameters: Vec<Binder>,
    pub guard: GuardHypothesis,
    pub atomic_predicates_evaluated: usize,
    pub invention: InventionOutcome,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RelationInventionError {
    EmptyExamples,
    MissingPositive,
    MissingNegative,
    FeatureMismatch,
    Capacity,
    Guard(GuardFailure),
    Verification(InventionError),
}

#[derive(Clone, Copy, Debug, Default)]
pub struct RelationalPredicateInventor;

impl RelationalPredicateInventor {
    /// Invent a conjunction of ordinary comparisons between two typed entity
    /// feature sets. No named spatial or domain relation is supplied.
    ///
    /// # Errors
    ///
    /// Rejects one-class or inconsistent bindings, row-index overflow, and a
    /// bounded guard search that cannot separate the evidence.
    #[allow(clippy::too_many_lines)] // Keep validation, atom generation, and admission together.
    pub fn invent(
        &self,
        left_features: &[(VarName, Type)],
        right_features: &[(VarName, Type)],
        examples: &[RelationExample],
        limits: GuardLimits,
    ) -> Result<InventedRelation, RelationInventionError> {
        if examples.is_empty() {
            return Err(RelationInventionError::EmptyExamples);
        }
        if !examples.iter().any(|example| example.related) {
            return Err(RelationInventionError::MissingPositive);
        }
        if !examples.iter().any(|example| !example.related) {
            return Err(RelationInventionError::MissingNegative);
        }
        let parameters = left_features
            .iter()
            .chain(right_features)
            .map(|(name, ty)| Binder {
                name: name.clone(),
                ty: ty.clone(),
            })
            .collect::<Vec<_>>();
        let names = parameters
            .iter()
            .map(|parameter| &parameter.name)
            .collect::<BTreeSet<_>>();
        if names.len() != parameters.len()
            || examples.iter().any(|example| {
                example.bindings.len() != parameters.len()
                    || example.bindings.keys().collect::<BTreeSet<_>>() != names
            })
        {
            return Err(RelationInventionError::FeatureMismatch);
        }
        let table = BindingTable {
            positive_rows: labeled_rows(examples, true)?,
            negative_rows: labeled_rows(examples, false)?,
        };
        let mut predicates = Vec::new();
        for (left_name, _) in left_features {
            for (right_name, _) in right_features {
                for (operation, reversed) in [
                    (BinaryOp::LessThan, false),
                    (BinaryOp::LessThan, true),
                    (BinaryOp::Equal, false),
                ] {
                    let (first, second) = if reversed {
                        (right_name, left_name)
                    } else {
                        (left_name, right_name)
                    };
                    let true_rows = examples
                        .iter()
                        .enumerate()
                        .filter_map(|(index, example)| {
                            let left = example.bindings[first];
                            let right = example.bindings[second];
                            let value = match operation {
                                BinaryOp::LessThan => left < right,
                                BinaryOp::Equal => left == right,
                                _ => false,
                            };
                            value.then(|| u32::try_from(index).ok()).flatten()
                        })
                        .collect();
                    predicates.push(PredicateColumn {
                        expression: Expr::Binary {
                            op: operation,
                            left: Box::new(Expr::Var(first.clone())),
                            right: Box::new(Expr::Var(second.clone())),
                        },
                        true_rows,
                        structural_cost: 3,
                    });
                }
            }
        }
        let guard = GuardInducer
            .induce(&table, &predicates, limits)
            .map_err(RelationInventionError::Guard)?;
        let demonstrations = examples
            .iter()
            .map(|example| {
                let mut inputs = RuntimeEnvironment::new();
                for (name, value) in &example.bindings {
                    inputs.insert(name.clone(), Value::Int(*value));
                }
                BindingExample {
                    inputs,
                    output: Value::Bool(example.related),
                }
            })
            .collect();
        let problem = InventionProblem {
            definition_name: "invented_relation".to_owned(),
            parameters: parameters.clone(),
            return_type: Type::Bool,
            literals: Vec::new(),
            demonstrations,
            limits: InventionLimits::default(),
        };
        let invention = DefinitionInventor
            .admit_definition(
                &problem,
                Definition {
                    name: problem.definition_name.clone(),
                    parameters: parameters.clone(),
                    return_type: Type::Bool,
                    body: guard.expression.clone(),
                    recursion: None,
                    proof_annotations: Vec::new(),
                },
                guard.structural_cost,
                u64::try_from(predicates.len()).map_err(|_| RelationInventionError::Capacity)?,
            )
            .map_err(RelationInventionError::Verification)?;
        Ok(InventedRelation {
            parameters,
            guard,
            atomic_predicates_evaluated: predicates.len(),
            invention,
        })
    }
}

fn labeled_rows(
    examples: &[RelationExample],
    label: bool,
) -> Result<BTreeSet<u32>, RelationInventionError> {
    examples
        .iter()
        .enumerate()
        .filter(|(_, example)| example.related == label)
        .map(|(index, _)| u32::try_from(index).map_err(|_| RelationInventionError::Capacity))
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn search_conjunction(
    candidates: &[(usize, &PredicateColumn)],
    negatives: &BTreeSet<u32>,
    width: usize,
    start: usize,
    selection: &mut Vec<usize>,
    evaluated: &mut u64,
    maximum: u64,
) -> Result<Option<Vec<usize>>, GuardFailure> {
    if selection.len() == width {
        *evaluated = evaluated.saturating_add(1);
        if *evaluated > maximum {
            return Err(GuardFailure::CandidateCapacity { maximum });
        }
        let survivors = selection.iter().fold(negatives.clone(), |rows, candidate| {
            rows.intersection(&candidates[*candidate].1.true_rows)
                .copied()
                .collect()
        });
        return Ok(survivors.is_empty().then(|| {
            selection
                .iter()
                .map(|candidate| candidates[*candidate].0)
                .collect()
        }));
    }
    for candidate in start..candidates.len() {
        selection.push(candidate);
        if let Some(result) = search_conjunction(
            candidates,
            negatives,
            width,
            candidate + 1,
            selection,
            evaluated,
            maximum,
        )? {
            return Ok(Some(result));
        }
        selection.pop();
    }
    Ok(None)
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ConstructorSynthesizer;

impl ConstructorSynthesizer {
    /// Synthesize a checked variable-valued constructor from demonstrations.
    ///
    /// # Errors
    ///
    /// Returns the exact bounded invention failure.
    pub fn synthesize(
        &self,
        problem: &InventionProblem,
    ) -> Result<InventionOutcome, InventionError> {
        DefinitionInventor.invent(problem)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IterativeTrace {
    pub episode: cairn_core::Hash32,
    pub initial: i64,
    pub parameter: i64,
    pub states: Vec<i64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LearnedIterativeMechanism {
    pub step: InventionOutcome,
    pub mechanism: InventionOutcome,
    pub source_episodes: BTreeSet<cairn_core::Hash32>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IterativeInventionError {
    InsufficientEpisodes,
    DuplicateEpisode,
    InvalidTrace,
    Invention(InventionError),
}

#[derive(Clone, Copy, Debug, Default)]
pub struct IterativeMechanismInventor;

impl IterativeMechanismInventor {
    /// Learn a task-local transition function from traces, then lift it into a
    /// bounded fixpoint program rather than retaining an unrolled sequence.
    ///
    /// # Errors
    ///
    /// Requires two independent, stable, nonnegative traces and returns exact
    /// bounded synthesis or cross-executor admission failures.
    #[allow(clippy::too_many_lines)] // Keep trace validation, step learning, and lifting atomic.
    pub fn invent(
        &self,
        traces: &[IterativeTrace],
        maximum_steps: u32,
        limits: InventionLimits,
    ) -> Result<LearnedIterativeMechanism, IterativeInventionError> {
        if traces.len() < 2 {
            return Err(IterativeInventionError::InsufficientEpisodes);
        }
        let source_episodes = traces
            .iter()
            .map(|trace| trace.episode)
            .collect::<BTreeSet<_>>();
        if source_episodes.len() != traces.len() {
            return Err(IterativeInventionError::DuplicateEpisode);
        }
        if maximum_steps == 0
            || traces.iter().any(|trace| {
                trace.states.len() < 2
                    || trace.states[0] != trace.initial
                    || trace.states.iter().any(|state| *state < 0)
                    || trace.parameter <= 0
                    || trace.states[trace.states.len() - 1] != trace.states[trace.states.len() - 2]
            })
        {
            return Err(IterativeInventionError::InvalidTrace);
        }
        let current = VarName("current".to_owned());
        let parameter = VarName("parameter".to_owned());
        let integer = Type::Int(cairn_rkl::IntRange {
            lower: 0,
            upper: i64::from(u16::MAX),
        });
        let step_parameters = vec![
            Binder {
                name: current.clone(),
                ty: integer.clone(),
            },
            Binder {
                name: parameter.clone(),
                ty: integer.clone(),
            },
        ];
        let step_demonstrations = traces
            .iter()
            .flat_map(|trace| {
                trace.states.windows(2).map(|transition| {
                    let mut inputs = RuntimeEnvironment::new();
                    inputs.insert(current.clone(), Value::Int(transition[0]));
                    inputs.insert(parameter.clone(), Value::Int(trace.parameter));
                    BindingExample {
                        inputs,
                        output: Value::Int(transition[1]),
                    }
                })
            })
            .collect();
        let step = DefinitionInventor
            .invent(&InventionProblem {
                definition_name: "invented_step".to_owned(),
                parameters: step_parameters,
                return_type: integer.clone(),
                literals: vec![Value::Int(0), Value::Int(1)],
                demonstrations: step_demonstrations,
                limits,
            })
            .map_err(IterativeInventionError::Invention)?;
        let initial = VarName("initial".to_owned());
        let mechanism_parameters = vec![
            Binder {
                name: initial.clone(),
                ty: integer.clone(),
            },
            Binder {
                name: parameter.clone(),
                ty: integer.clone(),
            },
        ];
        let demonstrations = traces
            .iter()
            .map(|trace| {
                let mut inputs = RuntimeEnvironment::new();
                inputs.insert(initial.clone(), Value::Int(trace.initial));
                inputs.insert(parameter.clone(), Value::Int(trace.parameter));
                BindingExample {
                    inputs,
                    output: Value::Int(trace.states[trace.states.len() - 1]),
                }
            })
            .collect();
        let problem = InventionProblem {
            definition_name: "invented_iteration".to_owned(),
            parameters: mechanism_parameters.clone(),
            return_type: integer,
            literals: Vec::new(),
            demonstrations,
            limits,
        };
        let mechanism = DefinitionInventor
            .admit_definition(
                &problem,
                Definition {
                    name: problem.definition_name.clone(),
                    parameters: mechanism_parameters,
                    return_type: problem.return_type.clone(),
                    body: Expr::BoundedFixpoint {
                        binder: current.clone(),
                        initial: Box::new(Expr::Var(initial)),
                        step: Box::new(step.proposal.definition.body.clone()),
                        maximum_steps,
                        progress_measure: Box::new(Expr::Var(current)),
                    },
                    recursion: None,
                    proof_annotations: Vec::new(),
                },
                step.proposal.structural_cost.saturating_add(3),
                step.proposal.candidates_evaluated,
            )
            .map_err(IterativeInventionError::Invention)?;
        Ok(LearnedIterativeMechanism {
            step,
            mechanism,
            source_episodes,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuleFootprint {
    pub name: String,
    pub reads: BTreeSet<Value>,
    pub writes: BTreeSet<Value>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ScheduleFailure {
    DuplicateRule(String),
    ConflictingWrites { left: String, right: String },
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ScheduleSynthesizer;

impl ScheduleSynthesizer {
    /// Infer parallel execution or a dependency DAG from generic read/write
    /// footprints; ambiguous write conflicts fail instead of choosing order.
    ///
    /// # Errors
    ///
    /// Rejects duplicate rules and unexplained write conflicts.
    pub fn synthesize(&self, rules: &[RuleFootprint]) -> Result<Schedule, ScheduleFailure> {
        let mut names = BTreeSet::new();
        for rule in rules {
            if !names.insert(rule.name.clone()) {
                return Err(ScheduleFailure::DuplicateRule(rule.name.clone()));
            }
        }
        let mut edges = BTreeSet::new();
        for (left_index, left) in rules.iter().enumerate() {
            for right in rules.iter().skip(left_index + 1) {
                if !left.writes.is_disjoint(&right.writes) {
                    return Err(ScheduleFailure::ConflictingWrites {
                        left: left.name.clone(),
                        right: right.name.clone(),
                    });
                }
                if !left.writes.is_disjoint(&right.reads) {
                    edges.insert((left.name.clone(), right.name.clone()));
                }
                if !right.writes.is_disjoint(&left.reads) {
                    edges.insert((right.name.clone(), left.name.clone()));
                }
            }
        }
        if edges.is_empty() {
            Ok(Schedule::Parallel)
        } else {
            Ok(Schedule::DependencyDag(edges.into_iter().collect()))
        }
    }
}
