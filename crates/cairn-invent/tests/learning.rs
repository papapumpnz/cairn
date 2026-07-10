use cairn_invent::{
    BindingExample, DefinitionInventor, InventionError, InventionLimits, InventionProblem,
};
use cairn_rkl::{Binder, ExecLimits, IntRange, RuntimeEnvironment, Type, Value, VarName};
use std::collections::BTreeSet;

fn int_type(lower: i64, upper: i64) -> Type {
    Type::Int(IntRange { lower, upper })
}

fn limits(maximum_cost: u16) -> InventionLimits {
    InventionLimits {
        maximum_cost,
        maximum_candidates: 1_000_000,
        maximum_expression_nodes: 1_000,
        execution: ExecLimits {
            fuel: 10_000,
            maximum_call_depth: 16,
            maximum_trace_events: 10_000,
        },
    }
}

fn environment(bindings: &[(&str, Value)]) -> RuntimeEnvironment {
    let mut environment = RuntimeEnvironment::new();
    for (name, value) in bindings {
        environment.insert(VarName((*name).to_owned()), value.clone());
    }
    environment
}

fn example(bindings: &[(&str, Value)], output: Value) -> BindingExample {
    BindingExample {
        inputs: environment(bindings),
        output,
    }
}

#[test]
fn invents_absolute_magnitude_and_transfers_to_unseen_readings() {
    let demonstrations = [-9_i64, -4, -1, 0, 2, 7]
        .into_iter()
        .map(|reading| {
            example(
                &[("reading", Value::Int(reading))],
                Value::Int(reading.abs()),
            )
        })
        .collect();
    let problem = InventionProblem {
        definition_name: "invented_magnitude".to_owned(),
        parameters: vec![Binder {
            name: VarName("reading".to_owned()),
            ty: int_type(-10, 10),
        }],
        return_type: int_type(0, 10),
        literals: vec![Value::Int(0)],
        demonstrations,
        limits: limits(6),
    };

    let outcome = DefinitionInventor.invent(&problem).expect("invent concept");
    assert_eq!(outcome.admission.exact_reference_replays, 6);
    assert_eq!(outcome.admission.exact_cross_implementation_replays, 6);
    for reading in [-10_i64, -8, 3, 6, 10] {
        let prediction = DefinitionInventor
            .predict(
                &outcome.proposal,
                &environment(&[("reading", Value::Int(reading))]),
                problem.limits.execution,
            )
            .expect("unseen prediction");
        assert_eq!(prediction, Value::Int(reading.abs()));
    }
}

#[test]
fn invents_clamping_and_rebinds_changed_limits() {
    let cases = [
        (-9, -3, 4, -3),
        (9, -3, 4, 4),
        (1, -3, 4, 1),
        (-8, -5, 2, -5),
        (7, -5, 2, 2),
        (0, -5, 2, 0),
    ];
    let problem = InventionProblem {
        definition_name: "invented_bound".to_owned(),
        parameters: vec![
            Binder {
                name: VarName("value".to_owned()),
                ty: int_type(-20, 20),
            },
            Binder {
                name: VarName("lower".to_owned()),
                ty: int_type(-10, 0),
            },
            Binder {
                name: VarName("upper".to_owned()),
                ty: int_type(0, 10),
            },
        ],
        return_type: int_type(-10, 10),
        literals: Vec::new(),
        demonstrations: cases
            .into_iter()
            .map(|(value, lower, upper, output)| {
                example(
                    &[
                        ("value", Value::Int(value)),
                        ("lower", Value::Int(lower)),
                        ("upper", Value::Int(upper)),
                    ],
                    Value::Int(output),
                )
            })
            .collect(),
        limits: limits(6),
    };

    let outcome = DefinitionInventor.invent(&problem).expect("invent clamp");
    for (value, lower, upper, expected) in [(-20, -7, 6, -7), (20, -7, 6, 6), (4, -7, 6, 4)] {
        let prediction = DefinitionInventor
            .predict(
                &outcome.proposal,
                &environment(&[
                    ("value", Value::Int(value)),
                    ("lower", Value::Int(lower)),
                    ("upper", Value::Int(upper)),
                ]),
                problem.limits.execution,
            )
            .expect("changed-bound prediction");
        assert_eq!(prediction, Value::Int(expected));
    }
}

