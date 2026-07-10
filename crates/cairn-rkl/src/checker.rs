use crate::{
    BinaryOp, CheckError, CheckErrorKind, Definition, Expr, ExprPath, IntRange, PathSegment,
    PredicateExpr, Type, TypedExpr, UnaryOp, VarName,
};
use std::collections::{BTreeMap, BTreeSet};

/// Public function signature available to derived-call checking.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DefinitionSignature {
    pub parameters: Vec<Type>,
    pub return_type: Type,
}

/// Deterministic lexical type environment.
#[derive(Clone, Debug, Default)]
pub struct TypeEnvironment {
    variables: BTreeMap<VarName, Type>,
}

impl TypeEnvironment {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            variables: BTreeMap::new(),
        }
    }

    pub fn insert(&mut self, name: VarName, ty: Type) -> Option<Type> {
        self.variables.insert(name, ty)
    }

    #[must_use]
    pub fn get(&self, name: &VarName) -> Option<&Type> {
        self.variables.get(name)
    }
}

/// Exact reference type and range checker.
#[derive(Clone, Debug)]
pub struct TypeChecker {
    definitions: BTreeMap<String, DefinitionSignature>,
    maximum_nodes: usize,
}

impl TypeChecker {
    #[must_use]
    pub const fn new(maximum_nodes: usize) -> Self {
        Self {
            definitions: BTreeMap::new(),
            maximum_nodes,
        }
    }

    pub fn register_definition(
        &mut self,
        name: String,
        signature: DefinitionSignature,
    ) -> Option<DefinitionSignature> {
        self.definitions.insert(name, signature)
    }

    /// Infer a type, exact integer range, collection cap, and free variables.
    ///
    /// # Errors
    ///
    /// Returns a path-qualified type, bound, scope, arity, or capacity error.
    pub fn check(
        &self,
        expression: &Expr,
        environment: &TypeEnvironment,
    ) -> Result<TypedExpr, CheckError> {
        let mut nodes = 0;
        let (ty, free_variables) =
            self.infer(expression, environment, &ExprPath::default(), &mut nodes)?;
        Ok(TypedExpr {
            expression: expression.clone(),
            ty,
            free_variables,
            proof_annotations: Vec::new(),
        })
    }

    /// Check and wrap an expression that must be Boolean.
    ///
    /// # Errors
    ///
    /// Returns the ordinary checker error or an exact root type mismatch.
    pub fn check_predicate(
        &self,
        expression: &Expr,
        environment: &TypeEnvironment,
    ) -> Result<PredicateExpr, CheckError> {
        let typed = self.check(expression, environment)?;
        if typed.ty != Type::Bool {
            return Err(CheckError::new(
                ExprPath::default(),
                CheckErrorKind::TypeMismatch {
                    expected: Type::Bool,
                    actual: typed.ty,
                },
            ));
        }
        Ok(PredicateExpr(typed))
    }

