use crate::{
    BinaryOp, CheckError, CheckErrorKind, CheckedTheory, Definition, Effect, Expr, ExprPath,
    IntRange, PathSegment, PredicateExpr, Schedule, Theory, Type, TypedExpr, UnaryOp, VarName,
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

    pub fn iter(&self) -> impl Iterator<Item = (&VarName, &Type)> {
        self.variables.iter()
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
            Expr::Atom(_) => Ok((Type::Atom, BTreeSet::new())),
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

/// Whole-theory structural, typing, totality, and schedule validation.
pub trait TheoryChecker {
    /// Admit a theory for canonical persistence and later execution.
    ///
    /// # Errors
    ///
    /// Returns a deterministic theory-qualified path and exact failure class.
    fn check_theory(&self, theory: &Theory) -> Result<CheckedTheory, CheckError>;
}

impl TheoryChecker for TypeChecker {
    fn check_theory(&self, theory: &Theory) -> Result<CheckedTheory, CheckError> {
        if theory.definitions.is_empty() && theory.rules.is_empty() {
            return Err(CheckError::new(
                ExprPath::default(),
                CheckErrorKind::EmptyTheory,
            ));
        }

        let inputs = check_inputs(&theory.inputs)?;
        let checker = checker_with_definitions(self, &theory.definitions)?;
        check_definitions(&checker, &theory.definitions)?;
        let rule_names = check_rules(&checker, &theory.rules, &inputs)?;
        validate_schedule(&theory.schedule, &rule_names)?;
        check_renderer_and_assumptions(&checker, theory, &inputs)?;

        Ok(CheckedTheory {
            theory: theory.clone(),
        })
    }
}

fn check_inputs(inputs: &[crate::Binder]) -> Result<TypeEnvironment, CheckError> {
    let mut environment = TypeEnvironment::new();
    for (index, input) in inputs.iter().enumerate() {
        let path = ExprPath::default().child(PathSegment::Binder(index));
        validate_type(&input.ty, &path)?;
        if environment
            .insert(input.name.clone(), input.ty.clone())
            .is_some()
        {
            return Err(CheckError::new(
                path,
                CheckErrorKind::DuplicateBinder(input.name.0.clone()),
            ));
        }
    }
    Ok(environment)
}

fn checker_with_definitions(
    base: &TypeChecker,
    definitions: &[Definition],
) -> Result<TypeChecker, CheckError> {
    let mut checker = base.clone();
    let mut names = BTreeSet::new();
    for (index, definition) in definitions.iter().enumerate() {
        let path = ExprPath::default().child(PathSegment::Definition(index));
        if !names.insert(definition.name.clone()) {
            return Err(CheckError::new(
                path,
                CheckErrorKind::DuplicateDefinition(definition.name.clone()),
            ));
        }
        for parameter in &definition.parameters {
            validate_type(&parameter.ty, &path)?;
        }
        validate_type(&definition.return_type, &path)?;
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
    Ok(checker)
}

fn check_definitions(checker: &TypeChecker, definitions: &[Definition]) -> Result<(), CheckError> {
    if let Some(cycle) = mutual_recursion_cycle(definitions) {
        return Err(CheckError::new(
            ExprPath::default(),
            CheckErrorKind::MutualRecursionUnsupported(cycle),
        ));
    }
    for (index, definition) in definitions.iter().enumerate() {
        checker
            .check_definition(definition)
            .map_err(|error| prefix_error(error, PathSegment::Definition(index)))?;
    }
    Ok(())
}

fn check_rules(
    checker: &TypeChecker,
    rules: &[crate::RewriteRule],
    inputs: &TypeEnvironment,
) -> Result<BTreeSet<String>, CheckError> {
    let mut names = BTreeSet::new();
    for (index, rule) in rules.iter().enumerate() {
        let path = ExprPath::default().child(PathSegment::Rule(index));
        if !names.insert(rule.name.clone()) {
            return Err(CheckError::new(
                path.clone(),
                CheckErrorKind::DuplicateRule(rule.name.clone()),
            ));
        }
        let environment = check_rule_binders(checker, rule, &path, inputs)?;
        checker
            .check_predicate(&rule.guard, &environment)
            .map_err(|error| prefix_path(error, &path.child(PathSegment::Guard)))?;
        check_effects(checker, &rule.effects, &environment, &path)?;
    }
    Ok(names)
}

fn check_rule_binders(
    checker: &TypeChecker,
    rule: &crate::RewriteRule,
    rule_path: &ExprPath,
    inputs: &TypeEnvironment,
) -> Result<TypeEnvironment, CheckError> {
    let mut environment = inputs.clone();
    let mut names = inputs
        .iter()
        .map(|(name, _)| name.clone())
        .collect::<BTreeSet<_>>();
    for (index, binder) in rule.binders.iter().enumerate() {
        let path = rule_path.child(PathSegment::Binder(index));
        if !names.insert(binder.variable.name.clone()) {
            return Err(CheckError::new(
                path,
                CheckErrorKind::DuplicateBinder(binder.variable.name.0.clone()),
            ));
        }
        validate_type(&binder.variable.ty, &path)?;
        let domain = checker
            .check(&binder.domain, &environment)
            .map_err(|error| prefix_path(error, &path.child(PathSegment::Domain)))?;
        let element = domain_element_type(domain.ty()).ok_or_else(|| {
            CheckError::new(
                path.child(PathSegment::Domain),
                CheckErrorKind::InvalidDomain(domain.ty().clone()),
            )
        })?;
        let binding_type = if binder.quantifier == crate::Quantifier::Group {
            domain.ty().clone()
        } else {
            element
        };
        require_compatible(
            &binder.variable.ty,
            &binding_type,
            path.child(PathSegment::Domain),
        )?;
        environment.insert(binder.variable.name.clone(), binder.variable.ty.clone());
    }
    Ok(environment)
}

fn check_effects(
    checker: &TypeChecker,
    effects: &[Effect],
    environment: &TypeEnvironment,
    rule_path: &ExprPath,
) -> Result<(), CheckError> {
    for (index, effect) in effects.iter().enumerate() {
        let path = rule_path.child(PathSegment::Effect(index));
        match effect {
            Effect::Preserve(value) | Effect::Delete(value) | Effect::Emit(value) => {
                checker
                    .check(value, environment)
                    .map_err(|error| prefix_path(error, &path))?;
            }
            Effect::Write { target, value } => {
                checker
                    .check(target, environment)
                    .map_err(|error| prefix_path(error, &path))?;
                checker
                    .check(value, environment)
                    .map_err(|error| prefix_path(error, &path))?;
            }
            Effect::SetState { target, value } => {
                let target_type = checker
                    .check(target, environment)
                    .map_err(|error| prefix_path(error, &path))?;
                require_type(Type::State, target_type.ty().clone(), path.clone())?;
                checker
                    .check(value, environment)
                    .map_err(|error| prefix_path(error, &path))?;
            }
        }
    }
    Ok(())
}

fn check_renderer_and_assumptions(
    checker: &TypeChecker,
    theory: &Theory,
    inputs: &TypeEnvironment,
) -> Result<(), CheckError> {
    checker
        .check(&theory.renderer.canvas, inputs)
        .map_err(|error| prefix_error(error, PathSegment::Canvas))?;
    for (index, layer) in theory.renderer.layers.iter().enumerate() {
        checker
            .check(layer, inputs)
            .map_err(|error| prefix_error(error, PathSegment::Layer(index)))?;
    }
    checker
        .check(&theory.renderer.background, inputs)
        .map_err(|error| prefix_error(error, PathSegment::Background))?;
    for (index, assumption) in theory.assumptions.iter().enumerate() {
        checker
            .check_predicate(assumption, inputs)
            .map_err(|error| prefix_error(error, PathSegment::Assumption(index)))?;
    }
    Ok(())
}

fn domain_element_type(ty: &Type) -> Option<Type> {
    match ty {
        Type::CellSet { .. } => Some(Type::Coord2),
        Type::EntitySet { .. } => Some(Type::Entity),
        Type::List { element, .. } => Some((**element).clone()),
        Type::Map { key, value, .. } => Some(Type::Tuple(vec![(**key).clone(), (**value).clone()])),
        Type::Relation { arguments, .. } => Some(Type::Tuple(arguments.clone())),
        _ => None,
    }
}

fn prefix_error(error: CheckError, segment: PathSegment) -> CheckError {
    prefix_path(error, &ExprPath::default().child(segment))
}

fn prefix_path(mut error: CheckError, prefix: &ExprPath) -> CheckError {
    let mut path = prefix.0.clone();
    path.extend(error.path.0);
    error.path = ExprPath(path);
    error
}

fn validate_schedule(schedule: &Schedule, rules: &BTreeSet<String>) -> Result<(), CheckError> {
    let path = ExprPath::default().child(PathSegment::Schedule);
    let mut mentioned = BTreeSet::new();
    validate_schedule_part(schedule, rules, &mut mentioned, &path)
}

fn validate_schedule_part(
    schedule: &Schedule,
    rules: &BTreeSet<String>,
    mentioned: &mut BTreeSet<String>,
    path: &ExprPath,
) -> Result<(), CheckError> {
    match schedule {
        Schedule::Parallel => Ok(()),
        Schedule::Sequence(names) | Schedule::DecisionList(names) => {
            for name in names {
                validate_scheduled_name(name, rules, mentioned, path)?;
            }
            Ok(())
        }
        Schedule::DependencyDag(edges) => {
            let mut graph = BTreeMap::<String, BTreeSet<String>>::new();
            for (before, after) in edges {
                validate_rule_name(before, rules, path)?;
                validate_rule_name(after, rules, path)?;
                mentioned.insert(before.clone());
                mentioned.insert(after.clone());
                graph
                    .entry(before.clone())
                    .or_default()
                    .insert(after.clone());
            }
            if directed_cycle(&graph) {
                return Err(CheckError::new(path.clone(), CheckErrorKind::ScheduleCycle));
            }
            Ok(())
        }
        Schedule::BoundedFixpoint {
            rules: names,
            maximum_steps,
        } => {
            if *maximum_steps == 0 {
                return Err(CheckError::new(path.clone(), CheckErrorKind::EmptyBound));
            }
            for name in names {
                validate_scheduled_name(name, rules, mentioned, path)?;
            }
            Ok(())
        }
        Schedule::Hierarchical(parts) => {
            for part in parts {
                validate_schedule_part(part, rules, mentioned, path)?;
            }
            Ok(())
        }
    }
}

fn validate_rule_name(
    name: &str,
    rules: &BTreeSet<String>,
    path: &ExprPath,
) -> Result<(), CheckError> {
    if rules.contains(name) {
        Ok(())
    } else {
        Err(CheckError::new(
            path.clone(),
            CheckErrorKind::UnknownRule(name.to_owned()),
        ))
    }
}

fn validate_scheduled_name(
    name: &str,
    rules: &BTreeSet<String>,
    mentioned: &mut BTreeSet<String>,
    path: &ExprPath,
) -> Result<(), CheckError> {
    validate_rule_name(name, rules, path)?;
    if !mentioned.insert(name.to_owned()) {
        return Err(CheckError::new(
            path.clone(),
            CheckErrorKind::DuplicateScheduledRule(name.to_owned()),
        ));
    }
    Ok(())
}

fn directed_cycle(graph: &BTreeMap<String, BTreeSet<String>>) -> bool {
    fn visit(
        node: &str,
        graph: &BTreeMap<String, BTreeSet<String>>,
        active: &mut BTreeSet<String>,
        complete: &mut BTreeSet<String>,
    ) -> bool {
        if active.contains(node) {
            return true;
        }
        if complete.contains(node) {
            return false;
        }
        active.insert(node.to_owned());
        if graph.get(node).is_some_and(|next| {
            next.iter()
                .any(|child| visit(child, graph, active, complete))
        }) {
            return true;
        }
        active.remove(node);
        complete.insert(node.to_owned());
        false
    }

    let mut active = BTreeSet::new();
    let mut complete = BTreeSet::new();
    graph
        .keys()
        .any(|node| visit(node, graph, &mut active, &mut complete))
}

fn mutual_recursion_cycle(definitions: &[Definition]) -> Option<Vec<String>> {
    let names = definitions
        .iter()
        .map(|definition| definition.name.clone())
        .collect::<BTreeSet<_>>();
    let graph = definitions
        .iter()
        .map(|definition| {
            let mut calls = BTreeSet::new();
            collect_calls(&definition.body, &mut calls);
            calls.remove(&definition.name);
            calls.retain(|name| names.contains(name));
            (definition.name.clone(), calls)
        })
        .collect::<BTreeMap<_, _>>();

    let mut active = Vec::new();
    let mut complete = BTreeSet::new();
    graph
        .keys()
        .find_map(|name| mutual_visit(name, &graph, &mut active, &mut complete))
}

fn mutual_visit(
    node: &str,
    graph: &BTreeMap<String, BTreeSet<String>>,
    active: &mut Vec<String>,
    complete: &mut BTreeSet<String>,
) -> Option<Vec<String>> {
    if let Some(index) = active.iter().position(|name| name == node) {
        return Some(active[index..].to_vec());
    }
    if complete.contains(node) {
        return None;
    }
    active.push(node.to_owned());
    if let Some(cycle) = graph
        .get(node)
        .into_iter()
        .flatten()
        .find_map(|child| mutual_visit(child, graph, active, complete))
    {
        return Some(cycle);
    }
    active.pop();
    complete.insert(node.to_owned());
    None
}

fn collect_calls(expression: &Expr, calls: &mut BTreeSet<String>) {
    match expression {
        Expr::Call {
            function,
            arguments,
        } => {
            calls.insert(function.clone());
            for argument in arguments {
                collect_calls(argument, calls);
            }
        }
        Expr::Tuple(values) => {
            for value in values {
                collect_calls(value, calls);
            }
        }
        Expr::Let { value, body, .. } => {
            collect_calls(value, calls);
            collect_calls(body, calls);
        }
        Expr::If {
            condition,
            then_branch,
            else_branch,
        } => {
            collect_calls(condition, calls);
            collect_calls(then_branch, calls);
            collect_calls(else_branch, calls);
        }
        Expr::Unary { argument, .. } | Expr::Count(argument) => collect_calls(argument, calls),
        Expr::Binary { left, right, .. } => {
            collect_calls(left, calls);
            collect_calls(right, calls);
        }
        Expr::SingletonSet { value, .. } => collect_calls(value, calls),
        Expr::BoundedFixpoint {
            initial,
            step,
            progress_measure,
            ..
        } => {
            collect_calls(initial, calls);
            collect_calls(step, calls);
            collect_calls(progress_measure, calls);
        }
        Expr::Bool(_)
        | Expr::Int(_)
        | Expr::Atom(_)
        | Expr::Coord2 { .. }
        | Expr::Var(_)
        | Expr::EmptySet { .. } => {}
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
        Type::FiniteField {
            axes,
            maximum_elements,
        } if *axes == 0 || *maximum_elements == 0 => {
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
        | Expr::Atom(_)
        | Expr::Coord2 { .. }
        | Expr::Var(_)
        | Expr::EmptySet { .. } => false,
    }
}
