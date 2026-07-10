use cairn_core::{decode_canonical, CanonicalEncode, CanonicalLimits};
use cairn_rkl::{
    parse_debug_expr, BinaryOp, Binder, BoundChecker, Canonicalizer, CheckErrorKind, CostModel,
    DefaultCanonicalizer, Definition, DefinitionSignature, Expr, IntRange, RecursionBound,
    StructuralCostModel, Type, TypeChecker, TypeEnvironment, UnaryOp, VarName,
};
use std::collections::BTreeMap;

#[test]
fn debug_parser_feeds_exact_type_and_range_checker() {
    let expression = parse_debug_expr("(let amount (add 1 2) (mul amount 3))").expect("parse");
    let typed = checker()
        .check(&expression, &TypeEnvironment::new())
        .expect("check");
    assert_eq!(typed.ty(), &Type::Int(IntRange::exact(9)));
    assert!(typed.free_variables().is_empty());
}

#[test]
fn errors_carry_precise_path_expected_and_actual_types() {
    let expression = parse_debug_expr("(if 1 true false)").expect("parse");
    let error = checker()
        .check(&expression, &TypeEnvironment::new())
        .expect_err("condition must fail");
    assert_eq!(error.path.to_string(), "$.condition");
    assert!(matches!(
        error.kind,
        CheckErrorKind::TypeMismatch {
            expected: Type::Bool,
            actual: Type::Int(IntRange { lower: 1, upper: 1 })
        }
    ));
}

#[test]
fn alpha_equivalent_binders_have_identical_semantic_hashes() {
    let first = parse_debug_expr("(let source 4 (add source 2))").expect("parse first");
    let second = parse_debug_expr("(let renamed 4 (add renamed 2))").expect("parse second");
    let first = checker()
        .check(&first, &TypeEnvironment::new())
        .expect("first");
    let second = checker()
        .check(&second, &TypeEnvironment::new())
        .expect("second");
    let canonicalizer = DefaultCanonicalizer;
    assert_eq!(
        canonicalizer
            .canonicalize(&first)
            .expect("canonical first")
            .semantic_hash,
        canonicalizer
            .canonicalize(&second)
            .expect("canonical second")
            .semantic_hash
    );
}

#[test]
fn associative_commutative_forms_canonicalize_identically() {
    let first = parse_debug_expr("(add 1 (add 2 3))").expect("first");
    let second = parse_debug_expr("(add 3 (add 1 2))").expect("second");
    let first = checker()
        .check(&first, &TypeEnvironment::new())
        .expect("check first");
    let second = checker()
        .check(&second, &TypeEnvironment::new())
        .expect("check second");
    assert_eq!(first.ty(), second.ty());
    assert_eq!(
        first.encode_canonical().expect("first bytes"),
        second.encode_canonical().expect("second bytes")
    );
}

#[test]
fn proof_annotations_do_not_enter_semantic_identity() {
    let expression = Expr::Int(7);
    let mut first = checker()
        .check(&expression, &TypeEnvironment::new())
        .expect("first");
    let mut second = first.clone();
    first.add_proof_annotation(cairn_core::Hash32::digest(b"proof-a"));
    second.add_proof_annotation(cairn_core::Hash32::digest(b"proof-b"));
    assert_eq!(
        first.encode_canonical().expect("first"),
        second.encode_canonical().expect("second")
    );
}

#[test]
fn categorical_atoms_are_distinct_from_integers() {
    let atom = checker()
        .check(&Expr::Atom(7), &TypeEnvironment::new())
        .expect("atom");
    let integer = checker()
        .check(&Expr::Int(7), &TypeEnvironment::new())
        .expect("integer");
    assert_eq!(atom.ty(), &Type::Atom);
    assert_ne!(
        atom.encode_canonical().expect("atom bytes"),
        integer.encode_canonical().expect("integer bytes")
    );
}

#[test]
fn finite_fields_require_axes_and_element_bounds() {
    for invalid in [
        Type::FiniteField {
            axes: 0,
            maximum_elements: 16,
        },
        Type::FiniteField {
            axes: 2,
            maximum_elements: 0,
        },
    ] {
        let mut environment = TypeEnvironment::new();
        environment.insert(VarName("field".to_owned()), invalid);
        assert!(matches!(
            checker().check(&Expr::Var(VarName("field".to_owned())), &environment),
            Err(error) if error.kind == CheckErrorKind::EmptyBound
        ));
    }

    let valid = Type::FiniteField {
        axes: 3,
        maximum_elements: 4_096,
    };
    let mut environment = TypeEnvironment::new();
    environment.insert(VarName("field".to_owned()), valid.clone());
    let typed = checker()
        .check(&Expr::Var(VarName("field".to_owned())), &environment)
        .expect("bounded finite field");
    assert_eq!(typed.ty(), &valid);
}