    // The exhaustive reference checker is intentionally centralized so an AST
    // addition cannot omit type/bound/free-variable semantics unnoticed.
    #[allow(clippy::too_many_lines)]
    fn infer(
        &self,
        expression: &Expr,
        environment: &TypeEnvironment,
        path: &ExprPath,
        nodes: &mut usize,
    ) -> Result<(Type, BTreeSet<VarName>), CheckError> {
        *nodes = nodes.checked_add(1).ok_or_else(|| {
            CheckError::new(
                path.clone(),
                CheckErrorKind::InternalInvariant("node counter overflow".to_owned()),
            )
        })?;
        if *nodes > self.maximum_nodes {
            return Err(CheckError::new(
                path.clone(),
                CheckErrorKind::CapacityCap {
                    requested: *nodes,
                    maximum: self.maximum_nodes,
                },
            ));
        }
        match expression {
            Expr::Bool(_) => Ok((Type::Bool, BTreeSet::new())),
            Expr::Int(value) => Ok((Type::Int(IntRange::exact(*value)), BTreeSet::new())),
            Expr::Color(_) => Ok((Type::Color, BTreeSet::new())),
            Expr::Coord2 { .. } => Ok((Type::Coord2, BTreeSet::new())),
            Expr::Var(name) => {
                let ty = environment.get(name).cloned().ok_or_else(|| {
                    CheckError::new(
                        path.clone(),
                        CheckErrorKind::UnknownVariable(name.0.clone()),
                    )
                })?;
                validate_type(&ty, path)?;
                Ok((ty, BTreeSet::from([name.clone()])))
            }
            Expr::Tuple(values) => {
                let mut types = Vec::with_capacity(values.len());
                let mut free = BTreeSet::new();
                for (index, value) in values.iter().enumerate() {
                    let (ty, nested) = self.infer(
                        value,
                        environment,
                        &path.child(PathSegment::Element(index)),
                        nodes,
                    )?;
                    types.push(ty);
                    free.extend(nested);
                }
                Ok((Type::Tuple(types), free))
            }
            Expr::Let {
                binder,
                value,
                body,
            } => {
                let (value_type, mut free) =
                    self.infer(value, environment, &path.child(PathSegment::Value), nodes)?;
                let mut nested_environment = environment.clone();
                nested_environment.insert(binder.clone(), value_type);
                let (body_type, mut body_free) = self.infer(
                    body,
                    &nested_environment,
                    &path.child(PathSegment::Body),
                    nodes,
                )?;
                body_free.remove(binder);
                free.extend(body_free);
                Ok((body_type, free))
            }
            Expr::If {
                condition,
                then_branch,
                else_branch,
            } => {
                let (condition_type, mut free) = self.infer(
                    condition,
                    environment,
                    &path.child(PathSegment::Condition),
                    nodes,
                )?;
                require_type(
                    Type::Bool,
                    condition_type,
                    path.child(PathSegment::Condition),
                )?;
                let (then_type, then_free) = self.infer(
                    then_branch,
                    environment,
                    &path.child(PathSegment::Then),
                    nodes,
                )?;
                let (else_type, else_free) = self.infer(
                    else_branch,
                    environment,
                    &path.child(PathSegment::Else),
                    nodes,
                )?;
                let result = join_types(&then_type, &else_type).ok_or_else(|| {
                    CheckError::new(
                        path.clone(),
                        CheckErrorKind::BranchTypeMismatch {
                            then_type,
                            else_type,
                        },
                    )
                })?;
                free.extend(then_free);
                free.extend(else_free);
                Ok((result, free))
            }
            Expr::Unary { op, argument } => {
                let (argument_type, free) = self.infer(
                    argument,
                    environment,
                    &path.child(PathSegment::Argument),
                    nodes,
                )?;
                unary_type(*op, argument_type, path.child(PathSegment::Argument))
                    .map(|ty| (ty, free))
            }
            Expr::Binary { op, left, right } => {
                let (left_type, mut free) =
                    self.infer(left, environment, &path.child(PathSegment::Left), nodes)?;
                let (right_type, right_free) =
                    self.infer(right, environment, &path.child(PathSegment::Right), nodes)?;
                free.extend(right_free);
                binary_type(*op, left_type, right_type, path.clone()).map(|ty| (ty, free))
            }
            Expr::EmptySet { element, maximum } => {
                finite_set_type(element, *maximum, path).map(|ty| (ty, BTreeSet::new()))
            }
            Expr::SingletonSet { value, maximum } => {
                if *maximum == 0 {
                    return Err(CheckError::new(path.clone(), CheckErrorKind::EmptyBound));
                }
                let (element, free) =
                    self.infer(value, environment, &path.child(PathSegment::Value), nodes)?;
                finite_set_type(&element, *maximum, path).map(|ty| (ty, free))
            }
            Expr::Count(value) => {
                let (value_type, free) = self.infer(
                    value,
                    environment,
                    &path.child(PathSegment::Argument),
                    nodes,
                )?;
                let maximum = value_type.collection_maximum().ok_or_else(|| {
                    CheckError::new(
                        path.child(PathSegment::Argument),
                        CheckErrorKind::InvalidOperand {
                            operation: "count",
                            actual: value_type,
                        },
                    )
                })?;
                Ok((
                    Type::Int(IntRange {
                        lower: 0,
                        upper: i64::from(maximum),
                    }),
                    free,
                ))
            }
            Expr::Call {
                function,
                arguments,
            } => {
                let signature = self.definitions.get(function).ok_or_else(|| {
                    CheckError::new(
                        path.clone(),
                        CheckErrorKind::UnknownFunction(function.clone()),
                    )
                })?;
                if signature.parameters.len() != arguments.len() {
                    return Err(CheckError::new(
                        path.clone(),
                        CheckErrorKind::ArityMismatch {
                            expected: signature.parameters.len(),
                            actual: arguments.len(),
                        },
                    ));
                }
                let mut free = BTreeSet::new();
                for (index, (argument, expected)) in arguments
                    .iter()
                    .zip(signature.parameters.iter())
                    .enumerate()
                {
                    let argument_path = path.child(PathSegment::CallArgument(index));
                    let (actual, nested) =
                        self.infer(argument, environment, &argument_path, nodes)?;
                    require_compatible(expected, &actual, argument_path)?;
                    free.extend(nested);
                }
                Ok((signature.return_type.clone(), free))
            }
            Expr::BoundedFixpoint {
                binder,
                initial,
                step,
                maximum_steps,
                progress_measure,
            } => {
                if *maximum_steps == 0 {
                    return Err(CheckError::new(path.clone(), CheckErrorKind::EmptyBound));
                }
                let (initial_type, mut free) = self.infer(
                    initial,
                    environment,
                    &path.child(PathSegment::Initial),
                    nodes,
                )?;
                let mut nested_environment = environment.clone();
                nested_environment.insert(binder.clone(), initial_type.clone());
                let (step_type, mut step_free) = self.infer(
                    step,
                    &nested_environment,
                    &path.child(PathSegment::Step),
                    nodes,
                )?;
                require_compatible(&initial_type, &step_type, path.child(PathSegment::Step))?;
                let (progress_type, mut progress_free) = self.infer(
                    progress_measure,
                    &nested_environment,
                    &path.child(PathSegment::Progress),
                    nodes,
                )?;
                if !matches!(progress_type, Type::Int(_)) {
                    return Err(CheckError::new(
                        path.child(PathSegment::Progress),
                        CheckErrorKind::InvalidOperand {
                            operation: "fixpoint progress",
                            actual: progress_type,
                        },
                    ));
                }
                step_free.remove(binder);
                progress_free.remove(binder);
                free.extend(step_free);
                free.extend(progress_free);
                Ok((initial_type, free))
            }
        }
    }
}

