use cairn_bytecode::{
    BytecodeCompiler, CompileError, CompileLimits, ExecutionWorkspace, FastExecutor,
    FastTheoryExecutor,
};
use cairn_rkl::{
    BinaryOp, Binder, ConflictPolicy, Definition, DefinitionSignature, Effect, ExecLimits, Expr,
    IntRange, Quantifier, ReferenceExecutor, ReferenceTheoryExecutor, Renderer, RewriteRule,
    RuleBinder, RuntimeEnvironment, Schedule, Theory, TheoryChecker, TheoryExecLimits,
    TheoryExecutionRequest, TraceLevel, Type, TypeChecker, TypeEnvironment, UnaryOp, Value,
    VarName,
};
use std::collections::{BTreeMap, BTreeSet};

fn execution_limits() -> ExecLimits {
    ExecLimits {
        fuel: 10_000,
        maximum_call_depth: 32,
        maximum_trace_events: 10_000,
    }
}

#[test]
fn whole_theory_bytecode_matches_reference_on_typed_observation_inputs() {
    let cells = VarName("cells".to_owned());
    let cell = VarName("cell".to_owned());
    let cells_type = Type::List {
        element: Box::new(Type::Coord2),
        maximum: 4,
    };
    let theory = TypeChecker::new(10_000)
        .check_theory(&Theory {
            inputs: vec![Binder {
                name: cells.clone(),
                ty: cells_type.clone(),
            }],
            definitions: Vec::new(),
            rules: vec![RewriteRule {
                name: "paint_observed".to_owned(),
                binders: vec![RuleBinder {
                    variable: Binder {
                        name: cell.clone(),
                        ty: Type::Coord2,
                    },
                    domain: Expr::Var(cells.clone()),
                    quantifier: Quantifier::ForEach,
                }],
                guard: Expr::Bool(true),
                effects: vec![Effect::Write {
                    target: Expr::Var(cell),
                    value: Expr::Atom(9),
                }],
                priority: 0,
            }],
            schedule: Schedule::Parallel,
            renderer: Renderer {
                canvas: Expr::Int(2),
                layers: Vec::new(),
                conflict_policy: ConflictPolicy::Reject,
                background: Expr::Atom(0),
            },
            assumptions: vec![Expr::Bool(true)],
            proof_annotations: Vec::new(),
        })
        .expect("checked theory");
    let coordinates = BTreeSet::from([
        Value::Coord2 { row: 0, column: 0 },
        Value::Coord2 { row: 1, column: 1 },
    ]);
    let mut types = TypeEnvironment::new();
    types.insert(cells.clone(), cells_type);
    let mut values = RuntimeEnvironment::new();
    values.insert(
        cells,
        Value::Set {
            element_type: Type::Coord2,
            maximum: 4,
            values: coordinates,
        },
    );
    let request = TheoryExecutionRequest {
        types,
        values,
        initial_facts: BTreeSet::new(),
        initial_state: BTreeMap::new(),
    };
    let limits = TheoryExecLimits {
        expression: execution_limits(),
        maximum_bindings: 100,
        maximum_rule_firings: 100,
    };
    let reference = ReferenceTheoryExecutor
        .execute(&theory, &request, limits)
        .expect("reference theory");
    let optimized = FastTheoryExecutor::new(CompileLimits::default())
        .execute(&theory, &request, limits)
        .expect("optimized theory");

    assert_eq!(optimized.rendered, reference.rendered);
    assert_eq!(optimized.state, reference.state);
    assert_eq!(optimized.rule_firings, reference.rule_firings);
    assert_eq!(optimized.fixpoint_iterations, reference.fixpoint_iterations);
}

#[test]
fn optimized_scalar_control_flow_has_identical_result_and_trace() {
    let expression = Expr::Let {
        binder: VarName("x".to_owned()),
        value: Box::new(Expr::Int(4)),
        body: Box::new(Expr::If {
            condition: Box::new(Expr::Binary {
                op: BinaryOp::LessThan,
                left: Box::new(Expr::Var(VarName("x".to_owned()))),
                right: Box::new(Expr::Int(8)),
            }),
            then_branch: Box::new(Expr::Binary {
                op: BinaryOp::Multiply,
                left: Box::new(Expr::Var(VarName("x".to_owned()))),
                right: Box::new(Expr::Int(3)),
            }),
            else_branch: Box::new(Expr::Int(-1)),
        }),
    };
    let expression = TypeChecker::new(1_000)
        .check(&expression, &TypeEnvironment::new())
        .expect("checked");
    let program = BytecodeCompiler::new(CompileLimits::default())
        .compile(&expression)
        .expect("compiled");
    let reference = ReferenceExecutor::without_definitions(execution_limits(), TraceLevel::Full)
        .execute(&expression, &RuntimeEnvironment::new())
        .expect("reference");
    let optimized = FastExecutor::new(execution_limits(), TraceLevel::Full)
        .execute(&program, &RuntimeEnvironment::new())
        .expect("optimized");

    assert_eq!(optimized, reference);
    assert_eq!(optimized.result, Value::Int(12));
}

#[test]
fn checked_task_local_definition_compiles_to_equivalent_micro_ops() {
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
        .expect("checked theory");
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
                arguments: vec![Expr::Int(9)],
            },
            &TypeEnvironment::new(),
        )
        .expect("checked call");
    let program = BytecodeCompiler::with_theory(&theory, CompileLimits::default())
        .compile(&call)
        .expect("compiled learned definition");
    let reference = ReferenceExecutor::new(&theory, execution_limits(), TraceLevel::Full)
        .execute(&call, &RuntimeEnvironment::new())
        .expect("reference");
    let optimized = FastExecutor::new(execution_limits(), TraceLevel::Full)
        .execute(&program, &RuntimeEnvironment::new())
        .expect("optimized");

    assert_eq!(optimized, reference);
    assert_eq!(optimized.result, Value::Int(18));
    assert_eq!(optimized.maximum_call_depth, 1);
}

