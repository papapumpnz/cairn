use crate::{BinaryOp, CheckedTheory, Definition, Expr, Type, TypedExpr, UnaryOp, VarName};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

/// Version of the normative reference-execution semantics.
pub const RKL_EXECUTION_SEMANTICS_VERSION: &str = "1.0.0";

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum HandleKind {
    Shape,
    Entity,
    Symbol,
    Action,
    State,
}

/// Exact runtime value for the executable WP05 reference subset.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Value {
    Bool(bool),
    Int(i64),
    Atom(u16),
    Coord2 {
        row: i32,
        column: i32,
    },
    Tuple(Vec<Self>),
    Set {
        element_type: Type,
        maximum: u32,
        values: BTreeSet<Self>,
    },
    Handle {
        kind: HandleKind,
        id: u32,
    },
    Unit,
}

impl Value {
    #[must_use]
    pub fn conforms_to(&self, ty: &Type) -> bool {
        match (self, ty) {
            (Self::Bool(_), Type::Bool)
            | (Self::Atom(_), Type::Atom)
            | (Self::Coord2 { .. }, Type::Coord2)
            | (Self::Unit, Type::Unit) => true,
            (Self::Int(value), Type::Int(range)) => range.lower <= *value && *value <= range.upper,
            (Self::Tuple(values), Type::Tuple(types)) => {
                values.len() == types.len()
                    && values
                        .iter()
                        .zip(types)
                        .all(|(value, ty)| value.conforms_to(ty))
            }
            (
                Self::Set {
                    element_type,
                    maximum,
                    values,
                },
                Type::CellSet { maximum: expected },
            ) => {
                element_type == &Type::Coord2
                    && maximum <= expected
                    && values.iter().all(|value| value.conforms_to(element_type))
            }
            (
                Self::Set {
                    element_type,
                    maximum,
                    values,
                },
                Type::EntitySet { maximum: expected },
            ) => {
                element_type == &Type::Entity
                    && maximum <= expected
                    && values.iter().all(|value| value.conforms_to(element_type))
            }
            (
                Self::Set {
                    element_type,
                    maximum,
                    values,
                },
                Type::List {
                    element,
                    maximum: expected,
                },
            ) => {
                element_type == element.as_ref()
                    && maximum <= expected
                    && values.iter().all(|value| value.conforms_to(element_type))
            }
            (Self::Handle { kind, .. }, expected) => handle_type(*kind) == *expected,
            _ => false,
        }
    }
}