/// Definition-level totality and explicit recursion-bound checks.
pub trait BoundChecker {
    /// Validate definition result type and explicit recursion obligations.
    ///
    /// # Errors
    ///
    /// Returns exact path-qualified type, recursion, bound, or capacity errors.
    fn check_definition(&self, definition: &Definition) -> Result<TypedExpr, CheckError>;
}

impl BoundChecker for TypeChecker {
    fn check_definition(&self, definition: &Definition) -> Result<TypedExpr, CheckError> {
        let recursive = contains_call(&definition.body, &definition.name);
        if recursive && definition.recursion.is_none() {
            return Err(CheckError::new(
                ExprPath::default(),
                CheckErrorKind::RecursiveCallWithoutBound(definition.name.clone()),
            ));
        }
        let mut environment = TypeEnvironment::new();
        for parameter in &definition.parameters {
            validate_type(&parameter.ty, &ExprPath::default())?;
            environment.insert(parameter.name.clone(), parameter.ty.clone());
        }
        validate_type(&definition.return_type, &ExprPath::default())?;
        if let Some(recursion) = &definition.recursion {
            if recursion.maximum_steps == 0 {
                return Err(CheckError::new(
                    ExprPath::default(),
                    CheckErrorKind::EmptyBound,
                ));
            }
            let progress = self.check(&recursion.progress_measure, &environment)?;
            if !matches!(progress.ty, Type::Int(_)) {
                return Err(CheckError::new(
                    ExprPath::default(),
                    CheckErrorKind::RecursiveBoundMissingProgress(definition.name.clone()),
                ));
            }
        }
        let typed = self.check(&definition.body, &environment)?;
        require_compatible(&definition.return_type, &typed.ty, ExprPath::default())?;
        Ok(typed)
    }
}