#[test]
fn inferred_integer_bounds_cover_exhaustive_small_domain() {
    let mut environment = TypeEnvironment::new();
    environment.insert(
        VarName("x".to_owned()),
        Type::Int(IntRange {
            lower: -3,
            upper: 3,
        }),
    );
    environment.insert(
        VarName("y".to_owned()),
        Type::Int(IntRange {
            lower: -2,
            upper: 2,
        }),
    );
    let expression = Expr::Binary {
        op: BinaryOp::Add,
        left: Box::new(Expr::Binary {
            op: BinaryOp::Multiply,
            left: Box::new(Expr::Var(VarName("x".to_owned()))),
            right: Box::new(Expr::Var(VarName("y".to_owned()))),
        }),
        right: Box::new(Expr::Int(1)),
    };
    let typed = checker().check(&expression, &environment).expect("check");
    let Type::Int(range) = typed.ty() else {
        panic!("expected integer range");
    };
    for x in -3..=3 {
        for y in -2..=2 {
            let value = x * y + 1;
            assert!(range.lower <= value && value <= range.upper);
        }
    }
}

#[test]
fn malformed_recursion_and_bound_overflow_fail_closed() {
    let range = Type::Int(IntRange {
        lower: -10,
        upper: 10,
    });
    let mut checker = checker();
    checker.register_definition(
        "loop".to_owned(),
        DefinitionSignature {
            parameters: vec![range.clone()],
            return_type: range.clone(),
        },
    );
    let mut definition = Definition {
        name: "loop".to_owned(),
        parameters: vec![Binder {
            name: VarName("n".to_owned()),
            ty: range.clone(),
        }],
        return_type: range,
        body: Expr::Call {
            function: "loop".to_owned(),
            arguments: vec![Expr::Var(VarName("n".to_owned()))],
        },
        recursion: None,
        proof_annotations: Vec::new(),
    };
    assert!(matches!(
        checker.check_definition(&definition),
        Err(error) if matches!(error.kind, CheckErrorKind::RecursiveCallWithoutBound(_))
    ));
    definition.recursion = Some(RecursionBound {
        maximum_steps: 4,
        progress_measure: Expr::Var(VarName("n".to_owned())),
    });
    checker
        .check_definition(&definition)
        .expect("bounded recursion");

    let overflow = Expr::Binary {
        op: BinaryOp::Add,
        left: Box::new(Expr::Int(i64::MAX)),
        right: Box::new(Expr::Int(1)),
    };
    assert!(matches!(
        checker.check(&overflow, &TypeEnvironment::new()),
        Err(error) if error.kind == CheckErrorKind::BoundOverflow
    ));
}

#[test]
fn bounded_fixpoint_requires_fuel_and_integer_progress() {
    let no_fuel = Expr::BoundedFixpoint {
        binder: VarName("state".to_owned()),
        initial: Box::new(Expr::Int(0)),
        step: Box::new(Expr::Var(VarName("state".to_owned()))),
        maximum_steps: 0,
        progress_measure: Box::new(Expr::Var(VarName("state".to_owned()))),
    };
    assert!(matches!(
        checker().check(&no_fuel, &TypeEnvironment::new()),
        Err(error) if error.kind == CheckErrorKind::EmptyBound
    ));
}

#[test]
fn structural_cost_accounts_for_fixpoint_fuel() {
    let expression = Expr::BoundedFixpoint {
        binder: VarName("state".to_owned()),
        initial: Box::new(Expr::Int(0)),
        step: Box::new(Expr::Binary {
            op: BinaryOp::Add,
            left: Box::new(Expr::Var(VarName("state".to_owned()))),
            right: Box::new(Expr::Int(1)),
        }),
        maximum_steps: 10,
        progress_measure: Box::new(Expr::Var(VarName("state".to_owned()))),
    };
    let typed = checker()
        .check(&expression, &TypeEnvironment::new())
        .expect("check");
    let cost = StructuralCostModel.cost(&typed).expect("cost");
    assert!(cost.maximum_execution_steps >= 30);
}

