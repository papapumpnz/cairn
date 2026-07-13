//! Environment-neutral induction of theory components from binding evidence.

mod conditional_construction;
mod construction_program;
mod correspondence;
mod group_merge;
mod grouping_selection;
mod layer;
mod predicate_grouping;
mod raw_grouping;

pub use conditional_construction::*;
pub use construction_program::*;
pub use correspondence::*;
pub use group_merge::*;
pub use grouping_selection::*;
pub use layer::*;
pub use predicate_grouping::*;
pub use raw_grouping::*;

use cairn_invent::{
    BindingExample, DefinitionInventor, InventionError, InventionLimits, InventionOutcome,
    InventionProblem, UnorderedMappingEpisode, UnorderedMappingProblem,
};
use cairn_rkl::{
    BinaryOp, Binder, CheckedTheory, ConflictPolicy, Definition, Effect, Expr, Quantifier,
    Renderer, RewriteRule, RuleBinder, RuntimeEnvironment, Schedule, Theory, TheoryChecker, Type,
    TypeChecker, Value, VarName,
};
use std::collections::{BTreeMap, BTreeSet};

use cairn_core::{
    CanonicalEncode, CanonicalValue, EntityId, EntitySelectionProblem, FeatureId, Hash32,
    RelationRole, RelationalEntityCollection,
};

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
pub struct CollectionTrace {
    pub episode: cairn_core::Hash32,
    pub inputs: Vec<i64>,
    pub outputs: Vec<i64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LearnedCollectionMechanism {
    pub element: InventionOutcome,
    pub theory: CheckedTheory,
    pub source_episodes: BTreeSet<cairn_core::Hash32>,
    pub observed_cardinalities: BTreeSet<usize>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LearnedExpansionMechanism {
    pub constructor: InventionOutcome,
    pub multiplicity: u32,
    pub theory: CheckedTheory,
    pub source_episodes: BTreeSet<cairn_core::Hash32>,
    pub observed_input_cardinalities: BTreeSet<usize>,
    pub observed_output_cardinalities: BTreeSet<usize>,
}

impl LearnedExpansionMechanism {
    /// Return the canonical executable expansion identity.
    ///
    /// # Errors
    ///
    /// Returns canonical encoding failure.
    pub fn semantic_hash(&self) -> Result<Hash32, cairn_core::CanonicalError> {
        CanonicalValue::Array(vec![
            CanonicalValue::Text("cairn.collection-expansion.v1".to_owned()),
            CanonicalValue::Unsigned(u64::from(self.multiplicity)),
            CanonicalValue::Bytes(self.constructor.proposal.definition.encode_canonical()?),
        ])
        .encode_canonical()
        .map(|bytes| Hash32::digest(&bytes))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CollectionInventionError {
    InsufficientEpisodes,
    DuplicateEpisode,
    InvalidTrace,
    FixedCardinalityOnly,
    Invention(InventionError),
    Theory(cairn_rkl::CheckError),
}

#[derive(Clone, Copy, Debug, Default)]
pub struct CollectionMechanismInventor;

impl CollectionMechanismInventor {
    /// Invent a bounded one-to-many constructor without supplied output/input
    /// correspondence.
    ///
    /// # Errors
    ///
    /// Rejects malformed or duplicate traces, fixed-only evidence, expansion
    /// cap exhaustion, bounded RKL invention failure, or invalid theory.
    #[allow(clippy::too_many_lines)]
    pub fn invent_expansion(
        &self,
        traces: &[CollectionTrace],
        maximum_input_cardinality: u32,
        maximum_multiplicity: u32,
        limits: InventionLimits,
    ) -> Result<LearnedExpansionMechanism, CollectionInventionError> {
        if traces.len() < 2 {
            return Err(CollectionInventionError::InsufficientEpisodes);
        }
        let source_episodes = traces
            .iter()
            .map(|trace| trace.episode)
            .collect::<BTreeSet<_>>();
        if source_episodes.len() != traces.len() {
            return Err(CollectionInventionError::DuplicateEpisode);
        }
        if maximum_input_cardinality == 0
            || maximum_multiplicity < 2
            || traces.iter().any(|trace| {
                trace.inputs.is_empty()
                    || trace.outputs.is_empty()
                    || trace.inputs.iter().copied().collect::<BTreeSet<_>>().len()
                        != trace.inputs.len()
                    || trace.outputs.iter().copied().collect::<BTreeSet<_>>().len()
                        != trace.outputs.len()
                    || trace.inputs.len() > maximum_input_cardinality as usize
            })
        {
            return Err(CollectionInventionError::InvalidTrace);
        }
        let observed_input_cardinalities = traces
            .iter()
            .map(|trace| trace.inputs.len())
            .collect::<BTreeSet<_>>();
        if observed_input_cardinalities.len() < 2 {
            return Err(CollectionInventionError::FixedCardinalityOnly);
        }
        let item = VarName("item".to_owned());
        let slot = VarName("slot".to_owned());
        let integer = Type::Int(cairn_rkl::IntRange {
            lower: -i64::from(u16::MAX),
            upper: i64::from(u16::MAX),
        });
        let mut selected = None;
        for multiplicity in 2..=maximum_multiplicity {
            if traces.iter().any(|trace| {
                trace.outputs.len() != trace.inputs.len().saturating_mul(multiplicity as usize)
            }) {
                continue;
            }
            let episodes = traces
                .iter()
                .map(|trace| {
                    let inputs = trace
                        .inputs
                        .iter()
                        .flat_map(|value| {
                            (0..multiplicity).map(|index| {
                                let mut environment = RuntimeEnvironment::new();
                                environment.insert(item.clone(), Value::Int(*value));
                                environment.insert(slot.clone(), Value::Int(i64::from(index)));
                                environment
                            })
                        })
                        .collect();
                    UnorderedMappingEpisode {
                        inputs,
                        expected_outputs: trace.outputs.iter().copied().map(Value::Int).collect(),
                    }
                })
                .collect();
            match DefinitionInventor.invent_unordered_mapping(&UnorderedMappingProblem {
                definition_name: "invented_expansion_constructor".to_owned(),
                parameters: vec![
                    Binder {
                        name: item.clone(),
                        ty: integer.clone(),
                    },
                    Binder {
                        name: slot.clone(),
                        ty: integer.clone(),
                    },
                ],
                return_type: integer.clone(),
                literals: vec![Value::Int(0), Value::Int(1), Value::Int(2)],
                episodes,
                limits,
            }) {
                Ok(outcome) => {
                    selected = Some((multiplicity, outcome));
                    break;
                }
                Err(InventionError::NoDefinition { .. }) => {}
                Err(error) => return Err(CollectionInventionError::Invention(error)),
            }
        }
        let (multiplicity, constructor) = selected.ok_or(CollectionInventionError::InvalidTrace)?;
        let collection = VarName("collection".to_owned());
        let slots = VarName("slots".to_owned());
        let rule_name = "invented_expand".to_owned();
        let theory = Theory {
            inputs: vec![
                Binder {
                    name: collection.clone(),
                    ty: Type::List {
                        element: Box::new(integer.clone()),
                        maximum: maximum_input_cardinality,
                    },
                },
                Binder {
                    name: slots.clone(),
                    ty: Type::List {
                        element: Box::new(integer.clone()),
                        maximum: multiplicity,
                    },
                },
            ],
            definitions: vec![constructor.proposal.definition.clone()],
            rules: vec![RewriteRule {
                name: rule_name.clone(),
                binders: vec![
                    RuleBinder {
                        variable: Binder {
                            name: item.clone(),
                            ty: integer.clone(),
                        },
                        domain: Expr::Var(collection.clone()),
                        quantifier: Quantifier::ForEach,
                    },
                    RuleBinder {
                        variable: Binder {
                            name: slot.clone(),
                            ty: integer,
                        },
                        domain: Expr::Var(slots),
                        quantifier: Quantifier::ForEach,
                    },
                ],
                guard: Expr::Bool(true),
                effects: vec![Effect::Emit(Expr::Call {
                    function: constructor.proposal.definition.name.clone(),
                    arguments: vec![Expr::Var(item), Expr::Var(slot)],
                })],
                priority: 0,
            }],
            schedule: Schedule::Sequence(vec![rule_name]),
            renderer: Renderer {
                canvas: Expr::Var(collection),
                layers: Vec::new(),
                conflict_policy: ConflictPolicy::SameValueIdempotent,
                background: Expr::Int(0),
            },
            assumptions: Vec::new(),
            proof_annotations: Vec::new(),
        };
        let theory = TypeChecker::new(10_000)
            .check_theory(&theory)
            .map_err(CollectionInventionError::Theory)?;
        Ok(LearnedExpansionMechanism {
            constructor,
            multiplicity,
            theory,
            source_episodes,
            observed_input_cardinalities,
            observed_output_cardinalities: traces.iter().map(|trace| trace.outputs.len()).collect(),
        })
    }

    /// Execute a task-local expansion on new inputs.
    ///
    /// # Errors
    ///
    /// Propagates exact constructor execution failure.
    pub fn expand(
        &self,
        learned: &LearnedExpansionMechanism,
        inputs: &[i64],
    ) -> Result<BTreeSet<i64>, CollectionInventionError> {
        let item = VarName("item".to_owned());
        let slot = VarName("slot".to_owned());
        inputs
            .iter()
            .flat_map(|value| (0..learned.multiplicity).map(move |index| (*value, index)))
            .map(|(value, index)| {
                let mut environment = RuntimeEnvironment::new();
                environment.insert(item.clone(), Value::Int(value));
                environment.insert(slot.clone(), Value::Int(i64::from(index)));
                match DefinitionInventor
                    .predict(
                        &learned.constructor.proposal,
                        &environment,
                        cairn_rkl::ExecLimits::default(),
                    )
                    .map_err(CollectionInventionError::Invention)?
                {
                    Value::Int(output) => Ok(output),
                    _ => Err(CollectionInventionError::InvalidTrace),
                }
            })
            .collect()
    }

    /// Learn one element transformation from aligned collection traces and
    /// lift it into a variable-cardinality `ForEach` rule.
    ///
    /// # Errors
    ///
    /// Requires independent traces with aligned elements and at least two
    /// observed cardinalities, then returns exact synthesis or theory-checking
    /// failures.
    #[allow(clippy::too_many_lines)] // Keep element induction and quantified lifting atomic.
    pub fn invent(
        &self,
        traces: &[CollectionTrace],
        maximum_cardinality: u32,
        limits: InventionLimits,
    ) -> Result<LearnedCollectionMechanism, CollectionInventionError> {
        if traces.len() < 2 {
            return Err(CollectionInventionError::InsufficientEpisodes);
        }
        let source_episodes = traces
            .iter()
            .map(|trace| trace.episode)
            .collect::<BTreeSet<_>>();
        if source_episodes.len() != traces.len() {
            return Err(CollectionInventionError::DuplicateEpisode);
        }
        if maximum_cardinality == 0
            || traces.iter().any(|trace| {
                trace.inputs.is_empty()
                    || trace.inputs.len() != trace.outputs.len()
                    || trace.inputs.len() > maximum_cardinality as usize
            })
        {
            return Err(CollectionInventionError::InvalidTrace);
        }
        let observed_cardinalities = traces
            .iter()
            .map(|trace| trace.inputs.len())
            .collect::<BTreeSet<_>>();
        if observed_cardinalities.len() < 2 {
            return Err(CollectionInventionError::FixedCardinalityOnly);
        }
        let item = VarName("item".to_owned());
        let integer = Type::Int(cairn_rkl::IntRange {
            lower: -i64::from(u16::MAX),
            upper: i64::from(u16::MAX),
        });
        let demonstrations = traces
            .iter()
            .flat_map(|trace| trace.inputs.iter().zip(&trace.outputs))
            .map(|(input, output)| {
                let mut inputs = RuntimeEnvironment::new();
                inputs.insert(item.clone(), Value::Int(*input));
                BindingExample {
                    inputs,
                    output: Value::Int(*output),
                }
            })
            .collect();
        let element = DefinitionInventor
            .invent(&InventionProblem {
                definition_name: "invented_element_transform".to_owned(),
                parameters: vec![Binder {
                    name: item.clone(),
                    ty: integer.clone(),
                }],
                return_type: integer.clone(),
                literals: vec![Value::Int(0), Value::Int(1), Value::Int(2)],
                demonstrations,
                limits,
            })
            .map_err(CollectionInventionError::Invention)?;
        let collection = VarName("collection".to_owned());
        let collection_type = Type::List {
            element: Box::new(integer.clone()),
            maximum: maximum_cardinality,
        };
        let rule_name = "invented_for_each".to_owned();
        let theory = Theory {
            inputs: vec![Binder {
                name: collection.clone(),
                ty: collection_type,
            }],
            definitions: vec![element.proposal.definition.clone()],
            rules: vec![RewriteRule {
                name: rule_name.clone(),
                binders: vec![RuleBinder {
                    variable: Binder {
                        name: item.clone(),
                        ty: integer,
                    },
                    domain: Expr::Var(collection.clone()),
                    quantifier: Quantifier::ForEach,
                }],
                guard: Expr::Bool(true),
                effects: vec![Effect::Emit(Expr::Call {
                    function: element.proposal.definition.name.clone(),
                    arguments: vec![Expr::Var(item)],
                })],
                priority: 0,
            }],
            schedule: Schedule::Sequence(vec![rule_name]),
            renderer: Renderer {
                canvas: Expr::Var(collection),
                layers: Vec::new(),
                conflict_policy: ConflictPolicy::SameValueIdempotent,
                background: Expr::Int(0),
            },
            assumptions: Vec::new(),
            proof_annotations: Vec::new(),
        };
        let theory = TypeChecker::new(10_000)
            .check_theory(&theory)
            .map_err(CollectionInventionError::Theory)?;
        Ok(LearnedCollectionMechanism {
            element,
            theory,
            source_episodes,
            observed_cardinalities,
        })
    }

    /// Invent an element-selection predicate from changing-cardinality traces
    /// and lift it into a guarded `ForEach` theory.
    ///
    /// # Errors
    ///
    /// Rejects insufficient, duplicate, fixed-cardinality, non-subset, or
    /// one-class traces and propagates bounded invention/checking failures.
    pub fn invent_filter(
        &self,
        traces: &[CollectionTrace],
        maximum_cardinality: u32,
        limits: InventionLimits,
    ) -> Result<LearnedCollectionMechanism, CollectionInventionError> {
        let (source_episodes, observed_cardinalities) =
            validate_filter_traces(traces, maximum_cardinality)?;
        let item = VarName("item".to_owned());
        let integer = Type::Int(cairn_rkl::IntRange {
            lower: -i64::from(u16::MAX),
            upper: i64::from(u16::MAX),
        });
        let demonstrations = traces
            .iter()
            .flat_map(|trace| {
                let selected = trace.outputs.iter().copied().collect::<BTreeSet<_>>();
                let item = item.clone();
                trace.inputs.iter().map(move |input| {
                    let mut inputs = RuntimeEnvironment::new();
                    inputs.insert(item.clone(), Value::Int(*input));
                    BindingExample {
                        inputs,
                        output: Value::Bool(selected.contains(input)),
                    }
                })
            })
            .collect();
        let element = DefinitionInventor
            .invent(&InventionProblem {
                definition_name: "invented_element_selector".to_owned(),
                parameters: vec![Binder {
                    name: item.clone(),
                    ty: integer.clone(),
                }],
                return_type: Type::Bool,
                literals: vec![Value::Int(0), Value::Int(1), Value::Int(2)],
                demonstrations,
                limits,
            })
            .map_err(CollectionInventionError::Invention)?;
        let collection = VarName("collection".to_owned());
        let rule_name = "invented_filter".to_owned();
        let theory = Theory {
            inputs: vec![Binder {
                name: collection.clone(),
                ty: Type::List {
                    element: Box::new(integer.clone()),
                    maximum: maximum_cardinality,
                },
            }],
            definitions: vec![element.proposal.definition.clone()],
            rules: vec![RewriteRule {
                name: rule_name.clone(),
                binders: vec![RuleBinder {
                    variable: Binder {
                        name: item.clone(),
                        ty: integer,
                    },
                    domain: Expr::Var(collection.clone()),
                    quantifier: Quantifier::ForEach,
                }],
                guard: Expr::Call {
                    function: element.proposal.definition.name.clone(),
                    arguments: vec![Expr::Var(item.clone())],
                },
                effects: vec![Effect::Emit(Expr::Var(item))],
                priority: 0,
            }],
            schedule: Schedule::Sequence(vec![rule_name]),
            renderer: Renderer {
                canvas: Expr::Var(collection),
                layers: Vec::new(),
                conflict_policy: ConflictPolicy::SameValueIdempotent,
                background: Expr::Int(0),
            },
            assumptions: Vec::new(),
            proof_annotations: Vec::new(),
        };
        let theory = TypeChecker::new(10_000)
            .check_theory(&theory)
            .map_err(CollectionInventionError::Theory)?;
        Ok(LearnedCollectionMechanism {
            element,
            theory,
            source_episodes,
            observed_cardinalities,
        })
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum EntityFeature {
    Attribute(FeatureId),
    OutDegree(RelationRole),
    InDegree(RelationRole),
    TwoHopOutDegree(RelationRole, RelationRole),
}

#[derive(Clone, Debug, PartialEq)]
pub struct LearnedEntitySelector {
    pub selector: InventionOutcome,
    pub features: Vec<EntityFeature>,
    pub source_collections: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EntitySelectionError {
    InsufficientCollections,
    EmptyCollection,
    UnknownEntity,
    InvalidSelection,
    Contrast(ContrastError),
    Prediction(InventionError),
}

#[derive(Clone, Copy, Debug, Default)]
pub struct RelationalEntitySelectorInventor;

impl RelationalEntitySelectorInventor {
    /// Invent one executable entity predicate from attributes and relation-role
    /// degrees, then retain the feature contract needed for rebinding.
    ///
    /// # Errors
    ///
    /// Rejects insufficient or malformed collections, invalid selections, and
    /// bounded contrast-invention failure.
    pub fn invent(
        &self,
        problem: &EntitySelectionProblem,
        limits: InventionLimits,
    ) -> Result<LearnedEntitySelector, EntitySelectionError> {
        if problem.demonstrations.len() < 2 {
            return Err(EntitySelectionError::InsufficientCollections);
        }
        for demonstration in &problem.demonstrations {
            validate_entity_collection(&demonstration.collection)?;
            let ids = demonstration
                .collection
                .entities
                .iter()
                .map(|entity| entity.id.clone())
                .collect::<BTreeSet<_>>();
            if !demonstration.selected.is_subset(&ids) {
                return Err(EntitySelectionError::InvalidSelection);
            }
        }
        validate_entity_collection(&problem.prediction)?;
        let features = entity_feature_schema(
            problem
                .demonstrations
                .iter()
                .map(|demonstration| &demonstration.collection)
                .chain(std::iter::once(&problem.prediction)),
        );
        let feature_binders = features
            .iter()
            .enumerate()
            .map(|(index, _)| {
                (
                    VarName(format!("feature_{index}")),
                    Type::Int(cairn_rkl::IntRange {
                        lower: -i64::from(u16::MAX),
                        upper: i64::from(u16::MAX),
                    }),
                )
            })
            .collect::<Vec<_>>();
        let examples = problem
            .demonstrations
            .iter()
            .flat_map(|demonstration| {
                demonstration.collection.entities.iter().map(|entity| {
                    let values = entity_features(&demonstration.collection, &entity.id, &features);
                    ContrastExample {
                        features: feature_binders
                            .iter()
                            .map(|(name, _)| name.clone())
                            .zip(values.into_iter().map(Value::Int))
                            .collect(),
                        selected: demonstration.selected.contains(&entity.id),
                    }
                })
            })
            .collect::<Vec<_>>();
        let selector = ContrastPredicateInventor
            .invent(
                "invented_entity_selector",
                &feature_binders,
                &examples,
                vec![Value::Int(0), Value::Int(1), Value::Int(2)],
                limits,
            )
            .map_err(EntitySelectionError::Contrast)?;
        Ok(LearnedEntitySelector {
            selector,
            features,
            source_collections: problem.demonstrations.len(),
        })
    }

    /// Rebind and execute a learned selector on an unseen entity collection.
    ///
    /// # Errors
    ///
    /// Rejects malformed relations or bounded selector execution failure.
    pub fn predict(
        &self,
        learned: &LearnedEntitySelector,
        collection: &RelationalEntityCollection,
    ) -> Result<BTreeSet<EntityId>, EntitySelectionError> {
        validate_entity_collection(collection)?;
        collection
            .entities
            .iter()
            .filter_map(|entity| {
                let values = entity_features(collection, &entity.id, &learned.features);
                let mut inputs = RuntimeEnvironment::new();
                for (parameter, value) in learned
                    .selector
                    .proposal
                    .definition
                    .parameters
                    .iter()
                    .zip(values)
                {
                    inputs.insert(parameter.name.clone(), Value::Int(value));
                }
                match DefinitionInventor.predict(
                    &learned.selector.proposal,
                    &inputs,
                    cairn_rkl::ExecLimits::default(),
                ) {
                    Ok(Value::Bool(true)) => Some(Ok(entity.id.clone())),
                    Ok(Value::Bool(false)) => None,
                    Ok(_) => Some(Err(EntitySelectionError::Prediction(
                        InventionError::VerificationMismatch,
                    ))),
                    Err(error) => Some(Err(EntitySelectionError::Prediction(error))),
                }
            })
            .collect()
    }
}

fn validate_entity_collection(
    collection: &RelationalEntityCollection,
) -> Result<(), EntitySelectionError> {
    if collection.entities.is_empty() {
        return Err(EntitySelectionError::EmptyCollection);
    }
    let ids = collection
        .entities
        .iter()
        .map(|entity| entity.id.clone())
        .collect::<BTreeSet<_>>();
    if ids.len() != collection.entities.len()
        || collection
            .relations
            .iter()
            .any(|relation| !ids.contains(&relation.subject) || !ids.contains(&relation.object))
    {
        return Err(EntitySelectionError::UnknownEntity);
    }
    Ok(())
}

fn entity_feature_schema<'a>(
    collections: impl Iterator<Item = &'a RelationalEntityCollection>,
) -> Vec<EntityFeature> {
    let mut attributes = BTreeSet::new();
    let mut roles = BTreeSet::new();
    for collection in collections {
        for entity in &collection.entities {
            attributes.extend(entity.attributes.keys().cloned());
        }
        roles.extend(
            collection
                .relations
                .iter()
                .map(|relation| relation.role.clone()),
        );
    }
    let path_roles = roles
        .iter()
        .flat_map(|first| {
            roles
                .iter()
                .map(move |second| EntityFeature::TwoHopOutDegree(first.clone(), second.clone()))
        })
        .collect::<Vec<_>>();
    attributes
        .into_iter()
        .map(EntityFeature::Attribute)
        .chain(roles.clone().into_iter().map(EntityFeature::OutDegree))
        .chain(roles.into_iter().map(EntityFeature::InDegree))
        .chain(path_roles)
        .collect()
}

fn entity_features(
    collection: &RelationalEntityCollection,
    id: &EntityId,
    features: &[EntityFeature],
) -> Vec<i64> {
    let entity = collection.entities.iter().find(|entity| &entity.id == id);
    features
        .iter()
        .map(|feature| match feature {
            EntityFeature::Attribute(feature) => entity
                .and_then(|entity| entity.attributes.get(feature))
                .copied()
                .unwrap_or(0),
            EntityFeature::OutDegree(role) => i64::try_from(
                collection
                    .relations
                    .iter()
                    .filter(|relation| relation.role == *role && relation.subject == *id)
                    .count(),
            )
            .unwrap_or(i64::MAX),
            EntityFeature::InDegree(role) => i64::try_from(
                collection
                    .relations
                    .iter()
                    .filter(|relation| relation.role == *role && relation.object == *id)
                    .count(),
            )
            .unwrap_or(i64::MAX),
            EntityFeature::TwoHopOutDegree(first, second) => i64::try_from(
                collection
                    .relations
                    .iter()
                    .filter(|relation| relation.role == *first && relation.subject == *id)
                    .flat_map(|relation| {
                        collection.relations.iter().filter_map(move |next| {
                            (next.role == *second && next.subject == relation.object)
                                .then_some(next.object.clone())
                        })
                    })
                    .collect::<BTreeSet<_>>()
                    .len(),
            )
            .unwrap_or(i64::MAX),
        })
        .collect()
}

fn validate_filter_traces(
    traces: &[CollectionTrace],
    maximum_cardinality: u32,
) -> Result<(BTreeSet<cairn_core::Hash32>, BTreeSet<usize>), CollectionInventionError> {
    if traces.len() < 2 {
        return Err(CollectionInventionError::InsufficientEpisodes);
    }
    let source = traces
        .iter()
        .map(|trace| trace.episode)
        .collect::<BTreeSet<_>>();
    if source.len() != traces.len() {
        return Err(CollectionInventionError::DuplicateEpisode);
    }
    if maximum_cardinality == 0
        || traces.iter().any(|trace| {
            let inputs = trace.inputs.iter().copied().collect::<BTreeSet<_>>();
            trace.inputs.is_empty()
                || inputs.len() != trace.inputs.len()
                || trace.inputs.len() > maximum_cardinality as usize
                || trace.outputs.iter().any(|value| !inputs.contains(value))
        })
    {
        return Err(CollectionInventionError::InvalidTrace);
    }
    let cardinalities = traces
        .iter()
        .map(|trace| trace.inputs.len())
        .collect::<BTreeSet<_>>();
    if cardinalities.len() < 2 {
        return Err(CollectionInventionError::FixedCardinalityOnly);
    }
    let selected = traces
        .iter()
        .map(|trace| trace.outputs.len())
        .sum::<usize>();
    let total = traces.iter().map(|trace| trace.inputs.len()).sum::<usize>();
    if selected == 0 || selected == total {
        return Err(CollectionInventionError::InvalidTrace);
    }
    Ok((source, cardinalities))
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
