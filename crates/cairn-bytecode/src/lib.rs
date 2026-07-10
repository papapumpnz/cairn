//! Typed, deterministic RKL bytecode compiler and executor.

use cairn_rkl::{
    BinaryOp, CheckedTheory, Definition, ExecError, ExecLimits, ExecutionTrace, Expr, TraceEvent,
    TraceLevel, Type, TypedExpr, UnaryOp, Value, VarName,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

pub const BYTECODE_SEMANTICS_VERSION: &str = "1.0.0";

type Register = usize;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CompileError {
    UnknownDefinition(String),
    RecursiveDefinition(String),
    UnsupportedExpression(&'static str),
    RegisterCapacity,
    InstructionCapacity,
}

impl fmt::Display for CompileError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for CompileError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CompileLimits {
    pub maximum_registers: usize,
    pub maximum_instructions: usize,
}

impl Default for CompileLimits {
    fn default() -> Self {
        Self {
            maximum_registers: 65_536,
            maximum_instructions: 1_000_000,
        }
    }
}

/// Immutable, compiler-validated register program.
#[derive(Clone, Debug)]
pub struct BytecodeProgram {
    instructions: Vec<Instruction>,
    register_count: usize,
    loop_count: usize,
    result: Register,
    result_type: Type,
}

impl BytecodeProgram {
    #[must_use]
    pub const fn register_count(&self) -> usize {
        self.register_count
    }

    #[must_use]
    pub fn instruction_count(&self) -> usize {
        self.instructions.len()
    }

    #[must_use]
    pub const fn result_type(&self) -> &Type {
        &self.result_type
    }
}

#[derive(Clone, Debug)]
enum Instruction {
    Enter(&'static str),
    LoadConstant {
        destination: Register,
        value: Value,
    },
    LoadInput {
        destination: Register,
        name: VarName,
    },
    Unary {
        destination: Register,
        operation: UnaryOp,
        argument: Register,
    },
    Binary {
        destination: Register,
        operation: BinaryOp,
        left: Register,
        right: Register,
    },
    Tuple {
        destination: Register,
        values: Vec<Register>,
    },
    EmptySet {
        destination: Register,
        element: Type,
        maximum: u32,
    },
    SingletonSet {
        destination: Register,
        value: Register,
        maximum: u32,
    },
    Count {
        destination: Register,
        value: Register,
    },
    Move {
        destination: Register,
        source: Register,
    },
    JumpIfFalse {
        condition: Register,
        target: usize,
    },
    Jump {
        target: usize,
    },
    CheckCallDepth,
    EnterCall,
    ExitCall,
    AssertType {
        value: Register,
        expected: Type,
        operation: &'static str,
    },
    InitializeLoop {
        loop_id: usize,
    },
    JumpIfEqual {
        left: Register,
        right: Register,
        target: usize,
    },
    RequireDecreasing {
        previous: Register,
        next: Register,
    },
    ContinueLoop {
        loop_id: usize,
        maximum: u32,
        target: usize,
    },
}

/// Compiles checked RKL into allocation-free register micro-ops.
pub struct BytecodeCompiler<'a> {
    definitions: BTreeMap<&'a str, &'a Definition>,
    limits: CompileLimits,
}

impl BytecodeCompiler<'static> {
    #[must_use]
    pub fn new(limits: CompileLimits) -> Self {
        Self {
            definitions: BTreeMap::new(),
            limits,
        }
    }
}

impl<'a> BytecodeCompiler<'a> {
    #[must_use]
    pub fn with_theory(theory: &'a CheckedTheory, limits: CompileLimits) -> Self {
        Self {
            definitions: theory
                .theory()
                .definitions
                .iter()
                .map(|definition| (definition.name.as_str(), definition))
                .collect(),
            limits,
        }
    }

    /// Compile a checked expression. Recursive definitions remain on the
    /// reference path until their decreasing-recursion opcode is available.
    ///
    /// # Errors
    ///
    /// Returns a typed capacity, definition, recursion, or feature error.
    pub fn compile(&self, expression: &TypedExpr) -> Result<BytecodeProgram, CompileError> {
        let mut context = CompilerContext {
            compiler: self,
            instructions: Vec::new(),
            next_register: 0,
            next_loop: 0,
            locals: BTreeMap::new(),
            active_calls: BTreeSet::new(),
        };
        let result = context.expression(expression.expression())?;
        Ok(BytecodeProgram {
            instructions: context.instructions,
            register_count: context.next_register,
            loop_count: context.next_loop,
            result,
            result_type: expression.ty().clone(),
        })
    }
}

struct CompilerContext<'compiler, 'theory> {
    compiler: &'compiler BytecodeCompiler<'theory>,
    instructions: Vec<Instruction>,
    next_register: usize,
    next_loop: usize,
    locals: BTreeMap<VarName, Register>,
    active_calls: BTreeSet<String>,
}

impl CompilerContext<'_, '_> {
    #[allow(clippy::too_many_lines)]
    fn expression(&mut self, expression: &Expr) -> Result<Register, CompileError> {
        self.emit(Instruction::Enter(operation_name(expression)))?;
        match expression {
            Expr::Bool(value) => self.constant(Value::Bool(*value)),
            Expr::Int(value) => self.constant(Value::Int(*value)),
            Expr::Atom(value) => self.constant(Value::Atom(*value)),
            Expr::Coord2 { row, column } => self.constant(Value::Coord2 {
                row: *row,
                column: *column,
            }),
            Expr::Var(name) => {
                if let Some(register) = self.locals.get(name) {
                    Ok(*register)
                } else {
                    let destination = self.register()?;
                    self.emit(Instruction::LoadInput {
                        destination,
                        name: name.clone(),
                    })?;
                    Ok(destination)
                }
            }
            Expr::Tuple(values) => {
                let values = values
                    .iter()
                    .map(|value| self.expression(value))
                    .collect::<Result<Vec<_>, _>>()?;
                let destination = self.register()?;
                self.emit(Instruction::Tuple {
                    destination,
                    values,
                })?;
                Ok(destination)
            }
            Expr::Let {
                binder,
                value,
                body,
            } => {
                let value = self.expression(value)?;
                let previous = self.locals.insert(binder.clone(), value);
                let result = self.expression(body);
                restore_local(&mut self.locals, binder, previous);
                result
            }
            Expr::If {
                condition,
                then_branch,
                else_branch,
            } => self.if_expression(condition, then_branch, else_branch),
            Expr::Unary { op, argument } => {
                let argument = self.expression(argument)?;
                let destination = self.register()?;
                self.emit(Instruction::Unary {
                    destination,
                    operation: *op,
                    argument,
                })?;
                Ok(destination)
            }
            Expr::Binary { op, left, right } => {
                let left = self.expression(left)?;
                let right = self.expression(right)?;
                let destination = self.register()?;
                self.emit(Instruction::Binary {
                    destination,
                    operation: *op,
                    left,
                    right,
                })?;
                Ok(destination)
            }
            Expr::EmptySet { element, maximum } => {
                let destination = self.register()?;
                self.emit(Instruction::EmptySet {
                    destination,
                    element: element.clone(),
                    maximum: *maximum,
                })?;
                Ok(destination)
            }
            Expr::SingletonSet { value, maximum } => {
                let value = self.expression(value)?;
                let destination = self.register()?;
                self.emit(Instruction::SingletonSet {
                    destination,
                    value,
                    maximum: *maximum,
                })?;
                Ok(destination)
            }
            Expr::Count(value) => {
                let value = self.expression(value)?;
                let destination = self.register()?;
                self.emit(Instruction::Count { destination, value })?;
                Ok(destination)
            }
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

    fn if_expression(
        &mut self,
        condition: &Expr,
        then_branch: &Expr,
        else_branch: &Expr,
    ) -> Result<Register, CompileError> {
        let condition = self.expression(condition)?;
        let destination = self.register()?;
        let conditional = self.instructions.len();
        self.emit(Instruction::JumpIfFalse {
            condition,
            target: usize::MAX,
        })?;
        let then_value = self.expression(then_branch)?;
        self.emit(Instruction::Move {
            destination,
            source: then_value,
        })?;
        let jump = self.instructions.len();
        self.emit(Instruction::Jump { target: usize::MAX })?;
        let else_target = self.instructions.len();
        let else_value = self.expression(else_branch)?;
        self.emit(Instruction::Move {
            destination,
            source: else_value,
        })?;
        let end = self.instructions.len();
        self.patch_target(conditional, else_target);
        self.patch_target(jump, end);
        Ok(destination)
    }

    fn call(&mut self, name: &str, arguments: &[Expr]) -> Result<Register, CompileError> {
        let definition = self
            .compiler
            .definitions
            .get(name)
            .copied()
            .ok_or_else(|| CompileError::UnknownDefinition(name.to_owned()))?;
        if definition.recursion.is_some() || self.active_calls.contains(name) {
            return Err(CompileError::RecursiveDefinition(name.to_owned()));
        }
        self.emit(Instruction::CheckCallDepth)?;
        let arguments = arguments
            .iter()
            .map(|argument| self.expression(argument))
            .collect::<Result<Vec<_>, _>>()?;
        for (argument, parameter) in arguments.iter().zip(&definition.parameters) {
            self.emit(Instruction::AssertType {
                value: *argument,
                expected: parameter.ty.clone(),
                operation: "call argument",
            })?;
        }
        self.emit(Instruction::EnterCall)?;
        self.active_calls.insert(name.to_owned());
        let previous_locals = self.locals.clone();
        for (parameter, argument) in definition.parameters.iter().zip(arguments) {
            self.locals.insert(parameter.name.clone(), argument);
        }
        let result = self.expression(&definition.body);
        self.locals = previous_locals;
        self.active_calls.remove(name);
        let result = result?;
        self.emit(Instruction::AssertType {
            value: result,
            expected: definition.return_type.clone(),
            operation: "call result",
        })?;
        self.emit(Instruction::ExitCall)?;
        Ok(result)
    }

    fn fixpoint(
        &mut self,
        binder: &VarName,
        initial: &Expr,
        step: &Expr,
        maximum_steps: u32,
        progress_measure: &Expr,
    ) -> Result<Register, CompileError> {
        let initial = self.expression(initial)?;
        let current = self.register()?;
        self.emit(Instruction::Move {
            destination: current,
            source: initial,
        })?;
        let previous_local = self.locals.insert(binder.clone(), current);
        let initial_progress = self.expression(progress_measure)?;
        let progress = self.register()?;
        self.emit(Instruction::Move {
            destination: progress,
            source: initial_progress,
        })?;
        let loop_id = self.next_loop;
        self.next_loop = self
            .next_loop
            .checked_add(1)
            .ok_or(CompileError::InstructionCapacity)?;
        self.emit(Instruction::InitializeLoop { loop_id })?;
        let loop_start = self.instructions.len();
        self.locals.insert(binder.clone(), current);
        let next = self.expression(step)?;
        let stable_jump = self.instructions.len();
        self.emit(Instruction::JumpIfEqual {
            left: current,
            right: next,
            target: usize::MAX,
        })?;
        self.locals.insert(binder.clone(), next);
        let next_progress = self.expression(progress_measure)?;
        self.emit(Instruction::RequireDecreasing {
            previous: progress,
            next: next_progress,
        })?;
        self.emit(Instruction::Move {
            destination: current,
            source: next,
        })?;
        self.emit(Instruction::Move {
            destination: progress,
            source: next_progress,
        })?;
        self.emit(Instruction::ContinueLoop {
            loop_id,
            maximum: maximum_steps,
            target: loop_start,
        })?;
        let end = self.instructions.len();
        self.patch_target(stable_jump, end);
        restore_local(&mut self.locals, binder, previous_local);
        Ok(current)
    }

    fn constant(&mut self, value: Value) -> Result<Register, CompileError> {
        let destination = self.register()?;
        self.emit(Instruction::LoadConstant { destination, value })?;
        Ok(destination)
    }

    fn register(&mut self) -> Result<Register, CompileError> {
        if self.next_register >= self.compiler.limits.maximum_registers {
            return Err(CompileError::RegisterCapacity);
        }
        let register = self.next_register;
        self.next_register += 1;
        Ok(register)
    }

    fn emit(&mut self, instruction: Instruction) -> Result<(), CompileError> {
        if self.instructions.len() >= self.compiler.limits.maximum_instructions {
            return Err(CompileError::InstructionCapacity);
        }
        self.instructions.push(instruction);
        Ok(())
    }

    fn patch_target(&mut self, instruction: usize, target: usize) {
        match &mut self.instructions[instruction] {
            Instruction::JumpIfFalse {
                target: current, ..
            }
            | Instruction::Jump { target: current }
            | Instruction::JumpIfEqual {
                target: current, ..
            } => *current = target,
            _ => unreachable!("compiler only patches jumps"),
        }
    }
}

fn restore_local(
    locals: &mut BTreeMap<VarName, Register>,
    name: &VarName,
    previous: Option<Register>,
) {
    if let Some(previous) = previous {
        locals.insert(name.clone(), previous);
    } else {
        locals.remove(name);
    }
}

/// Reusable preallocated register storage for hot-loop execution.
#[derive(Clone, Debug, Default)]
pub struct ExecutionWorkspace {
    registers: Vec<Option<Value>>,
    loop_counters: Vec<u32>,
}

impl ExecutionWorkspace {
    #[must_use]
    pub fn for_program(program: &BytecodeProgram) -> Self {
        Self {
            registers: vec![None; program.register_count],
            loop_counters: vec![0; program.loop_count],
        }
    }

    fn prepare(&mut self, registers: usize, loops: usize) {
        if self.registers.len() < registers {
            self.registers.resize(registers, None);
        }
        self.registers[..registers].fill(None);
        if self.loop_counters.len() < loops {
            self.loop_counters.resize(loops, 0);
        }
        self.loop_counters[..loops].fill(0);
    }
}

pub struct FastExecutor {
    limits: ExecLimits,
    trace_level: TraceLevel,
}

impl FastExecutor {
    #[must_use]
    pub const fn new(limits: ExecLimits, trace_level: TraceLevel) -> Self {
        Self {
            limits,
            trace_level,
        }
    }

    /// Execute with a temporary register workspace.
    ///
    /// # Errors
    ///
    /// Returns stable RKL execution failures.
    pub fn execute(
        &self,
        program: &BytecodeProgram,
        environment: &cairn_rkl::RuntimeEnvironment,
    ) -> Result<ExecutionTrace, ExecError> {
        let mut workspace = ExecutionWorkspace::for_program(program);
        self.execute_in(program, environment, &mut workspace)
    }

    /// Execute using caller-owned storage that can be reused across candidates.
    ///
    /// # Errors
    ///
    /// Returns stable RKL execution failures.
    pub fn execute_in(
        &self,
        program: &BytecodeProgram,
        environment: &cairn_rkl::RuntimeEnvironment,
        workspace: &mut ExecutionWorkspace,
    ) -> Result<ExecutionTrace, ExecError> {
        workspace.prepare(program.register_count, program.loop_count);
        let mut machine = Machine {
            executor: self,
            program,
            environment,
            registers: &mut workspace.registers,
            loop_counters: &mut workspace.loop_counters,
            fuel_remaining: self.limits.fuel,
            steps: 0,
            call_depth: 0,
            maximum_call_depth: 0,
            events: Vec::new(),
        };
        machine.run()
    }
}

struct Machine<'a> {
    executor: &'a FastExecutor,
    program: &'a BytecodeProgram,
    environment: &'a cairn_rkl::RuntimeEnvironment,
    registers: &'a mut [Option<Value>],
    loop_counters: &'a mut [u32],
    fuel_remaining: u64,
    steps: u64,
    call_depth: u16,
    maximum_call_depth: u16,
    events: Vec<TraceEvent>,
}

impl Machine<'_> {
    #[allow(clippy::too_many_lines)]
    fn run(&mut self) -> Result<ExecutionTrace, ExecError> {
        let mut program_counter = 0;
        while let Some(instruction) = self.program.instructions.get(program_counter) {
            program_counter += 1;
            match instruction {
                Instruction::Enter(operation) => self.consume(operation)?,
                Instruction::LoadConstant { destination, value } => {
                    self.write(*destination, value.clone());
                }
                Instruction::LoadInput { destination, name } => {
                    let value = self
                        .environment
                        .get(name)
                        .cloned()
                        .ok_or_else(|| ExecError::UnknownVariable(name.0.clone()))?;
                    self.write(*destination, value);
                }
                Instruction::Unary {
                    destination,
                    operation,
                    argument,
                } => {
                    let value = unary(*operation, self.read(*argument)?.clone())?;
                    self.write(*destination, value);
                }
                Instruction::Binary {
                    destination,
                    operation,
                    left,
                    right,
                } => {
                    let value = binary(
                        *operation,
                        self.read(*left)?.clone(),
                        self.read(*right)?.clone(),
                    )?;
                    self.write(*destination, value);
                }
                Instruction::Tuple {
                    destination,
                    values,
                } => {
                    let values = values
                        .iter()
                        .map(|register| self.read(*register).cloned())
                        .collect::<Result<Vec<_>, _>>()?;
                    self.write(*destination, Value::Tuple(values));
                }
                Instruction::EmptySet {
                    destination,
                    element,
                    maximum,
                } => self.write(
                    *destination,
                    Value::Set {
                        element_type: element.clone(),
                        maximum: *maximum,
                        values: BTreeSet::new(),
                    },
                ),
                Instruction::SingletonSet {
                    destination,
                    value,
                    maximum,
                } => {
                    let value = self.read(*value)?.clone();
                    self.write(
                        *destination,
                        Value::Set {
                            element_type: value_type(&value),
                            maximum: *maximum,
                            values: BTreeSet::from([value]),
                        },
                    );
                }
                Instruction::Count { destination, value } => {
                    let Value::Set { values, .. } = self.read(*value)? else {
                        return Err(ExecError::TypeFault { operation: "count" });
                    };
                    let count =
                        i64::try_from(values.len()).map_err(|_| ExecError::ArithmeticOverflow)?;
                    self.write(*destination, Value::Int(count));
                }
                Instruction::Move {
                    destination,
                    source,
                } => {
                    let value = self.read(*source)?.clone();
                    self.write(*destination, value);
                }
                Instruction::JumpIfFalse { condition, target } => {
                    if !boolean(self.read(*condition)?, "if")? {
                        program_counter = *target;
                    }
                }
                Instruction::Jump { target } => program_counter = *target,
                Instruction::CheckCallDepth => {
                    if self.call_depth >= self.executor.limits.maximum_call_depth {
                        return Err(ExecError::CallDepth {
                            maximum: self.executor.limits.maximum_call_depth,
                        });
                    }
                }
                Instruction::EnterCall => {
                    self.call_depth += 1;
                    self.maximum_call_depth = self.maximum_call_depth.max(self.call_depth);
                }
                Instruction::ExitCall => self.call_depth -= 1,
                Instruction::AssertType {
                    value,
                    expected,
                    operation,
                } => {
                    if !self.read(*value)?.conforms_to(expected) {
                        return Err(if *operation == "call result" {
                            ExecError::ResultTypeMismatch
                        } else {
                            ExecError::TypeFault { operation }
                        });
                    }
                }
                Instruction::InitializeLoop { loop_id } => self.loop_counters[*loop_id] = 0,
                Instruction::JumpIfEqual {
                    left,
                    right,
                    target,
                } => {
                    if self.read(*left)? == self.read(*right)? {
                        program_counter = *target;
                    }
                }
                Instruction::RequireDecreasing { previous, next } => {
                    let previous = integer(self.read(*previous)?, "fixpoint progress")?;
                    let next = integer(self.read(*next)?, "fixpoint progress")?;
                    if next >= previous {
                        return Err(ExecError::NonDecreasingProgress { previous, next });
                    }
                }
                Instruction::ContinueLoop {
                    loop_id,
                    maximum,
                    target,
                } => {
                    let counter = &mut self.loop_counters[*loop_id];
                    *counter = counter.saturating_add(1);
                    if *counter < *maximum {
                        program_counter = *target;
                    }
                }
            }
        }
        let result = self.read(self.program.result)?.clone();
        if !result.conforms_to(&self.program.result_type) {
            return Err(ExecError::ResultTypeMismatch);
        }
        Ok(ExecutionTrace {
            result,
            steps: self.steps,
            maximum_call_depth: self.maximum_call_depth,
            events: std::mem::take(&mut self.events),
        })
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

    fn read(&self, register: Register) -> Result<&Value, ExecError> {
        self.registers.get(register).and_then(Option::as_ref).ok_or(
            ExecError::UnsupportedExpression("invalid bytecode register"),
        )
    }

    fn write(&mut self, register: Register, value: Value) {
        self.registers[register] = Some(value);
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

fn boolean(value: &Value, operation: &'static str) -> Result<bool, ExecError> {
    if let Value::Bool(value) = value {
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
    let values = match operation {
        BinaryOp::Union => left.union(&right).cloned().collect(),
        BinaryOp::Intersection => left.intersection(&right).cloned().collect(),
        BinaryOp::Difference => left.difference(&right).cloned().collect(),
        _ => unreachable!("set operation"),
    };
    Ok(Value::Set {
        element_type,
        maximum,
        values,
    })
}

fn value_type(value: &Value) -> Type {
    match value {
        Value::Bool(_) => Type::Bool,
        Value::Int(value) => Type::Int(cairn_rkl::IntRange::exact(*value)),
        Value::Atom(_) => Type::Atom,
        Value::Coord2 { .. } => Type::Coord2,
        Value::Tuple(values) => Type::Tuple(values.iter().map(value_type).collect()),
        Value::Set { element_type, .. } => element_type.clone(),
        Value::Handle { kind, .. } => match kind {
            cairn_rkl::HandleKind::Shape => Type::Shape,
            cairn_rkl::HandleKind::Entity => Type::Entity,
            cairn_rkl::HandleKind::Symbol => Type::Symbol,
            cairn_rkl::HandleKind::Action => Type::Action,
            cairn_rkl::HandleKind::State => Type::State,
        },
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
