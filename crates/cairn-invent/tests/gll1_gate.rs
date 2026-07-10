use cairn_invent::{
    BindingExample, DefinitionInventor, InventionError, InventionLimits, InventionOutcome,
    InventionProblem,
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

fn magnitude_problem() -> InventionProblem {
    InventionProblem {
        definition_name: "invented_magnitude".to_owned(),
        parameters: vec![Binder {
            name: VarName("reading".to_owned()),
            ty: int_type(-10, 10),
        }],
        return_type: int_type(0, 10),
        literals: vec![Value::Int(0)],
        demonstrations: [-9_i64, -4, -1, 0, 2, 7]
            .into_iter()
            .map(|reading| {
                example(
                    &[("reading", Value::Int(reading))],
                    Value::Int(reading.abs()),
                )
            })
            .collect(),
        limits: limits(7),
    }
}

fn clamp_problem() -> InventionProblem {
    InventionProblem {
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
        demonstrations: [
            (-9, -3, 4, -3),
            (9, -3, 4, 4),
            (1, -3, 4, 1),
            (-8, -5, 2, -5),
            (7, -5, 2, 2),
            (0, -5, 2, 0),
        ]
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
        limits: limits(7),
    }
}

fn symbol_problem() -> InventionProblem {
    InventionProblem {
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
        demonstrations: [
            (-4, 0, 11, 23, 11),
            (3, 0, 11, 23, 23),
            (2, 5, 31, 47, 31),
            (8, 5, 31, 47, 47),
            (-2, -1, 59, 61, 59),
            (4, -1, 59, 61, 61),
        ]
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
    }
}

fn count_problem() -> InventionProblem {
    InventionProblem {
        definition_name: "invented_inventory_adjustment".to_owned(),
        parameters: vec![
            Binder {
                name: VarName("items".to_owned()),
                ty: Type::List {
                    element: Box::new(Type::Atom),
                    maximum: 16,
                },
            },
            Binder {
                name: VarName("offset".to_owned()),
                ty: int_type(-5, 5),
            },
        ],
        return_type: int_type(-5, 21),
        literals: Vec::new(),
        demonstrations: [(0_u16, -2_i64), (1, 3), (3, -1), (5, 2), (8, -3)]
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
        limits: limits(7),
    }
}

fn geometry_problem() -> InventionProblem {
    let coordinate = int_type(-25, 25);
    InventionProblem {
        definition_name: "invented_translation".to_owned(),
        parameters: vec![
            Binder {
                name: VarName("origin_row".to_owned()),
                ty: int_type(-20, 20),
            },
            Binder {
                name: VarName("origin_column".to_owned()),
                ty: int_type(-20, 20),
            },
            Binder {
                name: VarName("row_shift".to_owned()),
                ty: int_type(-5, 5),
            },
            Binder {
                name: VarName("column_shift".to_owned()),
                ty: int_type(-5, 5),
            },
        ],
        return_type: Type::Tuple(vec![coordinate.clone(), coordinate]),
        literals: Vec::new(),
        demonstrations: [
            (-10, 7, 3, -2),
            (4, -9, -3, 5),
            (0, 0, 2, 4),
            (12, 8, -4, -3),
            (-6, -11, 5, 1),
            (9, -2, 0, -5),
        ]
        .into_iter()
        .map(|(row, column, row_shift, column_shift)| {
            example(
                &[
                    ("origin_row", Value::Int(row)),
                    ("origin_column", Value::Int(column)),
                    ("row_shift", Value::Int(row_shift)),
                    ("column_shift", Value::Int(column_shift)),
                ],
                Value::Tuple(vec![
                    Value::Int(row + row_shift),
                    Value::Int(column + column_shift),
                ]),
            )
        })
        .collect(),
        limits: limits(8),
    }
}

#[test]
#[allow(clippy::too_many_lines)]
fn frozen_gll1_omitted_concepts_transfer_and_survive_ablation() {
    let inventor = DefinitionInventor;
    let magnitude_problem = magnitude_problem();
    let magnitude = inventor
        .invent(&magnitude_problem)
        .expect("magnitude invention");
    let mut magnitude_predictions = 0_u64;
    for reading in -10_i64..=10 {
        assert_prediction(
            &magnitude,
            &environment(&[("reading", Value::Int(reading))]),
            Value::Int(reading.abs()),
            magnitude_problem.limits.execution,
        );
        magnitude_predictions += 1;
    }

    let clamp_problem = clamp_problem();
    let clamp = inventor.invent(&clamp_problem).expect("clamp invention");
    let mut clamp_predictions = 0_u64;
    for value in -20_i64..=20 {
        for lower in -10_i64..=0 {
            for upper in 0_i64..=10 {
                assert_prediction(
                    &clamp,
                    &environment(&[
                        ("value", Value::Int(value)),
                        ("lower", Value::Int(lower)),
                        ("upper", Value::Int(upper)),
                    ]),
                    Value::Int(value.max(lower).min(upper)),
                    clamp_problem.limits.execution,
                );
                clamp_predictions += 1;
            }
        }
    }

    let symbol_problem = symbol_problem();
    let symbol = inventor.invent(&symbol_problem).expect("symbol invention");
    let mut symbol_predictions = 0_u64;
    for reading in -10_i64..=10 {
        for threshold in -10_i64..=10 {
            let expected = if reading < threshold { 1009 } else { 1013 };
            assert_prediction(
                &symbol,
                &environment(&[
                    ("reading", Value::Int(reading)),
                    ("threshold", Value::Int(threshold)),
                    ("below_symbol", Value::Atom(1009)),
                    ("other_symbol", Value::Atom(1013)),
                ]),
                Value::Atom(expected),
                symbol_problem.limits.execution,
            );
            symbol_predictions += 1;
        }
    }

    let count_problem = count_problem();
    let count = inventor.invent(&count_problem).expect("count invention");
    let mut count_predictions = 0_u64;
    for item_count in 0_u16..=16 {
        for offset in -5_i64..=5 {
            assert_prediction(
                &count,
                &environment(&[
                    ("items", atom_set(item_count, 16)),
                    ("offset", Value::Int(offset)),
                ]),
                Value::Int(i64::from(item_count) + offset),
                count_problem.limits.execution,
            );
            count_predictions += 1;
        }
    }

    let geometry_problem = geometry_problem();
    let geometry = inventor
        .invent(&geometry_problem)
        .expect("geometry invention");
    let mut geometry_predictions = 0_u64;
    for origin_row in [-20_i64, -7, 0, 9, 20] {
        for origin_column in [-20_i64, -3, 0, 14, 20] {
            for row_shift in [-5_i64, 0, 5] {
                for column_shift in [-5_i64, 0, 5] {
                    assert_prediction(
                        &geometry,
                        &environment(&[
                            ("origin_row", Value::Int(origin_row)),
                            ("origin_column", Value::Int(origin_column)),
                            ("row_shift", Value::Int(row_shift)),
                            ("column_shift", Value::Int(column_shift)),
                        ]),
                        Value::Tuple(vec![
                            Value::Int(origin_row + row_shift),
                            Value::Int(origin_column + column_shift),
                        ]),
                        geometry_problem.limits.execution,
                    );
                    geometry_predictions += 1;
                }
            }
        }
    }

    for (problem, learned) in [
        (magnitude_problem, &magnitude),
        (clamp_problem, &clamp),
        (symbol_problem, &symbol),
        (count_problem, &count),
        (geometry_problem, &geometry),
    ] {
        let mut ablated = problem;
        ablated.limits.maximum_cost = learned.proposal.structural_cost - 1;
        assert!(matches!(
            inventor.invent(&ablated),
            Err(InventionError::NoDefinition { .. })
        ));
    }

    println!(
        "GLL1 family=magnitude cost={} candidates={} classes={} exact_predictions={magnitude_predictions} body={:?}",
        magnitude.proposal.structural_cost,
        magnitude.proposal.candidates_evaluated,
        magnitude.proposal.observational_classes,
        magnitude.proposal.definition.body
    );
    println!(
        "GLL1 family=clamp cost={} candidates={} classes={} exact_predictions={clamp_predictions} body={:?}",
        clamp.proposal.structural_cost,
        clamp.proposal.candidates_evaluated,
        clamp.proposal.observational_classes,
        clamp.proposal.definition.body
    );
    println!(
        "GLL1 family=symbol-rebinding cost={} candidates={} classes={} exact_predictions={symbol_predictions} body={:?}",
        symbol.proposal.structural_cost,
        symbol.proposal.candidates_evaluated,
        symbol.proposal.observational_classes,
        symbol.proposal.definition.body
    );
    println!(
        "GLL1 family=collection-count cost={} candidates={} classes={} exact_predictions={count_predictions} body={:?}",
        count.proposal.structural_cost,
        count.proposal.candidates_evaluated,
        count.proposal.observational_classes,
        count.proposal.definition.body
    );
    println!(
        "GLL1 family=geometry-translation cost={} candidates={} classes={} exact_predictions={geometry_predictions} body={:?}",
        geometry.proposal.structural_cost,
        geometry.proposal.candidates_evaluated,
        geometry.proposal.observational_classes,
        geometry.proposal.definition.body
    );
}

#[test]
fn frozen_gll1_leave_one_out_reinvents_without_answer_access() {
    for problem in [
        magnitude_problem(),
        clamp_problem(),
        symbol_problem(),
        count_problem(),
        geometry_problem(),
    ] {
        for held_out in 0..problem.demonstrations.len() {
            let held_out_example = problem.demonstrations[held_out].clone();
            let mut reduced = problem.clone();
            reduced.demonstrations.remove(held_out);
            let outcome = DefinitionInventor
                .invent(&reduced)
                .expect("leave-one-out invention");
            assert_prediction(
                &outcome,
                &held_out_example.inputs,
                held_out_example.output,
                problem.limits.execution,
            );
        }
    }
}

#[test]
fn frozen_negative_controls_produce_no_false_definition() {
    let mut false_positives = 0_u64;
    for trial in 0_u16..100 {
        let problem = InventionProblem {
            definition_name: "negative_control".to_owned(),
            parameters: vec![
                Binder {
                    name: VarName("reading".to_owned()),
                    ty: int_type(-10, 10),
                },
                Binder {
                    name: VarName("first".to_owned()),
                    ty: Type::Atom,
                },
                Binder {
                    name: VarName("second".to_owned()),
                    ty: Type::Atom,
                },
            ],
            return_type: Type::Atom,
            literals: Vec::new(),
            demonstrations: [-6_i64, -5, -2, 1, 4, 7]
                .into_iter()
                .enumerate()
                .map(|(index, reading)| {
                    let first = 2 * trial + 1;
                    let second = 2 * trial + 2;
                    example(
                        &[
                            ("reading", Value::Int(reading)),
                            ("first", Value::Atom(first)),
                            ("second", Value::Atom(second)),
                        ],
                        Value::Atom(if index % 2 == 0 { first } else { second }),
                    )
                })
                .collect(),
            limits: limits(5),
        };
        if DefinitionInventor.invent(&problem).is_ok() {
            false_positives += 1;
        }
    }
    assert_eq!(false_positives, 0);
    println!("GLL1 negative_controls=100 false_positives={false_positives}");
}

#[allow(clippy::needless_pass_by_value)]
fn assert_prediction(
    outcome: &InventionOutcome,
    inputs: &RuntimeEnvironment,
    expected: Value,
    limits: ExecLimits,
) {
    let prediction = DefinitionInventor
        .predict(&outcome.proposal, inputs, limits)
        .expect("held-out execution");
    assert_eq!(prediction, expected);
}

fn atom_set(count: u16, maximum: u32) -> Value {
    Value::Set {
        element_type: Type::Atom,
        maximum,
        values: (0..count).map(Value::Atom).collect::<BTreeSet<_>>(),
    }
}
