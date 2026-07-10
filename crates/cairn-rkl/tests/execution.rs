use cairn_rkl::{
    BinaryOp, Binder, ConflictPolicy, Definition, DefinitionSignature, ExecError, ExecLimits, Expr,
    IntRange, ReferenceExecutor, Renderer, RuntimeEnvironment, Schedule, Theory, TheoryChecker,
    TraceLevel, Type, TypeChecker, TypeEnvironment, Value, VarName,
};

fn limits() -> ExecLimits {
    ExecLimits {
        fuel: 1_000,
        maximum_call_depth: 16,
        maximum_trace_events: 1_000,
    }
}

fn checked(expression: &Expr, environment: &TypeEnvironment) -> cairn_rkl::TypedExpr {
    TypeChecker::new(1_000)
        .check(expression, environment)
        .expect("checked expression")
}

#[test]
fn reference_execution_is_exact_and_trace_deterministic() {
    let expression = Expr::Let {
        binder: VarName("amount".to_owned()),
        value: Box::new(Expr::Binary {
            op: BinaryOp::Add,
            left: Box::new(Expr::Int(1)),
            right: Box::new(Expr::Int(2)),
        }),
        body: Box::new(Expr::Binary {
            op: BinaryOp::Multiply,
            left: Box::new(Expr::Var(VarName("amount".to_owned()))),
            right: Box::new(Expr::Int(3)),
        }),
    };
    let expression = checked(&expression, &TypeEnvironment::new());
    let executor = ReferenceExecutor::without_definitions(limits(), TraceLevel::Full);

    let first = executor
        .execute(&expression, &RuntimeEnvironment::new())
        .expect("first execution");
    let second = executor
        .execute(&expression, &RuntimeEnvironment::new())
        .expect("deterministic replay");

    assert_eq!(first, second);
    assert_eq!(first.result, Value::Int(9));
    assert_eq!(first.steps, 7);
    assert_eq!(
        first
            .events
            .iter()
            .map(|event| event.operation)
            .collect::<Vec<_>>(),
        ["let", "binary", "int", "int", "binary", "var", "int"]
    );
}

#[test]
fn fuel_and_trace_caps_fail_closed() {
    let expression = checked(
        &Expr::Binary {
            op: BinaryOp::Add,
            left: Box::new(Expr::Int(1)),
            right: Box::new(Expr::Int(2)),
        },
        &TypeEnvironment::new(),
    );
    let fuel_error = ReferenceExecutor::without_definitions(
        ExecLimits {
            fuel: 2,
            ..limits()
        },
        TraceLevel::Off,
    )
    .execute(&expression, &RuntimeEnvironment::new())
    .expect_err("fuel exhausted");
    assert_eq!(
        fuel_error,
        ExecError::FuelExhausted {
            consumed: 2,
            limit: 2
        }
    );

    let trace_error = ReferenceExecutor::without_definitions(
        ExecLimits {
            maximum_trace_events: 1,
            ..limits()
        },
        TraceLevel::Full,
    )
    .execute(&expression, &RuntimeEnvironment::new())
    .expect_err("trace cap reached");
    assert_eq!(trace_error, ExecError::TraceCapacity { maximum: 1 });
}

#[test]
fn bounded_collections_execute_without_losing_declared_capacity() {
    let expression = checked(
        &Expr::Count(Box::new(Expr::SingletonSet {
            value: Box::new(Expr::Atom(7)),
            maximum: 4,
        })),
        &TypeEnvironment::new(),
    );
    let trace = ReferenceExecutor::without_definitions(limits(), TraceLevel::Summary)
        .execute(&expression, &RuntimeEnvironment::new())
        .expect("bounded set count");
    assert_eq!(trace.result, Value::Int(1));
    assert!(trace.events.is_empty());
}