#[test]
fn caller_owned_workspace_replays_without_semantic_state() {
    let expression = TypeChecker::new(100)
        .check(
            &Expr::Count(Box::new(Expr::SingletonSet {
                value: Box::new(Expr::Atom(3)),
                maximum: 8,
            })),
            &TypeEnvironment::new(),
        )
        .expect("checked");
    let program = BytecodeCompiler::new(CompileLimits::default())
        .compile(&expression)
        .expect("compiled");
    let executor = FastExecutor::new(execution_limits(), TraceLevel::Off);
    let mut workspace = ExecutionWorkspace::for_program(&program);
    let first = executor
        .execute_in(&program, &RuntimeEnvironment::new(), &mut workspace)
        .expect("first");
    let second = executor
        .execute_in(&program, &RuntimeEnvironment::new(), &mut workspace)
        .expect("second");
    assert_eq!(first, second);
    assert_eq!(second.result, Value::Int(1));
}

#[test]
fn compiler_caps_are_typed_and_bounded_loops_are_equivalent() {
    let scalar = TypeChecker::new(100)
        .check(&Expr::Int(1), &TypeEnvironment::new())
        .expect("scalar");
    let error = BytecodeCompiler::new(CompileLimits {
        maximum_registers: 0,
        maximum_instructions: 8,
    })
    .compile(&scalar)
    .expect_err("register cap");
    assert_eq!(error, CompileError::RegisterCapacity);

    let input = VarName("input".to_owned());
    let state = VarName("state".to_owned());
    let mut types = TypeEnvironment::new();
    types.insert(input.clone(), Type::Int(IntRange { lower: 0, upper: 3 }));
    let loop_expression = TypeChecker::new(100)
        .check(
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
                progress_measure: Box::new(Expr::Var(state)),
            },
            &types,
        )
        .expect("checked bounded loop");
    let program = BytecodeCompiler::new(CompileLimits::default())
        .compile(&loop_expression)
        .expect("bounded loop compiled");
    let mut values = RuntimeEnvironment::new();
    values.insert(input, Value::Int(3));
    let reference = ReferenceExecutor::without_definitions(execution_limits(), TraceLevel::Full)
        .execute(&loop_expression, &values)
        .expect("reference loop");
    let optimized = FastExecutor::new(execution_limits(), TraceLevel::Full)
        .execute(&program, &values)
        .expect("optimized loop");
    assert_eq!(optimized, reference);
    assert_eq!(optimized.result, Value::Int(0));
}

#[test]
fn one_million_generated_terms_match_reference_exactly() {
    let checker = TypeChecker::new(1_000);
    let compiler = BytecodeCompiler::new(CompileLimits::default());
    let reference = ReferenceExecutor::without_definitions(execution_limits(), TraceLevel::Full);
    let optimized = FastExecutor::new(execution_limits(), TraceLevel::Full);
    let environment = RuntimeEnvironment::new();
    let mut seed = 0x5eed_cafe_f00d_beefu64;

    for _ in 0..1_000_000 {
        let expression = generated_int(&mut seed, 0);
        let expression = checker
            .check(&expression, &TypeEnvironment::new())
            .expect("generated expression checks");
        let program = compiler
            .compile(&expression)
            .expect("generated compilation");
        let expected = reference
            .execute(&expression, &environment)
            .expect("reference execution");
        let actual = optimized
            .execute(&program, &environment)
            .expect("optimized execution");
        assert_eq!(actual, expected);
    }
}

fn generated_int(state: &mut u64, depth: usize) -> Expr {
    if depth >= 3 || next(state) % 4 == 0 {
        return Expr::Int(i64::from(next(state).to_le_bytes()[0] % 11) - 5);
    }
    match next(state) % 6 {
        0 => Expr::Unary {
            op: UnaryOp::Negate,
            argument: Box::new(generated_int(state, depth + 1)),
        },
        1 => Expr::If {
            condition: Box::new(generated_bool(state, depth + 1)),
            then_branch: Box::new(generated_int(state, depth + 1)),
            else_branch: Box::new(generated_int(state, depth + 1)),
        },
        choice => Expr::Binary {
            op: match choice {
                2 => BinaryOp::Add,
                3 => BinaryOp::Subtract,
                4 => BinaryOp::Minimum,
                _ => BinaryOp::Maximum,
            },
            left: Box::new(generated_int(state, depth + 1)),
            right: Box::new(generated_int(state, depth + 1)),
        },
    }
}

fn generated_bool(state: &mut u64, depth: usize) -> Expr {
    if depth >= 3 || next(state) % 4 == 0 {
        return Expr::Bool(next(state) & 1 == 1);
    }
    match next(state) % 4 {
        0 => Expr::Unary {
            op: UnaryOp::Not,
            argument: Box::new(generated_bool(state, depth + 1)),
        },
        1 => Expr::Binary {
            op: BinaryOp::LessThan,
            left: Box::new(generated_int(state, depth + 1)),
            right: Box::new(generated_int(state, depth + 1)),
        },
        choice => Expr::Binary {
            op: if choice == 2 {
                BinaryOp::And
            } else {
                BinaryOp::Or
            },
            left: Box::new(generated_bool(state, depth + 1)),
            right: Box::new(generated_bool(state, depth + 1)),
        },
    }
}

fn next(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    *state
}
