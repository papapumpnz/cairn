use cairn_invent::{
    BindingExample, DefinitionInventor, InventionLimits, InventionOutcome, InventionProblem,
};
use cairn_memory::{
    MechanismStore, MemoryAssistedInventor, ModuleStatus, PromotionError, PromotionRunner,
    SolveOrigin, SupportGeometry, VariationDimension,
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

fn magnitude(name: &str, bound: i64, readings: &[i64]) -> InventionProblem {
    InventionProblem {
        definition_name: "learned_magnitude".to_owned(),
        parameters: vec![Binder {
            name: VarName(name.to_owned()),
            ty: int_type(-bound, bound),
        }],
        return_type: int_type(0, bound),
        literals: vec![Value::Int(0)],
        demonstrations: readings
            .iter()
            .map(|reading| example(&[(name, Value::Int(*reading))], Value::Int(reading.abs())))
            .collect(),
        limits: limits(8),
    }
}

fn clamp(names: [&str; 3], bound: i64, cases: &[(i64, i64, i64)]) -> InventionProblem {
    InventionProblem {
        definition_name: "learned_clamp".to_owned(),
        parameters: vec![
            Binder {
                name: VarName(names[0].to_owned()),
                ty: int_type(-bound, bound),
            },
            Binder {
                name: VarName(names[1].to_owned()),
                ty: int_type(-bound, 0),
            },
            Binder {
                name: VarName(names[2].to_owned()),
                ty: int_type(0, bound),
            },
        ],
        return_type: int_type(-bound, bound),
        literals: Vec::new(),
        demonstrations: cases
            .iter()
            .map(|(value, lower, upper)| {
                example(
                    &[
                        (names[0], Value::Int(*value)),
                        (names[1], Value::Int(*lower)),
                        (names[2], Value::Int(*upper)),
                    ],
                    Value::Int((*value).max(*lower).min(*upper)),
                )
            })
            .collect(),
        limits: limits(8),
    }
}

fn symbol(names: [&str; 4], cases: &[(i64, i64, u16, u16)]) -> InventionProblem {
    InventionProblem {
        definition_name: "learned_symbol_choice".to_owned(),
        parameters: vec![
            Binder {
                name: VarName(names[0].to_owned()),
                ty: int_type(-20, 20),
            },
            Binder {
                name: VarName(names[1].to_owned()),
                ty: int_type(-20, 20),
            },
            Binder {
                name: VarName(names[2].to_owned()),
                ty: Type::Atom,
            },
            Binder {
                name: VarName(names[3].to_owned()),
                ty: Type::Atom,
            },
        ],
        return_type: Type::Atom,
        literals: Vec::new(),
        demonstrations: cases
            .iter()
            .map(|(reading, threshold, below, other)| {
                example(
                    &[
                        (names[0], Value::Int(*reading)),
                        (names[1], Value::Int(*threshold)),
                        (names[2], Value::Atom(*below)),
                        (names[3], Value::Atom(*other)),
                    ],
                    Value::Atom(if reading < threshold { *below } else { *other }),
                )
            })
            .collect(),
        limits: limits(8),
    }
}

fn count(names: [&str; 2], maximum: u16, cases: &[(u16, i64)]) -> InventionProblem {
    InventionProblem {
        definition_name: "learned_count_adjustment".to_owned(),
        parameters: vec![
            Binder {
                name: VarName(names[0].to_owned()),
                ty: Type::List {
                    element: Box::new(Type::Atom),
                    maximum: u32::from(maximum),
                },
            },
            Binder {
                name: VarName(names[1].to_owned()),
                ty: int_type(-10, 10),
            },
        ],
        return_type: int_type(-10, i64::from(maximum) + 10),
        literals: Vec::new(),
        demonstrations: cases
            .iter()
            .map(|(item_count, offset)| {
                example(
                    &[
                        (names[0], atom_set(*item_count, u32::from(maximum))),
                        (names[1], Value::Int(*offset)),
                    ],
                    Value::Int(i64::from(*item_count) + offset),
                )
            })
            .collect(),
        limits: limits(8),
    }
}

fn geometry(names: [&str; 4], bound: i64, cases: &[(i64, i64, i64, i64)]) -> InventionProblem {
    InventionProblem {
        definition_name: "learned_translation".to_owned(),
        parameters: vec![
            Binder {
                name: VarName(names[0].to_owned()),
                ty: int_type(-bound, bound),
            },
            Binder {
                name: VarName(names[1].to_owned()),
                ty: int_type(-bound, bound),
            },
            Binder {
                name: VarName(names[2].to_owned()),
                ty: int_type(-10, 10),
            },
            Binder {
                name: VarName(names[3].to_owned()),
                ty: int_type(-10, 10),
            },
        ],
        return_type: Type::Tuple(vec![
            int_type(-bound - 10, bound + 10),
            int_type(-bound - 10, bound + 10),
        ]),
        literals: Vec::new(),
        demonstrations: cases
            .iter()
            .map(|(row, column, row_shift, column_shift)| {
                example(
                    &[
                        (names[0], Value::Int(*row)),
                        (names[1], Value::Int(*column)),
                        (names[2], Value::Int(*row_shift)),
                        (names[3], Value::Int(*column_shift)),
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

fn source_pairs() -> Vec<(
    InventionProblem,
    InventionProblem,
    BTreeSet<VariationDimension>,
)> {
    vec![
        (
            magnitude("reading", 10, &[-9, -4, -1, 0, 2, 7]),
            magnitude("signal", 16, &[-15, -6, -2, 1, 5, 13]),
            dimensions(&[
                VariationDimension::ParameterNames,
                VariationDimension::NumericRanges,
            ]),
        ),
        (
            clamp(
                ["value", "lower", "upper"],
                10,
                &[(-9, -3, 4), (9, -3, 4), (1, -3, 4), (-8, -5, 2), (7, -5, 2)],
            ),
            clamp(
                ["sample", "floor", "ceiling"],
                16,
                &[
                    (-15, -7, 5),
                    (14, -7, 5),
                    (2, -7, 5),
                    (-8, -4, 9),
                    (12, -4, 9),
                ],
            ),
            dimensions(&[
                VariationDimension::ParameterNames,
                VariationDimension::NumericRanges,
            ]),
        ),
        (
            symbol(
                ["reading", "threshold", "below", "other"],
                &[
                    (-4, 0, 11, 23),
                    (3, 0, 11, 23),
                    (2, 5, 31, 47),
                    (8, 5, 31, 47),
                ],
            ),
            symbol(
                ["signal", "boundary", "low_token", "high_token"],
                &[
                    (-8, -2, 101, 103),
                    (4, -2, 101, 103),
                    (1, 7, 107, 109),
                    (10, 7, 107, 109),
                ],
            ),
            dimensions(&[
                VariationDimension::ParameterNames,
                VariationDimension::CategoricalValues,
            ]),
        ),
        (
            count(
                ["items", "offset"],
                10,
                &[(0, -2), (1, 3), (3, -1), (5, 2), (8, -3)],
            ),
            count(
                ["members", "adjustment"],
                18,
                &[(0, 4), (2, -2), (6, 1), (11, -3), (17, 2)],
            ),
            dimensions(&[
                VariationDimension::ParameterNames,
                VariationDimension::Counts,
            ]),
        ),
        (
            geometry(
                ["row", "column", "row_shift", "column_shift"],
                20,
                &[
                    (-10, 7, 3, -2),
                    (4, -9, -3, 5),
                    (0, 0, 2, 4),
                    (12, 8, -4, -3),
                ],
            ),
            geometry(
                ["y", "x", "dy", "dx"],
                30,
                &[
                    (-24, 17, 6, -3),
                    (14, -19, -7, 8),
                    (2, 1, 4, 5),
                    (22, 18, -8, -6),
                ],
            ),
            dimensions(&[
                VariationDimension::ParameterNames,
                VariationDimension::Positions,
                VariationDimension::Layouts,
            ]),
        ),
    ]
}

fn promoted_store() -> MechanismStore {
    let mut store = MechanismStore::new();
    for (first, second, varied_dimensions) in source_pairs() {
        let first = DefinitionInventor
            .invent(&first)
            .expect("first source task");
        let second = DefinitionInventor
            .invent(&second)
            .expect("second source task");
        let module = PromotionRunner
            .promote(
                &[first.proposal, second.proposal],
                SupportGeometry {
                    independent_tasks: 2,
                    held_out_rebindings: 1,
                    varied_dimensions,
                },
            )
            .expect("proof-gated promotion");
        store.insert(module);
    }
    assert_eq!(store.len(), 5);
    store
}

#[allow(clippy::too_many_lines)]
fn related_tasks() -> Vec<(InventionProblem, Vec<BindingExample>)> {
    vec![
        (
            magnitude("measurement", 20, &[-18, -7, -1, 2, 9, 17]),
            vec![
                example(&[("measurement", Value::Int(-20))], Value::Int(20)),
                example(&[("measurement", Value::Int(14))], Value::Int(14)),
            ],
        ),
        (
            clamp(
                ["reading_now", "minimum_ok", "maximum_ok"],
                20,
                &[
                    (-18, -8, 6),
                    (17, -8, 6),
                    (3, -8, 6),
                    (-12, -4, 14),
                    (19, -4, 14),
                ],
            ),
            vec![
                example(
                    &[
                        ("reading_now", Value::Int(-20)),
                        ("minimum_ok", Value::Int(-9)),
                        ("maximum_ok", Value::Int(12)),
                    ],
                    Value::Int(-9),
                ),
                example(
                    &[
                        ("reading_now", Value::Int(8)),
                        ("minimum_ok", Value::Int(-9)),
                        ("maximum_ok", Value::Int(12)),
                    ],
                    Value::Int(8),
                ),
            ],
        ),
        (
            symbol(
                ["observed", "cutoff", "cold_code", "warm_code"],
                &[
                    (-12, -3, 211, 223),
                    (5, -3, 211, 223),
                    (3, 11, 227, 229),
                    (16, 11, 227, 229),
                ],
            ),
            vec![
                example(
                    &[
                        ("observed", Value::Int(-9)),
                        ("cutoff", Value::Int(2)),
                        ("cold_code", Value::Atom(401)),
                        ("warm_code", Value::Atom(409)),
                    ],
                    Value::Atom(401),
                ),
                example(
                    &[
                        ("observed", Value::Int(9)),
                        ("cutoff", Value::Int(2)),
                        ("cold_code", Value::Atom(401)),
                        ("warm_code", Value::Atom(409)),
                    ],
                    Value::Atom(409),
                ),
            ],
        ),
        (
            count(
                ["entries", "delta"],
                24,
                &[(0, -4), (2, 5), (7, -2), (14, 3), (22, -5)],
            ),
            vec![
                example(
                    &[("entries", atom_set(4, 24)), ("delta", Value::Int(6))],
                    Value::Int(10),
                ),
                example(
                    &[("entries", atom_set(20, 24)), ("delta", Value::Int(-7))],
                    Value::Int(13),
                ),
            ],
        ),
        (
            geometry(
                ["horizontal", "vertical", "horizontal_move", "vertical_move"],
                40,
                &[
                    (-35, 22, 7, -4),
                    (19, -31, -8, 9),
                    (0, 4, 5, 6),
                    (33, 28, -9, -7),
                ],
            ),
            vec![
                example(
                    &[
                        ("horizontal", Value::Int(-40)),
                        ("vertical", Value::Int(40)),
                        ("horizontal_move", Value::Int(10)),
                        ("vertical_move", Value::Int(-10)),
                    ],
                    Value::Tuple(vec![Value::Int(-30), Value::Int(30)]),
                ),
                example(
                    &[
                        ("horizontal", Value::Int(12)),
                        ("vertical", Value::Int(-17)),
                        ("horizontal_move", Value::Int(-6)),
                        ("vertical_move", Value::Int(8)),
                    ],
                    Value::Tuple(vec![Value::Int(6), Value::Int(-9)]),
                ),
            ],
        ),
    ]
}

#[test]
fn frozen_gll2_memory_reduces_search_and_preserves_exactness() {
    let store = promoted_store();
    let mut memory_work = Vec::new();
    let mut fresh_work = Vec::new();
    for (problem, held_out) in related_tasks() {
        let fresh = DefinitionInventor
            .invent(&problem)
            .expect("memory-off solve");
        let remembered = MemoryAssistedInventor
            .invent(&problem, &store)
            .expect("memory-on solve");
        assert_eq!(remembered.origin, SolveOrigin::MemoryProposal);
        for example in &held_out {
            assert_exact(&fresh, example, problem.limits.execution);
            assert_exact(&remembered.invention, example, problem.limits.execution);
        }
        fresh_work.push(fresh.proposal.candidates_evaluated);
        memory_work.push(remembered.metrics.total_search_work);
    }
    let fresh_median = median(&mut fresh_work);
    let memory_median = median(&mut memory_work);
    let reduction_basis_points = (fresh_median - memory_median) * 10_000 / fresh_median;
    let reduction_whole = reduction_basis_points / 100;
    let reduction_fraction = reduction_basis_points % 100;
    assert!(memory_median * 2 <= fresh_median);
    println!(
        "GLL2 fresh_median={fresh_median} memory_median={memory_median} search_reduction_percent={reduction_whole}.{reduction_fraction:02} related_exact=5/5"
    );
}

#[test]
fn frozen_gll2_memory_requires_fewer_demonstrations() {
    let store = promoted_store();
    let mut fresh_requirements = Vec::new();
    let mut memory_requirements = Vec::new();
    for (problem, held_out) in related_tasks() {
        fresh_requirements.push(minimum_demonstrations(&problem, &held_out, None));
        memory_requirements.push(minimum_demonstrations(&problem, &held_out, Some(&store)));
    }
    let fresh_median = median_usize(&mut fresh_requirements);
    let memory_median = median_usize(&mut memory_requirements);
    assert!(memory_median < fresh_median);
    println!(
        "GLL2 fresh_demonstrations_median={fresh_median} memory_demonstrations_median={memory_median}"
    );
}

#[test]
fn unrelated_tasks_fall_back_without_accuracy_regression_and_status_is_enforced() {
    let mut store = promoted_store();
    let unrelated = InventionProblem {
        definition_name: "unrelated_square".to_owned(),
        parameters: vec![Binder {
            name: VarName("quantity".to_owned()),
            ty: int_type(-10, 10),
        }],
        return_type: int_type(0, 100),
        literals: Vec::new(),
        demonstrations: [-9_i64, -3, 0, 2, 7]
            .into_iter()
            .map(|value| {
                example(
                    &[("quantity", Value::Int(value))],
                    Value::Int(value * value),
                )
            })
            .collect(),
        limits: limits(8),
    };
    let fresh = DefinitionInventor
        .invent(&unrelated)
        .expect("fresh unrelated solve");
    let remembered = MemoryAssistedInventor
        .invent(&unrelated, &store)
        .expect("memory fallback solve");
    assert_eq!(remembered.origin, SolveOrigin::FreshSearch);
    let held_out = example(&[("quantity", Value::Int(10))], Value::Int(100));
    assert_exact(&fresh, &held_out, unrelated.limits.execution);
    assert_exact(&remembered.invention, &held_out, unrelated.limits.execution);

    let id = store.reusable_modules().next().expect("module").id();
    assert!(store.set_status(id, ModuleStatus::Quarantined));
    assert_eq!(store.reusable_modules().count(), 4);

    let first_source = DefinitionInventor
        .invent(&source_pairs()[0].0)
        .expect("one source");
    assert!(matches!(
        PromotionRunner.promote(
            &[first_source.proposal],
            SupportGeometry {
                independent_tasks: 1,
                held_out_rebindings: 1,
                varied_dimensions: dimensions(&[VariationDimension::NumericRanges]),
            }
        ),
        Err(PromotionError::InsufficientTasks { .. })
    ));
}

fn minimum_demonstrations(
    problem: &InventionProblem,
    held_out: &[BindingExample],
    store: Option<&MechanismStore>,
) -> usize {
    for count in 1..=problem.demonstrations.len() {
        let mut reduced = problem.clone();
        reduced.demonstrations.truncate(count);
        let outcome = if let Some(store) = store {
            MemoryAssistedInventor
                .invent(&reduced, store)
                .ok()
                .map(|outcome| outcome.invention)
        } else {
            DefinitionInventor.invent(&reduced).ok()
        };
        if outcome.is_some_and(|outcome| {
            held_out
                .iter()
                .all(|example| predicts_exactly(&outcome, example, problem.limits.execution))
        }) {
            return count;
        }
    }
    problem.demonstrations.len() + 1
}

fn predicts_exactly(
    outcome: &InventionOutcome,
    example: &BindingExample,
    limits: ExecLimits,
) -> bool {
    DefinitionInventor
        .predict(&outcome.proposal, &example.inputs, limits)
        .is_ok_and(|prediction| prediction == example.output)
}

fn assert_exact(outcome: &InventionOutcome, example: &BindingExample, limits: ExecLimits) {
    assert!(predicts_exactly(outcome, example, limits));
}

fn dimensions(values: &[VariationDimension]) -> BTreeSet<VariationDimension> {
    values.iter().copied().collect()
}

fn atom_set(count: u16, maximum: u32) -> Value {
    Value::Set {
        element_type: Type::Atom,
        maximum,
        values: (0..count).map(Value::Atom).collect::<BTreeSet<_>>(),
    }
}

fn median(values: &mut [u64]) -> u64 {
    values.sort_unstable();
    values[values.len() / 2]
}

fn median_usize(values: &mut [usize]) -> usize {
    values.sort_unstable();
    values[values.len() / 2]
}