fn require_type(expected: Type, actual: Type, path: ExprPath) -> Result<(), CheckError> {
    if expected == actual {
        Ok(())
    } else {
        Err(CheckError::new(
            path,
            CheckErrorKind::TypeMismatch { expected, actual },
        ))
    }
}

fn require_compatible(expected: &Type, actual: &Type, path: ExprPath) -> Result<(), CheckError> {
    if compatible(expected, actual) {
        Ok(())
    } else {
        Err(CheckError::new(
            path,
            CheckErrorKind::TypeMismatch {
                expected: expected.clone(),
                actual: actual.clone(),
            },
        ))
    }
}

fn compatible(left: &Type, right: &Type) -> bool {
    match (left, right) {
        (Type::Int(_), Type::Int(_))
        | (Type::CellSet { .. }, Type::CellSet { .. })
        | (Type::EntitySet { .. }, Type::EntitySet { .. }) => true,
        (Type::List { element: left, .. }, Type::List { element: right, .. }) => {
            compatible(left, right)
        }
        _ => left == right,
    }
}

fn join_types(left: &Type, right: &Type) -> Option<Type> {
    match (left, right) {
        (Type::Int(left), Type::Int(right)) => Some(Type::Int(IntRange {
            lower: left.lower.min(right.lower),
            upper: left.upper.max(right.upper),
        })),
        (Type::CellSet { maximum: left }, Type::CellSet { maximum: right }) => {
            Some(Type::CellSet {
                maximum: (*left).max(*right),
            })
        }
        (Type::EntitySet { maximum: left }, Type::EntitySet { maximum: right }) => {
            Some(Type::EntitySet {
                maximum: (*left).max(*right),
            })
        }
        _ if left == right => Some(left.clone()),
        _ => None,
    }
}

fn unary_type(op: UnaryOp, argument: Type, path: ExprPath) -> Result<Type, CheckError> {
    match (op, argument) {
        (UnaryOp::Not, Type::Bool) => Ok(Type::Bool),
        (UnaryOp::Negate, Type::Int(range)) => {
            let lower = checked_i64(-i128::from(range.upper), &path)?;
            let upper = checked_i64(-i128::from(range.lower), &path)?;
            Ok(Type::Int(IntRange { lower, upper }))
        }
        (UnaryOp::Not, actual) => Err(invalid_operand("not", actual, path)),
        (UnaryOp::Negate, actual) => Err(invalid_operand("negate", actual, path)),
    }
}