#[test]
fn checked_task_local_definition_is_executable() {
    let input_type = Type::Int(IntRange {
        lower: -10,
        upper: 10,
    });
    let output_type = Type::Int(IntRange {
        lower: -20,
        upper: 20,
    });
    let theory = Theory {
        inputs: Vec::new(),
        definitions: vec![Definition {
            name: "double".to_owned(),
            parameters: vec![Binder {
                name: VarName("value".to_owned()),
                ty: input_type.clone(),
            }],
            return_type: output_type.clone(),
            body: Expr::Binary {
                op: BinaryOp::Multiply,
                left: Box::new(Expr::Var(VarName("value".to_owned()))),
                right: Box::new(Expr::Int(2)),
            },
            recursion: None,
            proof_annotations: Vec::new(),
        }],
        rules: Vec::new(),
        schedule: Schedule::Parallel,
        renderer: Renderer {
            canvas: Expr::Int(0),
            layers: Vec::new(),
            conflict_policy: ConflictPolicy::Reject,
            background: Expr::Atom(0),
        },
        assumptions: vec![Expr::Bool(true)],
        proof_annotations: Vec::new(),
    };
    let theory = TypeChecker::new(1_000)
        .check_theory(&theory)
        .expect("checked learned theory");
    let mut checker = TypeChecker::new(1_000);
    checker.register_definition(
        "double".to_owned(),
        DefinitionSignature {
            parameters: vec![input_type],
            return_type: output_type,
        },
    );
    let call = checker
        .check(
            &Expr::Call {
                function: "double".to_owned(),
                arguments: vec![Expr::Int(7)],
            },
            &TypeEnvironment::new(),
        )
        .expect("checked call");

    let trace = ReferenceExecutor::new(&theory, limits(), TraceLevel::Full)
        .execute(&call, &RuntimeEnvironment::new())
        .expect("execute task-local definition");

    assert_eq!(trace.result, Value::Int(14));
    assert_eq!(trace.maximum_call_depth, 1);
    assert!(trace.events.iter().any(|event| event.call_depth == 1));
}

#[test]
fn bounded_fixpoint_requires_strict_progress_and_stabilizes() {
    let state = VarName("state".to_owned());
    let input = VarName("input".to_owned());
    let mut types = TypeEnvironment::new();
    types.insert(input.clone(), Type::Int(IntRange { lower: 0, upper: 3 }));
    let countdown = checked(
        &Expr::BoundedFixpoint {
            binder: state.clone(),
            initial: Box::new(Expr::Var(input.clone())),
            step: Box::new(Expr::If {
                condition: Box::new(Expr::Binary {
                    op: BinaryOp::LessThan,
                    left: Box::new(Expr::Var(state.clone())),
                    right: Box::new(Expr::Int(1)),
                }),
                then_branch: Box::new(Expr::Var(state.clone())),
                else_branch: Box::new(Expr::Binary {
                    op: BinaryOp::Subtract,
                    left: Box::new(Expr::Var(state.clone())),
                    right: Box::new(Expr::Int(1)),
                }),
            }),
            maximum_steps: 8,
            progress_measure: Box::new(Expr::Var(state.clone())),
        },
        &types,
    );
    let mut values = RuntimeEnvironment::new();
    values.insert(input.clone(), Value::Int(3));
    let trace = ReferenceExecutor::without_definitions(limits(), TraceLevel::Off)
        .execute(&countdown, &values)
        .expect("countdown stabilizes");
    assert_eq!(trace.result, Value::Int(0));

    let increasing = checked(
        &Expr::BoundedFixpoint {
            binder: state.clone(),
            initial: Box::new(Expr::Var(input)),
            step: Box::new(Expr::Binary {
                op: BinaryOp::Add,
                left: Box::new(Expr::Var(state.clone())),
                right: Box::new(Expr::Int(1)),
            }),
            maximum_steps: 2,
            progress_measure: Box::new(Expr::Var(state)),
        },
        &types,
    );
    let error = ReferenceExecutor::without_definitions(limits(), TraceLevel::Off)
        .execute(&increasing, &values)
        .expect_err("increasing measure rejected");
    assert_eq!(
        error,
        ExecError::NonDecreasingProgress {
            previous: 3,
            next: 4
        }
    );
}

#[test]
fn runtime_values_must_respect_checked_integer_ranges() {
    let variable = VarName("value".to_owned());
    let mut types = TypeEnvironment::new();
    types.insert(variable.clone(), Type::Int(IntRange { lower: 0, upper: 3 }));
    let expression = checked(&Expr::Var(variable.clone()), &types);
    let mut values = RuntimeEnvironment::new();
    values.insert(variable, Value::Int(4));

    let error = ReferenceExecutor::without_definitions(limits(), TraceLevel::Off)
        .execute(&expression, &values)
        .expect_err("out-of-range runtime value");
    assert_eq!(error, ExecError::ResultTypeMismatch);
}