const fn handle_type(kind: HandleKind) -> Type {
    match kind {
        HandleKind::Shape => Type::Shape,
        HandleKind::Entity => Type::Entity,
        HandleKind::Symbol => Type::Symbol,
        HandleKind::Action => Type::Action,
        HandleKind::State => Type::State,
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RuntimeEnvironment(BTreeMap<VarName, Value>);

impl RuntimeEnvironment {
    #[must_use]
    pub const fn new() -> Self {
        Self(BTreeMap::new())
    }

    pub fn insert(&mut self, name: VarName, value: Value) -> Option<Value> {
        self.0.insert(name, value)
    }

    #[must_use]
    pub fn get(&self, name: &VarName) -> Option<&Value> {
        self.0.get(name)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ExecLimits {
    pub fuel: u64,
    pub maximum_call_depth: u16,
    pub maximum_trace_events: u32,
}

impl Default for ExecLimits {
    fn default() -> Self {
        Self {
            fuel: 1_000_000,
            maximum_call_depth: 256,
            maximum_trace_events: 100_000,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TraceLevel {
    Off,
    Summary,
    Full,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraceEvent {
    pub step: u64,
    pub operation: &'static str,
    pub call_depth: u16,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutionTrace {
    pub result: Value,
    pub steps: u64,
    pub maximum_call_depth: u16,
    pub events: Vec<TraceEvent>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExecError {
    FuelExhausted { consumed: u64, limit: u64 },
    TraceCapacity { maximum: u32 },
    CallDepth { maximum: u16 },
    RecursionBound { definition: String, maximum: u32 },
    UnknownVariable(String),
    UnknownDefinition(String),
    Arity { expected: usize, actual: usize },
    TypeFault { operation: &'static str },
    ArithmeticOverflow,
    CollectionBound { requested: usize, maximum: u32 },
    NonDecreasingProgress { previous: i64, next: i64 },
    ResultTypeMismatch,
    UnsupportedExpression(&'static str),
}

impl fmt::Display for ExecError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for ExecError {}

/// Deliberately direct AST interpreter used as the semantic reference.
pub struct ReferenceExecutor<'a> {
    definitions: BTreeMap<&'a str, &'a Definition>,
    limits: ExecLimits,
    trace_level: TraceLevel,
}

impl<'a> ReferenceExecutor<'a> {
    #[must_use]
    pub fn new(theory: &'a CheckedTheory, limits: ExecLimits, trace_level: TraceLevel) -> Self {
        Self {
            definitions: theory
                .theory()
                .definitions
                .iter()
                .map(|definition| (definition.name.as_str(), definition))
                .collect(),
            limits,
            trace_level,
        }
    }

    #[must_use]
    pub fn without_definitions(limits: ExecLimits, trace_level: TraceLevel) -> Self {
        Self {
            definitions: BTreeMap::new(),
            limits,
            trace_level,
        }
    }

    /// Execute a checked expression under exact limits.
    ///
    /// # Errors
    ///
    /// Returns explicit fuel, trace, call, type, bound, or arithmetic failures.
    pub fn execute(
        &self,
        expression: &TypedExpr,
        environment: &RuntimeEnvironment,
    ) -> Result<ExecutionTrace, ExecError> {
        for variable in expression.free_variables() {
            if environment.get(variable).is_none() {
                return Err(ExecError::UnknownVariable(variable.0.clone()));
            }
        }
        let mut context = Context {
            executor: self,
            environment: environment.clone(),
            fuel_remaining: self.limits.fuel,
            steps: 0,
            call_depth: 0,
            maximum_call_depth: 0,
            events: Vec::new(),
            recursion_counts: BTreeMap::new(),
        };
        let result = context.evaluate(expression.expression())?;
        if !result.conforms_to(expression.ty()) {
            return Err(ExecError::ResultTypeMismatch);
        }
        Ok(ExecutionTrace {
            result,
            steps: context.steps,
            maximum_call_depth: context.maximum_call_depth,
            events: context.events,
        })
    }
}

struct Context<'executor, 'theory> {
    executor: &'executor ReferenceExecutor<'theory>,
    environment: RuntimeEnvironment,
    fuel_remaining: u64,
    steps: u64,
    call_depth: u16,
    maximum_call_depth: u16,
    events: Vec<TraceEvent>,
    recursion_counts: BTreeMap<String, u32>,
}

impl Context<'_, '_> {
    #[allow(clippy::too_many_lines)]
    fn evaluate(&mut self, expression: &Expr) -> Result<Value, ExecError> {
        self.consume(operation_name(expression))?;
        match expression {
            Expr::Bool(value) => Ok(Value::Bool(*value)),
            Expr::Int(value) => Ok(Value::Int(*value)),
            Expr::Atom(value) => Ok(Value::Atom(*value)),
            Expr::Coord2 { row, column } => Ok(Value::Coord2 {
                row: *row,
                column: *column,
            }),
            Expr::Var(name) => self
                .environment
                .get(name)
                .cloned()
                .ok_or_else(|| ExecError::UnknownVariable(name.0.clone())),
            Expr::Tuple(values) => values
                .iter()
                .map(|value| self.evaluate(value))
                .collect::<Result<Vec<_>, _>>()
                .map(Value::Tuple),
            Expr::Let {
                binder,
                value,
                body,
            } => {
                let value = self.evaluate(value)?;
                let previous = self.environment.insert(binder.clone(), value);
                let result = self.evaluate(body);
                restore(&mut self.environment, binder, previous);
                result
            }
            Expr::If {
                condition,
                then_branch,
                else_branch,
            } => match self.evaluate(condition)? {
                Value::Bool(true) => self.evaluate(then_branch),
                Value::Bool(false) => self.evaluate(else_branch),
                _ => Err(ExecError::TypeFault { operation: "if" }),
            },
            Expr::Unary { op, argument } => {
                let value = self.evaluate(argument)?;
                unary(*op, value)
            }
            Expr::Binary { op, left, right } => {
                let left = self.evaluate(left)?;
                let right = self.evaluate(right)?;
                binary(*op, left, right)
            }
            Expr::EmptySet { element, maximum } => Ok(Value::Set {
                element_type: element.clone(),
                maximum: *maximum,
                values: BTreeSet::new(),
            }),
            Expr::SingletonSet { value, maximum } => {
                let value = self.evaluate(value)?;
                Ok(Value::Set {
                    element_type: value_type(&value),
                    maximum: *maximum,
                    values: BTreeSet::from([value]),
                })
            }
            Expr::Count(value) => match self.evaluate(value)? {
                Value::Set { values, .. } => i64::try_from(values.len())
                    .map(Value::Int)
                    .map_err(|_| ExecError::ArithmeticOverflow),
                _ => Err(ExecError::TypeFault { operation: "count" }),
            },
            Expr::Call {
                function,
                arguments,
            } => self.call(function, arguments),
            Expr::BoundedFixpoint {
                binder,
                initial,
                step,
                maximum_steps,
                progress_measure,
            } => self.fixpoint(binder, initial, step, *maximum_steps, progress_measure),
        }
    }

    fn call(&mut self, name: &str, arguments: &[Expr]) -> Result<Value, ExecError> {
        let definition = self
            .executor
            .definitions
            .get(name)
            .copied()
            .ok_or_else(|| ExecError::UnknownDefinition(name.to_owned()))?;
        if definition.parameters.len() != arguments.len() {
            return Err(ExecError::Arity {
                expected: definition.parameters.len(),
                actual: arguments.len(),
            });
        }
        if self.call_depth >= self.executor.limits.maximum_call_depth {
            return Err(ExecError::CallDepth {
                maximum: self.executor.limits.maximum_call_depth,
            });
        }
        let values = arguments
            .iter()
            .map(|argument| self.evaluate(argument))
            .collect::<Result<Vec<_>, _>>()?;
        if definition
            .parameters
            .iter()
            .zip(&values)
            .any(|(parameter, value)| !value.conforms_to(&parameter.ty))
        {
            return Err(ExecError::TypeFault {
                operation: "call argument",
            });
        }
        let count = self.recursion_counts.entry(name.to_owned()).or_default();
        *count = count.saturating_add(1);
        if let Some(bound) = &definition.recursion {
            if *count > bound.maximum_steps {
                *count -= 1;
                return Err(ExecError::RecursionBound {
                    definition: name.to_owned(),
                    maximum: bound.maximum_steps,
                });
            }
        }
        self.call_depth += 1;
        self.maximum_call_depth = self.maximum_call_depth.max(self.call_depth);
        let mut previous = Vec::with_capacity(values.len());
        for (parameter, value) in definition.parameters.iter().zip(values) {
            previous.push((
                parameter.name.clone(),
                self.environment.insert(parameter.name.clone(), value),
            ));
        }
        let result = self.evaluate(&definition.body);
        for (name, value) in previous.into_iter().rev() {
            restore(&mut self.environment, &name, value);
        }
        self.call_depth -= 1;
        *self
            .recursion_counts
            .get_mut(name)
            .expect("recursion counter") -= 1;
        let result = result?;
        if result.conforms_to(&definition.return_type) {
            Ok(result)
        } else {
            Err(ExecError::ResultTypeMismatch)
        }
    }

    fn fixpoint(
        &mut self,
        binder: &VarName,
        initial: &Expr,
        step: &Expr,
        maximum_steps: u32,
        progress_measure: &Expr,
    ) -> Result<Value, ExecError> {
        let mut current = self.evaluate(initial)?;
        let previous_binding = self.environment.insert(binder.clone(), current.clone());
        let mut progress = integer(&self.evaluate(progress_measure)?, "fixpoint progress")?;
        for _ in 0..maximum_steps {
            self.environment.insert(binder.clone(), current.clone());
            let next = self.evaluate(step)?;
            if next == current {
                restore(&mut self.environment, binder, previous_binding);
                return Ok(current);
            }
            self.environment.insert(binder.clone(), next.clone());
            let next_progress = integer(&self.evaluate(progress_measure)?, "fixpoint progress")?;
            if next_progress >= progress {
                restore(&mut self.environment, binder, previous_binding);
                return Err(ExecError::NonDecreasingProgress {
                    previous: progress,
                    next: next_progress,
                });
            }
            current = next;
            progress = next_progress;
        }
        restore(&mut self.environment, binder, previous_binding);
        Ok(current)
    }

    fn consume(&mut self, operation: &'static str) -> Result<(), ExecError> {
        if self.fuel_remaining == 0 {
            return Err(ExecError::FuelExhausted {
                consumed: self.steps,
                limit: self.executor.limits.fuel,
            });
        }
        self.fuel_remaining -= 1;
        self.steps += 1;
        if self.executor.trace_level == TraceLevel::Full {
            if self.events.len() >= self.executor.limits.maximum_trace_events as usize {
                return Err(ExecError::TraceCapacity {
                    maximum: self.executor.limits.maximum_trace_events,
                });
            }
            self.events.push(TraceEvent {
                step: self.steps,
                operation,
                call_depth: self.call_depth,
            });
        }
        Ok(())
    }
}

fn restore(environment: &mut RuntimeEnvironment, name: &VarName, previous: Option<Value>) {
    if let Some(previous) = previous {
        environment.insert(name.clone(), previous);
    } else {
        environment.0.remove(name);
    }
}

fn unary(operation: UnaryOp, value: Value) -> Result<Value, ExecError> {
    match (operation, value) {
        (UnaryOp::Not, Value::Bool(value)) => Ok(Value::Bool(!value)),
        (UnaryOp::Negate, Value::Int(value)) => value
            .checked_neg()
            .map(Value::Int)
            .ok_or(ExecError::ArithmeticOverflow),
        _ => Err(ExecError::TypeFault { operation: "unary" }),
    }
}

#[allow(clippy::too_many_lines)]
fn binary(operation: BinaryOp, left: Value, right: Value) -> Result<Value, ExecError> {
    match operation {
        BinaryOp::Add => integer_pair(&left, &right, "add", i64::checked_add),
        BinaryOp::Subtract => integer_pair(&left, &right, "subtract", i64::checked_sub),
        BinaryOp::Multiply => integer_pair(&left, &right, "multiply", i64::checked_mul),
        BinaryOp::Minimum => integer_pair(&left, &right, "minimum", |left, right| {
            Some(left.min(right))
        }),
        BinaryOp::Maximum => integer_pair(&left, &right, "maximum", |left, right| {
            Some(left.max(right))
        }),
        BinaryOp::And => bool_pair(left, right, "and", |left, right| left && right),
        BinaryOp::Or => bool_pair(left, right, "or", |left, right| left || right),
        BinaryOp::Equal => Ok(Value::Bool(left == right)),
        BinaryOp::LessThan => Ok(Value::Bool(
            integer(&left, "less_than")? < integer(&right, "less_than")?,
        )),
        BinaryOp::Union | BinaryOp::Intersection | BinaryOp::Difference => {
            set_binary(operation, left, right)
        }
    }
}

fn integer_pair(
    left: &Value,
    right: &Value,
    operation: &'static str,
    function: impl FnOnce(i64, i64) -> Option<i64>,
) -> Result<Value, ExecError> {
    function(integer(left, operation)?, integer(right, operation)?)
        .map(Value::Int)
        .ok_or(ExecError::ArithmeticOverflow)
}

fn bool_pair(
    left: Value,
    right: Value,
    operation: &'static str,
    function: impl FnOnce(bool, bool) -> bool,
) -> Result<Value, ExecError> {
    let (Value::Bool(left), Value::Bool(right)) = (left, right) else {
        return Err(ExecError::TypeFault { operation });
    };
    Ok(Value::Bool(function(left, right)))
}

fn integer(value: &Value, operation: &'static str) -> Result<i64, ExecError> {
    if let Value::Int(value) = value {
        Ok(*value)
    } else {
        Err(ExecError::TypeFault { operation })
    }
}

fn set_binary(operation: BinaryOp, left: Value, right: Value) -> Result<Value, ExecError> {
    let (
        Value::Set {
            element_type,
            maximum,
            values: left,
        },
        Value::Set {
            element_type: right_type,
            maximum: right_maximum,
            values: right,
        },
    ) = (left, right)
    else {
        return Err(ExecError::TypeFault { operation: "set" });
    };
    if element_type != right_type {
        return Err(ExecError::TypeFault {
            operation: "set element",
        });
    }
    let maximum = match operation {
        BinaryOp::Union => maximum
            .checked_add(right_maximum)
            .ok_or(ExecError::ArithmeticOverflow)?,
        BinaryOp::Intersection => maximum.min(right_maximum),
        BinaryOp::Difference => maximum,
        _ => unreachable!("set operation"),
    };
    let values: BTreeSet<Value> = match operation {
        BinaryOp::Union => left.union(&right).cloned().collect(),
        BinaryOp::Intersection => left.intersection(&right).cloned().collect(),
        BinaryOp::Difference => left.difference(&right).cloned().collect(),
        _ => unreachable!("set operation"),
    };
    if values.len() > maximum as usize {
        return Err(ExecError::CollectionBound {
            requested: values.len(),
            maximum,
        });
    }
    Ok(Value::Set {
        element_type,
        maximum,
        values,
    })
}

fn value_type(value: &Value) -> Type {
    match value {
        Value::Bool(_) => Type::Bool,
        Value::Int(value) => Type::Int(crate::IntRange::exact(*value)),
        Value::Atom(_) => Type::Atom,
        Value::Coord2 { .. } => Type::Coord2,
        Value::Tuple(values) => Type::Tuple(values.iter().map(value_type).collect()),
        Value::Set { element_type, .. } => element_type.clone(),
        Value::Handle { kind, .. } => handle_type(*kind),
        Value::Unit => Type::Unit,
    }
}

const fn operation_name(expression: &Expr) -> &'static str {
    match expression {
        Expr::Bool(_) => "bool",
        Expr::Int(_) => "int",
        Expr::Atom(_) => "atom",
        Expr::Coord2 { .. } => "coord2",
        Expr::Var(_) => "var",
        Expr::Tuple(_) => "tuple",
        Expr::Let { .. } => "let",
        Expr::If { .. } => "if",
        Expr::Unary { .. } => "unary",
        Expr::Binary { .. } => "binary",
        Expr::EmptySet { .. } => "empty_set",
        Expr::SingletonSet { .. } => "singleton_set",
        Expr::Count(_) => "count",
        Expr::Call { .. } => "call",
        Expr::BoundedFixpoint { .. } => "bounded_fixpoint",
    }
}