fn binary_type(op: BinaryOp, left: Type, right: Type, path: ExprPath) -> Result<Type, CheckError> {
    match op {
        BinaryOp::Add | BinaryOp::Subtract | BinaryOp::Multiply => {
            let (Type::Int(left), Type::Int(right)) = (&left, &right) else {
                return Err(invalid_operand("integer binary", left, path));
            };
            arithmetic_range(op, *left, *right, &path).map(Type::Int)
        }
        BinaryOp::Minimum | BinaryOp::Maximum => {
            let (Type::Int(left), Type::Int(right)) = (&left, &right) else {
                return Err(invalid_operand("integer comparison", left, path));
            };
            let range = if op == BinaryOp::Minimum {
                IntRange {
                    lower: left.lower.min(right.lower),
                    upper: left.upper.min(right.upper),
                }
            } else {
                IntRange {
                    lower: left.lower.max(right.lower),
                    upper: left.upper.max(right.upper),
                }
            };
            Ok(Type::Int(range))
        }
        BinaryOp::And | BinaryOp::Or => {
            require_type(Type::Bool, left, path.clone())?;
            require_type(Type::Bool, right, path)?;
            Ok(Type::Bool)
        }
        BinaryOp::Equal => {
            require_compatible(&left, &right, path)?;
            Ok(Type::Bool)
        }
        BinaryOp::LessThan => {
            if matches!(left, Type::Int(_)) && matches!(right, Type::Int(_)) {
                Ok(Type::Bool)
            } else {
                Err(invalid_operand("less-than", left, path))
            }
        }
        BinaryOp::Union | BinaryOp::Intersection | BinaryOp::Difference => {
            collection_binary_type(op, left, right, path)
        }
    }
}

fn arithmetic_range(
    op: BinaryOp,
    left: IntRange,
    right: IntRange,
    path: &ExprPath,
) -> Result<IntRange, CheckError> {
    let (lower, upper) = match op {
        BinaryOp::Add => (
            i128::from(left.lower) + i128::from(right.lower),
            i128::from(left.upper) + i128::from(right.upper),
        ),
        BinaryOp::Subtract => (
            i128::from(left.lower) - i128::from(right.upper),
            i128::from(left.upper) - i128::from(right.lower),
        ),
        BinaryOp::Multiply => {
            let products = [
                i128::from(left.lower) * i128::from(right.lower),
                i128::from(left.lower) * i128::from(right.upper),
                i128::from(left.upper) * i128::from(right.lower),
                i128::from(left.upper) * i128::from(right.upper),
            ];
            (
                *products.iter().min().ok_or_else(|| {
                    CheckError::new(
                        path.clone(),
                        CheckErrorKind::InternalInvariant("empty product bounds".to_owned()),
                    )
                })?,
                *products.iter().max().ok_or_else(|| {
                    CheckError::new(
                        path.clone(),
                        CheckErrorKind::InternalInvariant("empty product bounds".to_owned()),
                    )
                })?,
            )
        }
        _ => {
            return Err(CheckError::new(
                path.clone(),
                CheckErrorKind::InternalInvariant("non-arithmetic range request".to_owned()),
            ));
        }
    };
    Ok(IntRange {
        lower: checked_i64(lower, path)?,
        upper: checked_i64(upper, path)?,
    })
}

fn checked_i64(value: i128, path: &ExprPath) -> Result<i64, CheckError> {
    i64::try_from(value).map_err(|_| CheckError::new(path.clone(), CheckErrorKind::BoundOverflow))
}

fn finite_set_type(element: &Type, maximum: u32, path: &ExprPath) -> Result<Type, CheckError> {
    if maximum == 0 {
        return Err(CheckError::new(path.clone(), CheckErrorKind::EmptyBound));
    }
    match element {
        Type::Coord2 => Ok(Type::CellSet { maximum }),
        Type::Entity => Ok(Type::EntitySet { maximum }),
        other => Ok(Type::List {
            element: Box::new(other.clone()),
            maximum,
        }),
    }
}

fn collection_binary_type(
    op: BinaryOp,
    left: Type,
    right: Type,
    path: ExprPath,
) -> Result<Type, CheckError> {
    match (left, right) {
        (Type::CellSet { maximum: left }, Type::CellSet { maximum: right }) => {
            collection_bound(op, left, right, path).map(|maximum| Type::CellSet { maximum })
        }
        (Type::EntitySet { maximum: left }, Type::EntitySet { maximum: right }) => {
            collection_bound(op, left, right, path).map(|maximum| Type::EntitySet { maximum })
        }
        (left, _) => Err(invalid_operand("set binary", left, path)),
    }
}

