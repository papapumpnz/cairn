//! Task-local invention of executable RKL definitions from demonstrations.
//!
//! The inventor is deliberately domain-neutral. It searches bounded typed RKL
//! compositions and never receives a task-family name or target expression.

use cairn_bytecode::{BytecodeCompiler, CompileLimits, FastExecutor};
use cairn_core::{CanonicalEncode, Hash32};
use cairn_rkl::{
    BinaryOp, Binder, BoundChecker, Definition, ExecLimits, Expr, ReferenceExecutor,
    RuntimeEnvironment, TraceLevel, Type, TypeChecker, TypeEnvironment, TypedExpr, UnaryOp, Value,
};
use std::collections::BTreeSet;
use std::fmt;

pub const INVENTION_SEMANTICS_VERSION: &str = "1.0.0";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BindingExample {
    pub inputs: RuntimeEnvironment,
    pub output: Value,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InventionLimits {
    pub maximum_cost: u16,
    pub maximum_candidates: u64,
    pub maximum_expression_nodes: usize,
    pub execution: ExecLimits,
}

impl Default for InventionLimits {
    fn default() -> Self {
        Self {
            maximum_cost: 9,
            maximum_candidates: 1_000_000,
            maximum_expression_nodes: 10_000,
            execution: ExecLimits::default(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InventionProblem {
    pub definition_name: String,
    pub parameters: Vec<Binder>,
    pub return_type: Type,
    /// Explicit generic literal basis. Demonstration outputs are never copied
    /// into this basis by the inventor.
    pub literals: Vec<Value>,
    pub demonstrations: Vec<BindingExample>,
    pub limits: InventionLimits,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DefinitionProposal {
    pub definition: Definition,
    pub structural_cost: u16,
    pub semantic_hash: Hash32,
    pub candidates_evaluated: u64,
    pub observational_classes: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdmissionReport {
    pub training_examples: usize,
    pub exact_reference_replays: usize,
    pub exact_cross_implementation_replays: usize,
    pub totality: TotalityStatus,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TotalityStatus {
    Checked,
}

#[derive(Clone, Debug, PartialEq)]
pub struct InventionOutcome {
    pub proposal: DefinitionProposal,
    pub admission: AdmissionReport,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum InventionError {
    EmptyDemonstrations,
    EmptyBound,
    DuplicateParameter(String),
    MissingInput(String),
    InputTypeMismatch(String),
    OutputTypeMismatch,
    UnsupportedLiteral,
    CandidateCapacity { maximum: u64 },
    NoDefinition { maximum_cost: u16 },
    InternalCheck,
    InternalExecution,
    InternalCanonicalization,
    OptimizedCompilation,
    VerificationMismatch,
}

impl fmt::Display for InventionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for InventionError {}

#[derive(Clone, Copy, Debug, Default)]
pub struct DefinitionInventor;

impl DefinitionInventor {
    /// Find the minimum-cost observed behavior expressible within the supplied
    /// generic RKL basis, then independently admit it through both executors.
    ///
    /// # Errors
    ///
    /// Returns explicit problem, capacity, exhaustion, or verification errors.
    pub fn invent(&self, problem: &InventionProblem) -> Result<InventionOutcome, InventionError> {
        validate_problem(problem)?;
        let environment = type_environment(&problem.parameters);
        let checker = TypeChecker::new(problem.limits.maximum_expression_nodes);
        let expected = problem
            .demonstrations
            .iter()
            .map(|example| example.output.clone())
            .collect::<Vec<_>>();
        let mut search = Search {
            problem,
            checker: &checker,
            environment: &environment,
            by_cost: vec![Vec::new(); usize::from(problem.limits.maximum_cost) + 1],
            behaviors: BTreeSet::new(),
            candidates_evaluated: 0,
        };

        let mut bases = problem
            .parameters
            .iter()
            .map(|parameter| Expr::Var(parameter.name.clone()))
            .collect::<Vec<_>>();
        for literal in &problem.literals {
            bases.push(literal_expression(literal)?);
        }
        for expression in bases {
            if let Some(proposal) = search.consider(expression, 1, &expected)? {
                return admit(problem, proposal);
            }
        }

        for cost in 2..=problem.limits.maximum_cost {
            if let Some(proposal) = search.generate_cost(cost, &expected)? {
                return admit(problem, proposal);
            }
        }
        Err(InventionError::NoDefinition {
            maximum_cost: problem.limits.maximum_cost,
        })
    }

    /// Execute an admitted definition body on a new binding without changing
    /// or retraining the proposal.
    ///
    /// # Errors
    ///
    /// Returns an exact input or execution failure.
    pub fn predict(
        &self,
        proposal: &DefinitionProposal,
        inputs: &RuntimeEnvironment,
        limits: ExecLimits,
    ) -> Result<Value, InventionError> {
        let checker = TypeChecker::new(10_000);
        let typed = checker
            .check(
                &proposal.definition.body,
                &type_environment(&proposal.definition.parameters),
            )
            .map_err(|_| InventionError::InternalCheck)?;
        ReferenceExecutor::without_definitions(limits, TraceLevel::Off)
            .execute(&typed, inputs)
            .map(|trace| trace.result)
            .map_err(|_| InventionError::InternalExecution)
    }

    /// Admit an externally proposed definition against the complete current
    /// problem. This is the only entry point used by mechanism memory: a
    /// retrieved module remains a proposal and cannot bypass replay.
    ///
    /// # Errors
    ///
    /// Returns exact checking, replay, compilation, or disagreement failures.
    pub fn admit_definition(
        &self,
        problem: &InventionProblem,
        definition: Definition,
        structural_cost: u16,
        proposals_evaluated: u64,
    ) -> Result<InventionOutcome, InventionError> {
        validate_problem(problem)?;
        let bytes = definition
            .encode_canonical()
            .map_err(|_| InventionError::InternalCanonicalization)?;
        admit(
            problem,
            DefinitionProposal {
                definition,
                structural_cost,
                semantic_hash: Hash32::digest(&bytes),
                candidates_evaluated: proposals_evaluated,
                observational_classes: 0,
            },
        )
    }
}

#[derive(Clone)]
struct Candidate {
    expression: Expr,
    ty: Type,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct BehaviorKey {
    kind: TypeKind,
    outputs: Vec<Value>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum TypeKind {
    Bool,
    Int,
    Atom,
    Coord2,
    Tuple(Vec<Self>),
    Collection(Box<Self>),
    Other(Type),
}

struct Search<'a> {
    problem: &'a InventionProblem,
    checker: &'a TypeChecker,
    environment: &'a TypeEnvironment,
    by_cost: Vec<Vec<Candidate>>,
    behaviors: BTreeSet<BehaviorKey>,
    candidates_evaluated: u64,
}

impl Search<'_> {
    fn generate_cost(
        &mut self,
        cost: u16,
        expected: &[Value],
    ) -> Result<Option<DefinitionProposal>, InventionError> {
        for expression in self.tuple_expressions(cost) {
            if let Some(proposal) = self.consider(expression, cost, expected)? {
                return Ok(Some(proposal));
            }
        }

        let unary_cost = cost - 1;
        for index in 0..self.by_cost[usize::from(unary_cost)].len() {
            let candidate = &self.by_cost[usize::from(unary_cost)][index];
            let expressions = match type_kind(&candidate.ty) {
                TypeKind::Int => vec![Expr::Unary {
                    op: UnaryOp::Negate,
                    argument: Box::new(candidate.expression.clone()),
                }],
                TypeKind::Bool => vec![Expr::Unary {
                    op: UnaryOp::Not,
                    argument: Box::new(candidate.expression.clone()),
                }],
                TypeKind::Collection(_) => {
                    vec![Expr::Count(Box::new(candidate.expression.clone()))]
                }
                _ => Vec::new(),
            };
            for expression in expressions {
                if let Some(proposal) = self.consider(expression, cost, expected)? {
                    return Ok(Some(proposal));
                }
            }
        }

        for left_cost in 1..cost - 1 {
            let right_cost = cost - 1 - left_cost;
            let left_candidates = self.by_cost[usize::from(left_cost)]
                .iter()
                .map(|candidate| (candidate.expression.clone(), type_kind(&candidate.ty)))
                .collect::<Vec<_>>();
            let right_candidates = self.by_cost[usize::from(right_cost)]
                .iter()
                .map(|candidate| (candidate.expression.clone(), type_kind(&candidate.ty)))
                .collect::<Vec<_>>();
            for (left, left_kind) in &left_candidates {
                for (right, right_kind) in &right_candidates {
                    for operation in binary_operations(left_kind, right_kind) {
                        let expression = Expr::Binary {
                            op: operation,
                            left: Box::new(left.clone()),
                            right: Box::new(right.clone()),
                        };
                        if let Some(proposal) = self.consider(expression, cost, expected)? {
                            return Ok(Some(proposal));
                        }
                    }
                }
            }
        }

        if cost >= 4 {
            for condition_cost in 1..cost - 2 {
                for then_cost in 1..cost - condition_cost - 1 {
                    let else_cost = cost - 1 - condition_cost - then_cost;
                    let conditions = expressions_of_kind(
                        &self.by_cost[usize::from(condition_cost)],
                        &TypeKind::Bool,
                    );
                    let then_candidates = self.by_cost[usize::from(then_cost)]
                        .iter()
                        .map(|candidate| (candidate.expression.clone(), type_kind(&candidate.ty)))
                        .collect::<Vec<_>>();
                    let else_candidates = self.by_cost[usize::from(else_cost)]
                        .iter()
                        .map(|candidate| (candidate.expression.clone(), type_kind(&candidate.ty)))
                        .collect::<Vec<_>>();
                    for condition in &conditions {
                        for (then_branch, then_kind) in &then_candidates {
                            for (else_branch, else_kind) in &else_candidates {
                                if then_kind != else_kind {
                                    continue;
                                }
                                let expression = Expr::If {
                                    condition: Box::new(condition.clone()),
                                    then_branch: Box::new(then_branch.clone()),
                                    else_branch: Box::new(else_branch.clone()),
                                };
                                if let Some(proposal) = self.consider(expression, cost, expected)? {
                                    return Ok(Some(proposal));
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(None)
    }

    fn tuple_expressions(&self, cost: u16) -> Vec<Expr> {
        let Type::Tuple(component_types) = &self.problem.return_type else {
            return Vec::new();
        };
        let Ok(component_count) = u16::try_from(component_types.len()) else {
            return Vec::new();
        };
        if component_types.is_empty() || cost <= component_count {
            return Vec::new();
        }
        let mut results = Vec::new();
        self.tuple_components(component_types, 0, cost - 1, &mut Vec::new(), &mut results);
        results
    }

    fn tuple_components(
        &self,
        component_types: &[Type],
        index: usize,
        remaining_cost: u16,
        selected: &mut Vec<Expr>,
        results: &mut Vec<Expr>,
    ) {
        if index == component_types.len() {
            if remaining_cost == 0 {
                results.push(Expr::Tuple(selected.clone()));
            }
            return;
        }
        let remaining_components = component_types.len() - index - 1;
        let Ok(remaining_components) = u16::try_from(remaining_components) else {
            return;
        };
        if remaining_cost <= remaining_components {
            return;
        }
        let maximum_component_cost = remaining_cost - remaining_components;
        for component_cost in 1..=maximum_component_cost {
            for candidate in &self.by_cost[usize::from(component_cost)] {
                if !type_compatible(&component_types[index], &candidate.ty) {
                    continue;
                }
                selected.push(candidate.expression.clone());
                self.tuple_components(
                    component_types,
                    index + 1,
                    remaining_cost - component_cost,
                    selected,
                    results,
                );
                selected.pop();
            }
        }
    }

    fn consider(
        &mut self,
        expression: Expr,
        cost: u16,
        expected: &[Value],
    ) -> Result<Option<DefinitionProposal>, InventionError> {
        self.candidates_evaluated = self.candidates_evaluated.saturating_add(1);
        if self.candidates_evaluated > self.problem.limits.maximum_candidates {
            return Err(InventionError::CandidateCapacity {
                maximum: self.problem.limits.maximum_candidates,
            });
        }
        let Ok(typed) = self.checker.check(&expression, self.environment) else {
            return Ok(None);
        };
        let Some(outputs) = evaluate_all(&typed, self.problem) else {
            return Ok(None);
        };
        let key = BehaviorKey {
            kind: type_kind(typed.ty()),
            outputs: outputs.clone(),
        };
        if !self.behaviors.insert(key) {
            return Ok(None);
        }
        if type_compatible(&self.problem.return_type, typed.ty()) && outputs == expected {
            let definition = Definition {
                name: self.problem.definition_name.clone(),
                parameters: self.problem.parameters.clone(),
                return_type: self.problem.return_type.clone(),
                body: expression,
                recursion: None,
                proof_annotations: Vec::new(),
            };
            let bytes = definition
                .encode_canonical()
                .map_err(|_| InventionError::InternalCanonicalization)?;
            return Ok(Some(DefinitionProposal {
                definition,
                structural_cost: cost,
                semantic_hash: Hash32::digest(&bytes),
                candidates_evaluated: self.candidates_evaluated,
                observational_classes: self.behaviors.len() as u64,
            }));
        }
        self.by_cost[usize::from(cost)].push(Candidate {
            expression,
            ty: typed.ty().clone(),
        });
        Ok(None)
    }
}

fn expressions_of_kind(candidates: &[Candidate], kind: &TypeKind) -> Vec<Expr> {
    candidates
        .iter()
        .filter(|candidate| &type_kind(&candidate.ty) == kind)
        .map(|candidate| candidate.expression.clone())
        .collect()
}

fn binary_operations(left: &TypeKind, right: &TypeKind) -> Vec<BinaryOp> {
    match (left, right) {
        (TypeKind::Int, TypeKind::Int) => vec![
            BinaryOp::Minimum,
            BinaryOp::Maximum,
            BinaryOp::Add,
            BinaryOp::Subtract,
            BinaryOp::Multiply,
            BinaryOp::LessThan,
            BinaryOp::Equal,
        ],
        (TypeKind::Bool, TypeKind::Bool) => {
            vec![BinaryOp::And, BinaryOp::Or, BinaryOp::Equal]
        }
        (TypeKind::Atom, TypeKind::Atom) | (TypeKind::Coord2, TypeKind::Coord2) => {
            vec![BinaryOp::Equal]
        }
        _ => Vec::new(),
    }
}

fn evaluate_all(expression: &TypedExpr, problem: &InventionProblem) -> Option<Vec<Value>> {
    let executor =
        ReferenceExecutor::without_definitions(problem.limits.execution, TraceLevel::Off);
    problem
        .demonstrations
        .iter()
        .map(|example| {
            executor
                .execute(expression, &example.inputs)
                .ok()
                .map(|trace| trace.result)
        })
        .collect()
}

fn admit(
    problem: &InventionProblem,
    proposal: DefinitionProposal,
) -> Result<InventionOutcome, InventionError> {
    let checker = TypeChecker::new(problem.limits.maximum_expression_nodes);
    let typed = checker
        .check_definition(&proposal.definition)
        .map_err(|_| InventionError::InternalCheck)?;
    let program = BytecodeCompiler::new(CompileLimits::default())
        .compile(&typed)
        .map_err(|_| InventionError::OptimizedCompilation)?;
    let reference =
        ReferenceExecutor::without_definitions(problem.limits.execution, TraceLevel::Full);
    let optimized = FastExecutor::new(problem.limits.execution, TraceLevel::Full);
    for example in &problem.demonstrations {
        let reference_trace = reference
            .execute(&typed, &example.inputs)
            .map_err(|_| InventionError::InternalExecution)?;
        let optimized_trace = optimized
            .execute(&program, &example.inputs)
            .map_err(|_| InventionError::InternalExecution)?;
        if reference_trace.result != example.output
            || optimized_trace.result != example.output
            || reference_trace != optimized_trace
        {
            return Err(InventionError::VerificationMismatch);
        }
    }
    Ok(InventionOutcome {
        proposal,
        admission: AdmissionReport {
            training_examples: problem.demonstrations.len(),
            exact_reference_replays: problem.demonstrations.len(),
            exact_cross_implementation_replays: problem.demonstrations.len(),
            totality: TotalityStatus::Checked,
        },
    })
}

fn validate_problem(problem: &InventionProblem) -> Result<(), InventionError> {
    if problem.demonstrations.is_empty() {
        return Err(InventionError::EmptyDemonstrations);
    }
    if problem.limits.maximum_cost == 0
        || problem.limits.maximum_candidates == 0
        || problem.limits.maximum_expression_nodes == 0
    {
        return Err(InventionError::EmptyBound);
    }
    let mut names = BTreeSet::new();
    for parameter in &problem.parameters {
        if !names.insert(parameter.name.clone()) {
            return Err(InventionError::DuplicateParameter(parameter.name.0.clone()));
        }
    }
    for example in &problem.demonstrations {
        for parameter in &problem.parameters {
            let value = example
                .inputs
                .get(&parameter.name)
                .ok_or_else(|| InventionError::MissingInput(parameter.name.0.clone()))?;
            if !value.conforms_to(&parameter.ty) {
                return Err(InventionError::InputTypeMismatch(parameter.name.0.clone()));
            }
        }
        if !example.output.conforms_to(&problem.return_type) {
            return Err(InventionError::OutputTypeMismatch);
        }
    }
    Ok(())
}

fn literal_expression(value: &Value) -> Result<Expr, InventionError> {
    match value {
        Value::Bool(value) => Ok(Expr::Bool(*value)),
        Value::Int(value) => Ok(Expr::Int(*value)),
        Value::Atom(value) => Ok(Expr::Atom(*value)),
        Value::Coord2 { row, column } => Ok(Expr::Coord2 {
            row: *row,
            column: *column,
        }),
        _ => Err(InventionError::UnsupportedLiteral),
    }
}

fn type_environment(parameters: &[Binder]) -> TypeEnvironment {
    let mut environment = TypeEnvironment::new();
    for parameter in parameters {
        environment.insert(parameter.name.clone(), parameter.ty.clone());
    }
    environment
}

fn type_kind(ty: &Type) -> TypeKind {
    match ty {
        Type::Bool => TypeKind::Bool,
        Type::Int(_) => TypeKind::Int,
        Type::Atom => TypeKind::Atom,
        Type::Coord2 => TypeKind::Coord2,
        Type::Tuple(types) => TypeKind::Tuple(types.iter().map(type_kind).collect()),
        Type::CellSet { .. } => TypeKind::Collection(Box::new(TypeKind::Coord2)),
        Type::EntitySet { .. } => TypeKind::Collection(Box::new(TypeKind::Other(Type::Entity))),
        Type::List { element, .. } => TypeKind::Collection(Box::new(type_kind(element))),
        other => TypeKind::Other(other.clone()),
    }
}

fn type_compatible(expected: &Type, actual: &Type) -> bool {
    match (expected, actual) {
        (Type::Int(_), Type::Int(_))
        | (Type::CellSet { .. }, Type::CellSet { .. })
        | (Type::EntitySet { .. }, Type::EntitySet { .. }) => true,
        (Type::List { element: left, .. }, Type::List { element: right, .. }) => {
            type_compatible(left, right)
        }
        (Type::Tuple(left), Type::Tuple(right)) => {
            left.len() == right.len()
                && left
                    .iter()
                    .zip(right)
                    .all(|(left, right)| type_compatible(left, right))
        }
        _ => expected == actual,
    }
}
