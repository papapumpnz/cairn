use cairn_invent::{BindingExample, InventionLimits, InventionProblem};
use cairn_rkl::{
    Binder, ExecLimits, Expr, IntRange, RuntimeEnvironment, Schedule, Type, Value, VarName,
};
use cairn_theory::{
    BindingTable, ConstructorSynthesizer, GuardInducer, GuardLimits, PredicateColumn,
    RuleFootprint, ScheduleFailure, ScheduleSynthesizer,
};
use std::collections::BTreeSet;

#[test]
fn conjunction_is_invented_from_positive_and_hard_negative_bindings() {
    let predicates = vec![
        PredicateColumn {
            expression: Expr::Bool(true),
            true_rows: BTreeSet::from([0, 1, 2]),
            structural_cost: 1,
        },
        PredicateColumn {
            expression: Expr::Bool(true),
            true_rows: BTreeSet::from([0, 1, 3]),
            structural_cost: 1,
        },
    ];
    let guard = GuardInducer
        .induce(
            &BindingTable {
                positive_rows: BTreeSet::from([0, 1]),
                negative_rows: BTreeSet::from([2, 3]),
            },
            &predicates,
            GuardLimits {
                maximum_conjuncts: 2,
                maximum_candidates: 20,
            },
        )
        .expect("guard");
    assert_eq!(guard.predicates, vec![0, 1]);
}

#[test]
fn constructor_is_learned_as_executable_rkl_not_output_lookup() {
    let name = VarName("count".to_owned());
    let ty = Type::Int(IntRange {
        lower: -10,
        upper: 10,
    });
    let examples = [(-2, 1), (0, 3), (4, 7)]
        .into_iter()
        .map(|(input, output)| {
            let mut inputs = RuntimeEnvironment::new();
            inputs.insert(name.clone(), Value::Int(input));
            BindingExample {
                inputs,
                output: Value::Int(output),
            }
        })
        .collect();
    let learned = ConstructorSynthesizer
        .synthesize(&InventionProblem {
            definition_name: "construct".to_owned(),
            parameters: vec![Binder {
                name,
                ty: ty.clone(),
            }],
            return_type: Type::Int(IntRange {
                lower: -20,
                upper: 20,
            }),
            literals: vec![Value::Int(3)],
            demonstrations: examples,
            limits: InventionLimits {
                maximum_cost: 5,
                maximum_candidates: 10_000,
                maximum_expression_nodes: 1_000,
                execution: ExecLimits::default(),
            },
        })
        .expect("constructor");
    assert!(learned.admission.exact_cross_implementation_replays >= 3);
}

#[test]
fn schedule_uses_dependencies_and_refuses_unexplained_conflicts() {
    let key = Value::Atom(1);
    let schedule = ScheduleSynthesizer
        .synthesize(&[
            RuleFootprint {
                name: "produce".to_owned(),
                reads: BTreeSet::new(),
                writes: BTreeSet::from([key.clone()]),
            },
            RuleFootprint {
                name: "consume".to_owned(),
                reads: BTreeSet::from([key]),
                writes: BTreeSet::from([Value::Atom(2)]),
            },
        ])
        .expect("schedule");
    assert_eq!(
        schedule,
        Schedule::DependencyDag(vec![("produce".to_owned(), "consume".to_owned())])
    );

    let conflict = ScheduleSynthesizer.synthesize(&[
        RuleFootprint {
            name: "a".to_owned(),
            reads: BTreeSet::new(),
            writes: BTreeSet::from([Value::Atom(9)]),
        },
        RuleFootprint {
            name: "b".to_owned(),
            reads: BTreeSet::new(),
            writes: BTreeSet::from([Value::Atom(9)]),
        },
    ]);
    assert!(matches!(
        conflict,
        Err(ScheduleFailure::ConflictingWrites { .. })
    ));
}