#[test]
fn one_hundred_thousand_generated_terms_are_stable_and_in_bounds() {
    let checker = checker();
    let canonicalizer = DefaultCanonicalizer;
    let mut state = 0x6a09_e667_f3bc_c909_u64;
    for index in 0..100_000 {
        let integer = index & 1 == 0;
        let expression = if integer {
            generated_int(&mut state, 0)
        } else {
            generated_bool(&mut state, 0)
        };
        let first = checker
            .check(&expression, &TypeEnvironment::new())
            .expect("first check");
        let second = checker
            .check(&expression, &TypeEnvironment::new())
            .expect("second check");
        assert_eq!(first.ty(), second.ty());
        let first_form = canonicalizer.canonicalize(&first).expect("first canonical");
        let second_form = canonicalizer
            .canonicalize(&second)
            .expect("second canonical");
        assert_eq!(first_form.bytes, second_form.bytes);
        assert_eq!(first_form.semantic_hash, second_form.semantic_hash);
        let replay = decode_canonical(&first_form.bytes, CanonicalLimits::default())
            .expect("canonical replay");
        assert_eq!(
            replay.encode_canonical().expect("reencode"),
            first_form.bytes
        );
        if let Type::Int(range) = first.ty() {
            let value = evaluate_int(&expression, &mut BTreeMap::new());
            assert!(range.lower <= value && value <= range.upper);
        }
    }
}

fn checker() -> TypeChecker {
    TypeChecker::new(10_000)
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

fn evaluate_int(expression: &Expr, environment: &mut BTreeMap<String, Scalar>) -> i64 {
    match evaluate(expression, environment) {
        Scalar::Int(value) => value,
        Scalar::Bool(_) => panic!("expected integer"),
    }
}

fn evaluate_bool(expression: &Expr, environment: &mut BTreeMap<String, Scalar>) -> bool {
    match evaluate(expression, environment) {
        Scalar::Bool(value) => value,
        Scalar::Int(_) => panic!("expected Boolean"),
    }
}

#[derive(Clone, Copy)]
enum Scalar {
    Int(i64),
    Bool(bool),
}

fn evaluate(expression: &Expr, environment: &mut BTreeMap<String, Scalar>) -> Scalar {
    match expression {
        Expr::Bool(value) => Scalar::Bool(*value),
        Expr::Int(value) => Scalar::Int(*value),
        Expr::Var(name) => *environment.get(&name.0).expect("bound variable"),
        Expr::If {
            condition,
            then_branch,
            else_branch,
        } => {
            if evaluate_bool(condition, environment) {
                evaluate(then_branch, environment)
            } else {
                evaluate(else_branch, environment)
            }
        }
        Expr::Let {
            binder,
            value,
            body,
        } => {
            let value = evaluate(value, environment);
            let previous = environment.insert(binder.0.clone(), value);
            let result = evaluate(body, environment);
            if let Some(previous) = previous {
                environment.insert(binder.0.clone(), previous);
            } else {
                environment.remove(&binder.0);
            }
            result
        }
        Expr::Unary { op, argument } => match op {
            UnaryOp::Not => Scalar::Bool(!evaluate_bool(argument, environment)),
            UnaryOp::Negate => Scalar::Int(-evaluate_int(argument, environment)),
        },
        Expr::Binary { op, left, right } => match op {
            BinaryOp::Add => {
                Scalar::Int(evaluate_int(left, environment) + evaluate_int(right, environment))
            }
            BinaryOp::Subtract => {
                Scalar::Int(evaluate_int(left, environment) - evaluate_int(right, environment))
            }
            BinaryOp::Multiply => {
                Scalar::Int(evaluate_int(left, environment) * evaluate_int(right, environment))
            }
            BinaryOp::Minimum => {
                Scalar::Int(evaluate_int(left, environment).min(evaluate_int(right, environment)))
            }
            BinaryOp::Maximum => {
                Scalar::Int(evaluate_int(left, environment).max(evaluate_int(right, environment)))
            }
            BinaryOp::And => {
                Scalar::Bool(evaluate_bool(left, environment) && evaluate_bool(right, environment))
            }
            BinaryOp::Or => {
                Scalar::Bool(evaluate_bool(left, environment) || evaluate_bool(right, environment))
            }
            BinaryOp::Equal => {
                Scalar::Bool(evaluate_int(left, environment) == evaluate_int(right, environment))
            }
            BinaryOp::LessThan => {
                Scalar::Bool(evaluate_int(left, environment) < evaluate_int(right, environment))
            }
            BinaryOp::Union | BinaryOp::Intersection | BinaryOp::Difference => {
                panic!("set expression in scalar generator")
            }
        },
        _ => panic!("unsupported scalar test expression"),
    }
}

fn next(state: &mut u64) -> u64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state
}