fn collection_bound(
    op: BinaryOp,
    left: u32,
    right: u32,
    path: ExprPath,
) -> Result<u32, CheckError> {
    match op {
        BinaryOp::Union => left
            .checked_add(right)
            .ok_or_else(|| CheckError::new(path, CheckErrorKind::BoundOverflow)),
        BinaryOp::Intersection => Ok(left.min(right)),
        BinaryOp::Difference => Ok(left),
        _ => Err(CheckError::new(
            path,
            CheckErrorKind::InternalInvariant("non-set bound request".to_owned()),
        )),
    }
}

fn invalid_operand(operation: &'static str, actual: Type, path: ExprPath) -> CheckError {
    CheckError::new(path, CheckErrorKind::InvalidOperand { operation, actual })
}

fn validate_type(ty: &Type, path: &ExprPath) -> Result<(), CheckError> {
    match ty {
        Type::Int(range) if !range.is_valid() => {
            Err(CheckError::new(path.clone(), CheckErrorKind::InvalidRange))
        }
        Type::CellSet { maximum }
        | Type::EntitySet { maximum }
        | Type::Relation { maximum, .. }
            if *maximum == 0 =>
        {
            Err(CheckError::new(path.clone(), CheckErrorKind::EmptyBound))
        }
        Type::Grid {
            maximum_width,
            maximum_height,
        } if *maximum_width == 0 || *maximum_height == 0 => {
            Err(CheckError::new(path.clone(), CheckErrorKind::EmptyBound))
        }
        Type::Tuple(values) => {
            for value in values {
                validate_type(value, path)?;
            }
            Ok(())
        }
        Type::Option(value) => validate_type(value, path),
        Type::List { element, maximum } => {
            if *maximum == 0 {
                return Err(CheckError::new(path.clone(), CheckErrorKind::EmptyBound));
            }
            validate_type(element, path)
        }
        Type::Map {
            key,
            value,
            maximum,
        } => {
            if *maximum == 0 {
                return Err(CheckError::new(path.clone(), CheckErrorKind::EmptyBound));
            }
            validate_type(key, path)?;
            validate_type(value, path)
        }
        Type::Relation { arguments, .. } => {
            for argument in arguments {
                validate_type(argument, path)?;
            }
            Ok(())
        }
        Type::Operator { input, output } => {
            validate_type(input, path)?;
            validate_type(output, path)
        }
        _ => Ok(()),
    }
}

fn contains_call(expression: &Expr, name: &str) -> bool {
    match expression {
        Expr::Call {
            function,
            arguments,
        } => function == name || arguments.iter().any(|value| contains_call(value, name)),
        Expr::Tuple(values) => values.iter().any(|value| contains_call(value, name)),
        Expr::Let { value, body, .. } => contains_call(value, name) || contains_call(body, name),
        Expr::If {
            condition,
            then_branch,
            else_branch,
        } => {
            contains_call(condition, name)
                || contains_call(then_branch, name)
                || contains_call(else_branch, name)
        }
        Expr::Unary { argument, .. } | Expr::Count(argument) => contains_call(argument, name),
        Expr::Binary { left, right, .. } => contains_call(left, name) || contains_call(right, name),
        Expr::SingletonSet { value, .. } => contains_call(value, name),
        Expr::BoundedFixpoint {
            initial,
            step,
            progress_measure,
            ..
        } => {
            contains_call(initial, name)
                || contains_call(step, name)
                || contains_call(progress_measure, name)
        }
        Expr::Bool(_)
        | Expr::Int(_)
        | Expr::Color(_)
        | Expr::Coord2 { .. }
        | Expr::Var(_)
        | Expr::EmptySet { .. } => false,
    }
}
