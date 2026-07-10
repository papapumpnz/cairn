use crate::{
    CheckError, CheckedTheory, ConflictPolicy, DefinitionSignature, Effect, ExecError, ExecLimits,
    ExecutionTrace, Expr, Quantifier, ReferenceExecutor, RewriteRule, RuntimeEnvironment, Schedule,
    TraceLevel, TypeChecker, TypeEnvironment, TypedExpr, Value,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

pub const THEORY_EXECUTION_SEMANTICS_VERSION: &str = "1.0.0";

type SemanticMap = BTreeMap<Value, Option<Value>>;
type SemanticSnapshot = (SemanticMap, SemanticMap, SemanticMap);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TheoryExecLimits {
    pub expression: ExecLimits,
    pub maximum_bindings: u64,
    pub maximum_rule_firings: u64,
}

impl Default for TheoryExecLimits {
    fn default() -> Self {
        Self {
            expression: ExecLimits::default(),
            maximum_bindings: 1_000_000,
            maximum_rule_firings: 1_000_000,
        }
    }
}

#[derive(Clone, Debug)]
pub struct TheoryExecutionRequest {
    pub types: TypeEnvironment,
    pub values: RuntimeEnvironment,
    pub initial_facts: BTreeSet<Value>,
    pub initial_state: BTreeMap<Value, Value>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LogicalWrite {
    pub target: Value,
    pub value: Value,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RenderedObservation {
    pub canvas: Value,
    pub background: Value,
    pub layers: Vec<Value>,
    pub writes: BTreeMap<Value, Value>,
    pub facts: BTreeSet<Value>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuleFiring {
    pub rule: String,
    pub priority: u32,
    pub binding_ordinal: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TheoryExecutionTrace {
    pub rendered: RenderedObservation,
    pub state: BTreeMap<Value, Value>,
    pub rule_firings: Vec<RuleFiring>,
    pub expression_steps: u64,
    pub fixpoint_iterations: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TheoryExecError {
    Check(CheckError),
    Expression(ExecError),
    ExpressionBackend(String),
    InvalidDomain,
    MissingInput(String),
    InputTypeMismatch(String),
    Cardinality {
        quantifier: Quantifier,
        actual: usize,
    },
    BindingCapacity {
        maximum: u64,
    },
    FiringCapacity {
        maximum: u64,
    },
    UnknownRule(String),
    AssumptionFailed(usize),
    GuardNotBoolean,
    InvalidLayer,
    Conflict {
        target: Value,
    },
    PriorityTie {
        target: Value,
        priority: u32,
    },
    FixpointDidNotConverge {
        maximum: u32,
    },
    ScheduleCycle,
    Capacity,
}

impl fmt::Display for TheoryExecError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for TheoryExecError {}

impl From<CheckError> for TheoryExecError {
    fn from(value: CheckError) -> Self {
        Self::Check(value)
    }
}

impl From<ExecError> for TheoryExecError {
    fn from(value: ExecError) -> Self {
        Self::Expression(value)
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ReferenceTheoryExecutor;

/// Pluggable checked-expression backend used by the shared theory scheduler.
/// This keeps scheduling semantics singular while allowing the interpreter and
/// bytecode machine to be checked differentially.
pub trait TheoryExpressionExecutor {
    /// Execute one already type-checked expression.
    ///
    /// # Errors
    ///
    /// Returns a stable expression or backend failure.
    fn execute(
        &self,
        theory: &CheckedTheory,
        expression: &TypedExpr,
        environment: &RuntimeEnvironment,
        limits: ExecLimits,
    ) -> Result<ExecutionTrace, TheoryExecError>;
}

#[derive(Clone, Copy, Debug, Default)]
struct ReferenceExpressionExecutor;

impl TheoryExpressionExecutor for ReferenceExpressionExecutor {
    fn execute(
        &self,
        theory: &CheckedTheory,
        expression: &TypedExpr,
        environment: &RuntimeEnvironment,
        limits: ExecLimits,
    ) -> Result<ExecutionTrace, TheoryExecError> {
        ReferenceExecutor::new(theory, limits, TraceLevel::Off)
            .execute(expression, environment)
            .map_err(TheoryExecError::from)
    }
}

impl ReferenceTheoryExecutor {
    /// Execute a checked theory including assumptions, rule quantification,
    /// schedules, effects, conflicts, and renderer layers.
    ///
    /// # Errors
    ///
    /// Returns exact checking, execution, capacity, schedule, layer, cardinality,
    /// fixpoint, or conflict failures.
    pub fn execute(
        &self,
        theory: &CheckedTheory,
        request: &TheoryExecutionRequest,
        limits: TheoryExecLimits,
    ) -> Result<TheoryExecutionTrace, TheoryExecError> {
        execute_theory_with(&ReferenceExpressionExecutor, theory, request, limits)
    }
}

/// Execute whole-theory semantics with a caller-selected expression backend.
///
/// # Errors
///
/// Returns exact admission, backend, schedule, conflict, or capacity failures.
pub fn execute_theory_with<E: TheoryExpressionExecutor>(
    backend: &E,
    theory: &CheckedTheory,
    request: &TheoryExecutionRequest,
    limits: TheoryExecLimits,
) -> Result<TheoryExecutionTrace, TheoryExecError> {
    let mut machine = TheoryMachine::new(backend, theory, request, limits)?;
    machine.check_assumptions()?;
    machine.execute_schedule(&theory.theory().schedule)?;
    let (canvas, steps) = machine.evaluate(
        &theory.theory().renderer.canvas,
        &request.types,
        &request.values,
    )?;
    machine.expression_steps = machine.expression_steps.saturating_add(steps);
    let (background, steps) = machine.evaluate(
        &theory.theory().renderer.background,
        &request.types,
        &request.values,
    )?;
    machine.expression_steps = machine.expression_steps.saturating_add(steps);
    let mut layers = Vec::with_capacity(theory.theory().renderer.layers.len());
    for (index, layer) in theory.theory().renderer.layers.iter().enumerate() {
        let (value, steps) = machine.evaluate(layer, &request.types, &request.values)?;
        machine.expression_steps = machine.expression_steps.saturating_add(steps);
        machine.apply_layer(
            &value,
            u32::try_from(index).map_err(|_| TheoryExecError::Capacity)?,
        )?;
        layers.push(value);
    }
    Ok(TheoryExecutionTrace {
        rendered: RenderedObservation {
            canvas,
            background,
            layers,
            writes: machine
                .writes
                .into_iter()
                .filter_map(|(target, resolved)| resolved.value.map(|value| (target, value)))
                .collect(),
            facts: machine
                .facts
                .into_iter()
                .filter_map(|(target, resolved)| resolved.value.map(|_| target))
                .collect(),
        },
        state: machine
            .state
            .into_iter()
            .filter_map(|(target, resolved)| resolved.value.map(|value| (target, value)))
            .collect(),
        rule_firings: machine.rule_firings,
        expression_steps: machine.expression_steps,
        fixpoint_iterations: machine.fixpoint_iterations,
    })
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Resolved {
    value: Option<Value>,
    priority: u32,
    ordinal: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ChangeSpace {
    Fact,
    Write,
    State,
}

#[derive(Clone, Debug)]
struct Change {
    space: ChangeSpace,
    target: Value,
    value: Option<Value>,
    priority: u32,
    ordinal: u64,
    preserve: bool,
}

struct TheoryMachine<'a, E> {
    backend: &'a E,
    theory: &'a CheckedTheory,
    checker: TypeChecker,
    limits: TheoryExecLimits,
    facts: BTreeMap<Value, Resolved>,
    writes: BTreeMap<Value, Resolved>,
    state: BTreeMap<Value, Resolved>,
    rule_firings: Vec<RuleFiring>,
    expression_steps: u64,
    fixpoint_iterations: u64,
    bindings_seen: u64,
    changes_seen: u64,
    base_types: TypeEnvironment,
    base_values: RuntimeEnvironment,
}

impl<'a, E: TheoryExpressionExecutor> TheoryMachine<'a, E> {
    fn new(
        backend: &'a E,
        theory: &'a CheckedTheory,
        request: &TheoryExecutionRequest,
        limits: TheoryExecLimits,
    ) -> Result<Self, TheoryExecError> {
        if limits.maximum_bindings == 0 || limits.maximum_rule_firings == 0 {
            return Err(TheoryExecError::Capacity);
        }
        for input in &theory.theory().inputs {
            let Some(actual_type) = request.types.get(&input.name) else {
                return Err(TheoryExecError::MissingInput(input.name.0.clone()));
            };
            if actual_type != &input.ty {
                return Err(TheoryExecError::InputTypeMismatch(input.name.0.clone()));
            }
            let Some(value) = request.values.get(&input.name) else {
                return Err(TheoryExecError::MissingInput(input.name.0.clone()));
            };
            if !value.conforms_to(&input.ty) {
                return Err(TheoryExecError::InputTypeMismatch(input.name.0.clone()));
            }
        }
        let mut checker = TypeChecker::new(1_000_000);
        for definition in &theory.theory().definitions {
            checker.register_definition(
                definition.name.clone(),
                DefinitionSignature {
                    parameters: definition
                        .parameters
                        .iter()
                        .map(|parameter| parameter.ty.clone())
                        .collect(),
                    return_type: definition.return_type.clone(),
                },
            );
        }
        let facts = request
            .initial_facts
            .iter()
            .cloned()
            .map(|value| {
                (
                    value.clone(),
                    Resolved {
                        value: Some(value),
                        priority: 0,
                        ordinal: 0,
                    },
                )
            })
            .collect();
        let state = request
            .initial_state
            .iter()
            .map(|(target, value)| {
                (
                    target.clone(),
                    Resolved {
                        value: Some(value.clone()),
                        priority: 0,
                        ordinal: 0,
                    },
                )
            })
            .collect();
        Ok(Self {
            backend,
            theory,
            checker,
            limits,
            facts,
            writes: BTreeMap::new(),
            state,
            rule_firings: Vec::new(),
            expression_steps: 0,
            fixpoint_iterations: 0,
            bindings_seen: 0,
            changes_seen: 0,
            base_types: request.types.clone(),
            base_values: request.values.clone(),
        })
    }

    fn check_assumptions(&mut self) -> Result<(), TheoryExecError> {
        let types = self.base_types.clone();
        let values = self.base_values.clone();
        for (index, assumption) in self.theory.theory().assumptions.iter().enumerate() {
            let (value, steps) = self.evaluate(assumption, &types, &values)?;
            self.expression_steps = self.expression_steps.saturating_add(steps);
            if value != Value::Bool(true) {
                return Err(TheoryExecError::AssumptionFailed(index));
            }
        }
        Ok(())
    }

    fn evaluate(
        &self,
        expression: &Expr,
        types: &TypeEnvironment,
        values: &RuntimeEnvironment,
    ) -> Result<(Value, u64), TheoryExecError> {
        let checked_expression = self.checker.check(expression, types)?;
        let trace = self.backend.execute(
            self.theory,
            &checked_expression,
            values,
            self.limits.expression,
        )?;
        Ok((trace.result, trace.steps))
    }

    fn execute_schedule(&mut self, schedule: &Schedule) -> Result<(), TheoryExecError> {
        match schedule {
            Schedule::Parallel => {
                let names = self
                    .theory
                    .theory()
                    .rules
                    .iter()
                    .map(|rule| rule.name.clone())
                    .collect::<Vec<_>>();
                self.execute_parallel(&names).map(|_| ())
            }
            Schedule::Sequence(names) => self.execute_sequence(names).map(|_| ()),
            Schedule::DependencyDag(edges) => {
                let names = self.topological_order(edges)?;
                self.execute_sequence(&names).map(|_| ())
            }
            Schedule::DecisionList(names) => {
                for name in names {
                    let (changes, matched) = self.evaluate_named_rule(name)?;
                    if matched {
                        self.apply_changes(changes)?;
                        break;
                    }
                }
                Ok(())
            }
            Schedule::BoundedFixpoint {
                rules,
                maximum_steps,
            } => {
                for iteration in 0..*maximum_steps {
                    let before = self.semantic_snapshot();
                    self.execute_sequence(rules)?;
                    self.fixpoint_iterations = self.fixpoint_iterations.saturating_add(1);
                    if before == self.semantic_snapshot() {
                        return Ok(());
                    }
                    if iteration + 1 == *maximum_steps {
                        return Err(TheoryExecError::FixpointDidNotConverge {
                            maximum: *maximum_steps,
                        });
                    }
                }
                Ok(())
            }
            Schedule::Hierarchical(parts) => {
                for part in parts {
                    self.execute_schedule(part)?;
                }
                Ok(())
            }
        }
    }

    fn execute_parallel(&mut self, names: &[String]) -> Result<bool, TheoryExecError> {
        let mut changes = Vec::new();
        let mut matched = false;
        for name in names {
            let (mut rule_changes, rule_matched) = self.evaluate_named_rule(name)?;
            changes.append(&mut rule_changes);
            matched |= rule_matched;
        }
        self.apply_changes(changes)?;
        Ok(matched)
    }

    fn execute_sequence(&mut self, names: &[String]) -> Result<bool, TheoryExecError> {
        let mut matched = false;
        for name in names {
            let (changes, rule_matched) = self.evaluate_named_rule(name)?;
            self.apply_changes(changes)?;
            matched |= rule_matched;
        }
        Ok(matched)
    }

    fn evaluate_named_rule(&mut self, name: &str) -> Result<(Vec<Change>, bool), TheoryExecError> {
        let rule = self
            .theory
            .theory()
            .rules
            .iter()
            .find(|rule| rule.name == name)
            .cloned()
            .ok_or_else(|| TheoryExecError::UnknownRule(name.to_owned()))?;
        self.evaluate_rule(&rule)
    }

    fn evaluate_rule(
        &mut self,
        rule: &RewriteRule,
    ) -> Result<(Vec<Change>, bool), TheoryExecError> {
        let bindings =
            self.expand_binders(rule, 0, self.base_types.clone(), self.base_values.clone())?;
        let existential = rule
            .binders
            .iter()
            .any(|binder| binder.quantifier == Quantifier::Exists);
        let mut changes = Vec::new();
        let mut matched = false;
        for (types, values) in bindings {
            let (guard, steps) = self.evaluate(&rule.guard, &types, &values)?;
            self.expression_steps = self.expression_steps.saturating_add(steps);
            match guard {
                Value::Bool(false) => continue,
                Value::Bool(true) => {}
                _ => return Err(TheoryExecError::GuardNotBoolean),
            }
            matched = true;
            let binding_ordinal = self.bindings_seen;
            self.rule_firings.push(RuleFiring {
                rule: rule.name.clone(),
                priority: rule.priority,
                binding_ordinal,
            });
            if self.rule_firings.len() as u64 > self.limits.maximum_rule_firings {
                return Err(TheoryExecError::FiringCapacity {
                    maximum: self.limits.maximum_rule_firings,
                });
            }
            for effect in &rule.effects {
                self.effect_changes(
                    effect,
                    rule.priority,
                    binding_ordinal,
                    &types,
                    &values,
                    &mut changes,
                )?;
            }
            if existential {
                break;
            }
        }
        Ok((changes, matched))
    }

    fn expand_binders(
        &mut self,
        rule: &RewriteRule,
        index: usize,
        types: TypeEnvironment,
        values: RuntimeEnvironment,
    ) -> Result<Vec<(TypeEnvironment, RuntimeEnvironment)>, TheoryExecError> {
        if index == rule.binders.len() {
            self.bindings_seen = self.bindings_seen.saturating_add(1);
            if self.bindings_seen > self.limits.maximum_bindings {
                return Err(TheoryExecError::BindingCapacity {
                    maximum: self.limits.maximum_bindings,
                });
            }
            return Ok(vec![(types, values)]);
        }
        let binder = &rule.binders[index];
        let (domain, steps) = self.evaluate(&binder.domain, &types, &values)?;
        self.expression_steps = self.expression_steps.saturating_add(steps);
        let Value::Set {
            values: members, ..
        } = domain.clone()
        else {
            return Err(TheoryExecError::InvalidDomain);
        };
        let bindings = match binder.quantifier {
            Quantifier::One => members.iter().next().cloned().into_iter().collect(),
            Quantifier::Unique => {
                if members.len() != 1 {
                    return Err(TheoryExecError::Cardinality {
                        quantifier: Quantifier::Unique,
                        actual: members.len(),
                    });
                }
                members.into_iter().collect()
            }
            Quantifier::ForEach | Quantifier::Exists => members.into_iter().collect(),
            Quantifier::Group => vec![domain],
        };
        let mut expanded = Vec::new();
        for value in bindings {
            let mut nested_types = types.clone();
            nested_types.insert(binder.variable.name.clone(), binder.variable.ty.clone());
            let mut nested_values = values.clone();
            nested_values.insert(binder.variable.name.clone(), value);
            expanded.extend(self.expand_binders(rule, index + 1, nested_types, nested_values)?);
        }
        Ok(expanded)
    }

    #[allow(clippy::too_many_arguments)]
    fn effect_changes(
        &mut self,
        effect: &Effect,
        priority: u32,
        binding_ordinal: u64,
        types: &TypeEnvironment,
        values: &RuntimeEnvironment,
        changes: &mut Vec<Change>,
    ) -> Result<(), TheoryExecError> {
        self.changes_seen = self.changes_seen.saturating_add(1);
        let ordinal = self.changes_seen;
        let mut push = |space, target: Value, value, preserve| {
            changes.push(Change {
                space,
                target,
                value,
                priority,
                ordinal: ordinal.saturating_add(binding_ordinal),
                preserve,
            });
        };
        match effect {
            Effect::Preserve(expression) => {
                let (value, steps) = self.evaluate(expression, types, values)?;
                self.expression_steps = self.expression_steps.saturating_add(steps);
                push(ChangeSpace::Fact, value.clone(), Some(value), true);
            }
            Effect::Delete(expression) => {
                let (value, steps) = self.evaluate(expression, types, values)?;
                self.expression_steps = self.expression_steps.saturating_add(steps);
                push(ChangeSpace::Fact, value, None, false);
            }
            Effect::Emit(expression) => {
                let (value, steps) = self.evaluate(expression, types, values)?;
                self.expression_steps = self.expression_steps.saturating_add(steps);
                push(ChangeSpace::Fact, value.clone(), Some(value), false);
            }
            Effect::Write { target, value } => {
                let (target, target_steps) = self.evaluate(target, types, values)?;
                let (value, value_steps) = self.evaluate(value, types, values)?;
                self.expression_steps = self
                    .expression_steps
                    .saturating_add(target_steps.saturating_add(value_steps));
                push(ChangeSpace::Write, target, Some(value), false);
            }
            Effect::SetState { target, value } => {
                let (target, target_steps) = self.evaluate(target, types, values)?;
                let (value, value_steps) = self.evaluate(value, types, values)?;
                self.expression_steps = self
                    .expression_steps
                    .saturating_add(target_steps.saturating_add(value_steps));
                push(ChangeSpace::State, target, Some(value), false);
            }
        }
        Ok(())
    }

    fn apply_layer(&mut self, value: &Value, priority: u32) -> Result<(), TheoryExecError> {
        let entries = match value {
            Value::Tuple(values) if values.len() == 2 => vec![values.clone()],
            Value::Set { values, .. } => values
                .iter()
                .map(|value| match value {
                    Value::Tuple(values) if values.len() == 2 => Ok(values.clone()),
                    _ => Err(TheoryExecError::InvalidLayer),
                })
                .collect::<Result<Vec<_>, _>>()?,
            _ => return Err(TheoryExecError::InvalidLayer),
        };
        let changes = entries
            .into_iter()
            .map(|entry| {
                self.changes_seen = self.changes_seen.saturating_add(1);
                Change {
                    space: ChangeSpace::Write,
                    target: entry[0].clone(),
                    value: Some(entry[1].clone()),
                    priority,
                    ordinal: self.changes_seen,
                    preserve: false,
                }
            })
            .collect();
        self.apply_changes(changes)
    }

    fn apply_changes(&mut self, changes: Vec<Change>) -> Result<(), TheoryExecError> {
        let policy = self.theory.theory().renderer.conflict_policy;
        for change in changes {
            let target = change.target.clone();
            let map = match change.space {
                ChangeSpace::Fact => &mut self.facts,
                ChangeSpace::Write => &mut self.writes,
                ChangeSpace::State => &mut self.state,
            };
            if change.preserve
                && map
                    .get(&target)
                    .is_some_and(|old| old.value == change.value)
            {
                continue;
            }
            let candidate = Resolved {
                value: change.value,
                priority: change.priority,
                ordinal: change.ordinal,
            };
            let Some(old) = map.get(&target) else {
                map.insert(target, candidate);
                continue;
            };
            let replace = match policy {
                ConflictPolicy::Reject => {
                    return Err(TheoryExecError::Conflict { target });
                }
                ConflictPolicy::SameValueIdempotent => {
                    if old.value != candidate.value {
                        return Err(TheoryExecError::Conflict { target });
                    }
                    false
                }
                ConflictPolicy::Priority => {
                    if old.priority == candidate.priority && old.value != candidate.value {
                        return Err(TheoryExecError::PriorityTie {
                            target,
                            priority: candidate.priority,
                        });
                    }
                    candidate.priority > old.priority
                }
                ConflictPolicy::LastWrite => candidate.ordinal >= old.ordinal,
            };
            if replace {
                map.insert(change.target, candidate);
            }
        }
        Ok(())
    }

    fn topological_order(
        &self,
        edges: &[(String, String)],
    ) -> Result<Vec<String>, TheoryExecError> {
        let mut incoming = self
            .theory
            .theory()
            .rules
            .iter()
            .map(|rule| (rule.name.clone(), 0_usize))
            .collect::<BTreeMap<_, _>>();
        let mut outgoing = BTreeMap::<String, BTreeSet<String>>::new();
        for (before, after) in edges {
            *incoming
                .get_mut(after)
                .ok_or_else(|| TheoryExecError::UnknownRule(after.clone()))? += 1;
            if !incoming.contains_key(before) {
                return Err(TheoryExecError::UnknownRule(before.clone()));
            }
            outgoing
                .entry(before.clone())
                .or_default()
                .insert(after.clone());
        }
        let mut ready = incoming
            .iter()
            .filter(|(_, count)| **count == 0)
            .map(|(name, _)| name.clone())
            .collect::<BTreeSet<_>>();
        let mut order = Vec::with_capacity(incoming.len());
        while let Some(name) = ready.pop_first() {
            order.push(name.clone());
            for next in outgoing.get(&name).into_iter().flatten() {
                let count = incoming.get_mut(next).expect("validated node");
                *count -= 1;
                if *count == 0 {
                    ready.insert(next.clone());
                }
            }
        }
        if order.len() != incoming.len() {
            return Err(TheoryExecError::ScheduleCycle);
        }
        Ok(order)
    }

    fn semantic_snapshot(&self) -> SemanticSnapshot {
        let values = |map: &BTreeMap<Value, Resolved>| {
            map.iter()
                .map(|(target, resolved)| (target.clone(), resolved.value.clone()))
                .collect()
        };
        (
            values(&self.facts),
            values(&self.writes),
            values(&self.state),
        )
    }
}