#[test]
fn invents_relational_symbol_selection_and_rebinds_unseen_atoms() {
    let cases = [
        (-4, 0, 11, 23, 11),
        (3, 0, 11, 23, 23),
        (2, 5, 31, 47, 31),
        (8, 5, 31, 47, 47),
        (-2, -1, 59, 61, 59),
        (4, -1, 59, 61, 61),
    ];
    let problem = InventionProblem {
        definition_name: "invented_control_binding".to_owned(),
        parameters: vec![
            Binder {
                name: VarName("reading".to_owned()),
                ty: int_type(-10, 10),
            },
            Binder {
                name: VarName("threshold".to_owned()),
                ty: int_type(-10, 10),
            },
            Binder {
                name: VarName("below_symbol".to_owned()),
                ty: Type::Atom,
            },
            Binder {
                name: VarName("other_symbol".to_owned()),
                ty: Type::Atom,
            },
        ],
        return_type: Type::Atom,
        literals: Vec::new(),
        demonstrations: cases
            .into_iter()
            .map(|(reading, threshold, below, other, output)| {
                example(
                    &[
                        ("reading", Value::Int(reading)),
                        ("threshold", Value::Int(threshold)),
                        ("below_symbol", Value::Atom(below)),
                        ("other_symbol", Value::Atom(other)),
                    ],
                    Value::Atom(output),
                )
            })
            .collect(),
        limits: limits(7),
    };

    let outcome = DefinitionInventor
        .invent(&problem)
        .expect("invent symbol control");
    for (reading, threshold, expected) in [(-8, -3, 101), (6, -3, 103)] {
        let prediction = DefinitionInventor
            .predict(
                &outcome.proposal,
                &environment(&[
                    ("reading", Value::Int(reading)),
                    ("threshold", Value::Int(threshold)),
                    ("below_symbol", Value::Atom(101)),
                    ("other_symbol", Value::Atom(103)),
                ]),
                problem.limits.execution,
            )
            .expect("unseen symbol prediction");
        assert_eq!(prediction, Value::Atom(expected));
    }
}

#[test]
fn invents_collection_cardinality_adjustment_across_changed_counts() {
    let collection_type = Type::List {
        element: Box::new(Type::Atom),
        maximum: 16,
    };
    let cases = [(0_u16, -2_i64), (1, 3), (3, -1), (5, 2), (8, -3)];
    let problem = InventionProblem {
        definition_name: "invented_inventory_adjustment".to_owned(),
        parameters: vec![
            Binder {
                name: VarName("items".to_owned()),
                ty: collection_type.clone(),
            },
            Binder {
                name: VarName("offset".to_owned()),
                ty: int_type(-5, 5),
            },
        ],
        return_type: int_type(-5, 21),
        literals: Vec::new(),
        demonstrations: cases
            .into_iter()
            .map(|(count, offset)| {
                example(
                    &[
                        ("items", atom_set(count, 16)),
                        ("offset", Value::Int(offset)),
                    ],
                    Value::Int(i64::from(count) + offset),
                )
            })
            .collect(),
        limits: limits(6),
    };

    let outcome = DefinitionInventor
        .invent(&problem)
        .expect("invent count adjustment");
    for (count, offset) in [(2_u16, 4_i64), (7, 0), (12, -4)] {
        let prediction = DefinitionInventor
            .predict(
                &outcome.proposal,
                &environment(&[
                    ("items", atom_set(count, 16)),
                    ("offset", Value::Int(offset)),
                ]),
                problem.limits.execution,
            )
            .expect("changed-count prediction");
        assert_eq!(prediction, Value::Int(i64::from(count) + offset));
    }
}

#[test]
fn leave_one_demonstration_out_relearns_magnitude_without_the_held_out_answer() {
    let readings = [-9_i64, -5, -2, 1, 4, 8];
    for held_out in 0..readings.len() {
        let problem = InventionProblem {
            definition_name: "invented_loo".to_owned(),
            parameters: vec![Binder {
                name: VarName("reading".to_owned()),
                ty: int_type(-10, 10),
            }],
            return_type: int_type(0, 10),
            literals: vec![Value::Int(0)],
            demonstrations: readings
                .iter()
                .enumerate()
                .filter(|(index, _)| *index != held_out)
                .map(|(_, reading)| {
                    example(
                        &[("reading", Value::Int(*reading))],
                        Value::Int(reading.abs()),
                    )
                })
                .collect(),
            limits: limits(6),
        };
        let outcome = DefinitionInventor
            .invent(&problem)
            .expect("relearn without held-out example");
        let reading = readings[held_out];
        let prediction = DefinitionInventor
            .predict(
                &outcome.proposal,
                &environment(&[("reading", Value::Int(reading))]),
                problem.limits.execution,
            )
            .expect("predict held-out example");
        assert_eq!(prediction, Value::Int(reading.abs()));
    }
}

#[test]
fn insufficient_candidate_budget_is_not_reported_as_no_concept() {
    let problem = InventionProblem {
        definition_name: "bounded_search".to_owned(),
        parameters: vec![Binder {
            name: VarName("value".to_owned()),
            ty: int_type(-10, 10),
        }],
        return_type: int_type(0, 10),
        literals: vec![Value::Int(0)],
        demonstrations: vec![
            example(&[("value", Value::Int(-2))], Value::Int(2)),
            example(&[("value", Value::Int(3))], Value::Int(3)),
        ],
        limits: InventionLimits {
            maximum_candidates: 1,
            ..limits(6)
        },
    };
    assert_eq!(
        DefinitionInventor
            .invent(&problem)
            .expect_err("capacity is explicit"),
        InventionError::CandidateCapacity { maximum: 1 }
    );
}

fn atom_set(count: u16, maximum: u32) -> Value {
    Value::Set {
        element_type: Type::Atom,
        maximum,
        values: (0..count).map(Value::Atom).collect::<BTreeSet<_>>(),
    }
}
