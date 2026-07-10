use cairn_invent::{
    BindingExample, DefinitionInventor, InventionLimits, InventionOutcome, InventionProblem,
};
use cairn_memory::{
    MechanismStore, MemoryAssistedInventor, ModuleStatus, PromotionRunner, SolveOrigin,
    SupportGeometry, VariationDimension,
};
use cairn_rkl::{Binder, ExecLimits, IntRange, RuntimeEnvironment, Type, Value, VarName};
use std::collections::BTreeSet;

fn int_type(lower: i64, upper: i64) -> Type {
    Type::Int(IntRange { lower, upper })
}

fn limits(maximum_cost: u16) -> InventionLimits {
    InventionLimits {
        maximum_cost,
        maximum_candidates: 500_000,
        maximum_expression_nodes: 2_000,
        execution: ExecLimits {
            fuel: 20_000,
            maximum_call_depth: 16,
            maximum_trace_events: 20_000,
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
        definition_name: "magnitude_skill".to_owned(),
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
        limits: limits(7),
    }
}

fn clamp(names: [&str; 3], bound: i64) -> InventionProblem {
    let cases = [
        (-bound + 1, -bound / 2, bound / 3),
        (bound - 1, -bound / 2, bound / 3),
        (1, -bound / 2, bound / 3),
        (-bound / 2, -bound / 3, bound / 2),
        (bound / 2 + 2, -bound / 3, bound / 2),
    ];
    InventionProblem {
        definition_name: "clamp_skill".to_owned(),
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
            .into_iter()
            .map(|(value, lower, upper)| {
                example(
                    &[
                        (names[0], Value::Int(value)),
                        (names[1], Value::Int(lower)),
                        (names[2], Value::Int(upper)),
                    ],
                    Value::Int(value.max(lower).min(upper)),
                )
            })
            .collect(),
        limits: limits(7),
    }
}

fn count_adjustment(names: [&str; 2], maximum: u16) -> InventionProblem {
    InventionProblem {
        definition_name: "count_skill".to_owned(),
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
        demonstrations: [(0_u16, -2_i64), (1, 3), (3, -1), (6, 2), (maximum - 1, -3)]
            .into_iter()
            .map(|(count, offset)| {
                example(
                    &[
                        (names[0], atom_set(count, u32::from(maximum))),
                        (names[1], Value::Int(offset)),
                    ],
                    Value::Int(i64::from(count) + offset),
                )
            })
            .collect(),
        limits: limits(7),
    }
}

fn symbol_choice(names: [&str; 4], atoms: [(u16, u16); 2]) -> InventionProblem {
    let cases = [
        (-8, -2, atoms[0]),
        (4, -2, atoms[0]),
        (1, 7, atoms[1]),
        (10, 7, atoms[1]),
    ];
    InventionProblem {
        definition_name: "symbol_skill".to_owned(),
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
            .into_iter()
            .map(|(reading, threshold, (below, other))| {
                example(
                    &[
                        (names[0], Value::Int(reading)),
                        (names[1], Value::Int(threshold)),
                        (names[2], Value::Atom(below)),
                        (names[3], Value::Atom(other)),
                    ],
                    Value::Atom(if reading < threshold { below } else { other }),
                )
            })
            .collect(),
        limits: limits(7),
    }
}

fn promoted_store() -> MechanismStore {
    let pairs = [
        (
            magnitude("reading", 10, &[-9, -4, -1, 0, 2, 7]),
            magnitude("signal", 18, &[-17, -6, -2, 1, 5, 14]),
            dimensions(&[
                VariationDimension::ParameterNames,
                VariationDimension::NumericRanges,
            ]),
        ),
        (
            clamp(["value", "lower", "upper"], 12),
            clamp(["sample", "floor", "ceiling"], 20),
            dimensions(&[
                VariationDimension::ParameterNames,
                VariationDimension::NumericRanges,
            ]),
        ),
        (
            count_adjustment(["items", "offset"], 10),
            count_adjustment(["members", "adjustment"], 18),
            dimensions(&[
                VariationDimension::ParameterNames,
                VariationDimension::Counts,
            ]),
        ),
        (
            symbol_choice(
                ["reading", "threshold", "below", "other"],
                [(11, 13), (17, 19)],
            ),
            symbol_choice(
                ["signal", "boundary", "low_code", "high_code"],
                [(101, 103), (107, 109)],
            ),
            dimensions(&[
                VariationDimension::ParameterNames,
                VariationDimension::CategoricalValues,
            ]),
        ),
    ];
    let mut store = MechanismStore::new();
    for (first, second, varied_dimensions) in pairs {
        let first = DefinitionInventor
            .invent(&first)
            .expect("learn first skill");
        let second = DefinitionInventor
            .invent(&second)
            .expect("learn second skill");
        let module = PromotionRunner
            .promote(
                &[first.proposal, second.proposal],
                SupportGeometry {
                    independent_tasks: 2,
                    held_out_rebindings: 1,
                    varied_dimensions,
                },
            )
            .expect("promote independently learned skill");
        store.insert(module);
    }
    assert_eq!(store.len(), 4);
    store
}

#[test]
fn frozen_gll3_composes_magnitude_with_clamping() {
    let store = promoted_store();
    let problem = InventionProblem {
        definition_name: "composed_bounded_magnitude".to_owned(),
        parameters: vec![
            Binder {
                name: VarName("measurement".to_owned()),
                ty: int_type(-20, 20),
            },
            Binder {
                name: VarName("minimum".to_owned()),
                ty: int_type(0, 10),
            },
            Binder {
                name: VarName("maximum".to_owned()),
                ty: int_type(10, 20),
            },
        ],
        return_type: int_type(0, 20),
        literals: Vec::new(),
        demonstrations: [
            (-18, 3, 12),
            (-2, 3, 12),
            (7, 3, 12),
            (-15, 5, 10),
            (1, 5, 10),
            (9, 5, 10),
        ]
        .into_iter()
        .map(|(value, lower, upper)| bounded_magnitude_example(value, lower, upper))
        .collect(),
        limits: limits(7),
    };

    let outcome = MemoryAssistedInventor
        .invent(&problem, &store)
        .expect("compose learned skills");
    assert_eq!(outcome.origin, SolveOrigin::MemoryComposition);
    let record = outcome.composition.as_ref().expect("composition proof");
    assert_ne!(record.outer_module, record.inner_module);
    let fresh = DefinitionInventor.invent(&problem).ok();
    let mut fresh_exact = fresh.is_some();
    let mut held_out = 0_u64;
    for value in -20_i64..=20 {
        for lower in [0_i64, 4, 8] {
            for upper in [10_i64, 15, 20] {
                let example = bounded_magnitude_example(value, lower, upper);
                assert_exact(&outcome.invention, &example, problem.limits.execution);
                fresh_exact &= fresh.as_ref().is_some_and(|fresh| {
                    predicts_exactly(fresh, &example, problem.limits.execution)
                });
                held_out += 1;
            }
        }
    }
    assert!(!fresh_exact, "memory-off path unexpectedly composed skills");
    assert_skill_removal_disables_composition(&problem, &store);
    println!(
        "GLL3 composition=magnitude->clamp proposals={} held_out_exact={held_out} body={:?}",
        outcome.metrics.composition_proposals_evaluated, outcome.invention.proposal.definition.body
    );
}

#[test]
fn frozen_gll3_composes_counting_with_clamping() {
    let store = promoted_store();
    let problem = InventionProblem {
        definition_name: "composed_bounded_inventory".to_owned(),
        parameters: vec![
            Binder {
                name: VarName("objects".to_owned()),
                ty: Type::List {
                    element: Box::new(Type::Atom),
                    maximum: 24,
                },
            },
            Binder {
                name: VarName("change".to_owned()),
                ty: int_type(-10, 10),
            },
            Binder {
                name: VarName("floor".to_owned()),
                ty: int_type(-5, 5),
            },
            Binder {
                name: VarName("ceiling".to_owned()),
                ty: int_type(10, 25),
            },
        ],
        return_type: int_type(-5, 25),
        literals: Vec::new(),
        demonstrations: [
            (0, -4, 1, 12),
            (3, 2, 1, 12),
            (15, 3, 1, 12),
            (8, -6, 4, 18),
        ]
        .into_iter()
        .map(|(count, offset, lower, upper)| bounded_count_example(count, offset, lower, upper))
        .collect(),
        limits: limits(7),
    };
    let outcome = MemoryAssistedInventor
        .invent(&problem, &store)
        .expect("compose count and clamp");
    assert_eq!(outcome.origin, SolveOrigin::MemoryComposition);
    let fresh = DefinitionInventor.invent(&problem).ok();
    let mut fresh_exact = fresh.is_some();
    let mut held_out = 0_u64;
    for count in [0_u16, 2, 7, 13, 20, 24] {
        for offset in [-8_i64, 0, 6] {
            let example = bounded_count_example(count, offset, 2, 17);
            assert_exact(&outcome.invention, &example, problem.limits.execution);
            fresh_exact &= fresh
                .as_ref()
                .is_some_and(|fresh| predicts_exactly(fresh, &example, problem.limits.execution));
            held_out += 1;
        }
    }
    assert!(!fresh_exact, "memory-off path unexpectedly composed skills");
    assert_skill_removal_disables_composition(&problem, &store);
    println!(
        "GLL3 composition=count->clamp proposals={} held_out_exact={held_out} body={:?}",
        outcome.metrics.composition_proposals_evaluated, outcome.invention.proposal.definition.body
    );
}

#[test]
fn frozen_gll3_composes_magnitude_with_symbol_selection() {
    let store = promoted_store();
    let problem = InventionProblem {
        definition_name: "composed_magnitude_control".to_owned(),
        parameters: vec![
            Binder {
                name: VarName("signed_signal".to_owned()),
                ty: int_type(-20, 20),
            },
            Binder {
                name: VarName("limit".to_owned()),
                ty: int_type(0, 20),
            },
            Binder {
                name: VarName("inside_code".to_owned()),
                ty: Type::Atom,
            },
            Binder {
                name: VarName("outside_code".to_owned()),
                ty: Type::Atom,
            },
        ],
        return_type: Type::Atom,
        literals: Vec::new(),
        demonstrations: [
            (-12, 5, 211, 223),
            (-3, 5, 211, 223),
            (2, 5, 211, 223),
            (9, 5, 211, 223),
            (-7, 10, 227, 229),
            (14, 10, 227, 229),
        ]
        .into_iter()
        .map(|(value, threshold, inside, outside)| {
            magnitude_symbol_example(value, threshold, inside, outside)
        })
        .collect(),
        limits: limits(8),
    };
    let outcome = MemoryAssistedInventor
        .invent(&problem, &store)
        .expect("compose magnitude and selector");
    assert_eq!(outcome.origin, SolveOrigin::MemoryComposition);
    let fresh = DefinitionInventor.invent(&problem).ok();
    let mut fresh_exact = fresh.is_some();
    let mut held_out = 0_u64;
    for value in -20_i64..=20 {
        for threshold in [4_i64, 9, 15] {
            let example = magnitude_symbol_example(value, threshold, 401, 409);
            assert_exact(&outcome.invention, &example, problem.limits.execution);
            fresh_exact &= fresh
                .as_ref()
                .is_some_and(|fresh| predicts_exactly(fresh, &example, problem.limits.execution));
            held_out += 1;
        }
    }
    assert!(!fresh_exact, "memory-off path unexpectedly composed skills");
    assert_skill_removal_disables_composition(&problem, &store);
    println!(
        "GLL3 composition=magnitude->symbol proposals={} held_out_exact={held_out} body={:?}",
        outcome.metrics.composition_proposals_evaluated, outcome.invention.proposal.definition.body
    );
}

fn assert_skill_removal_disables_composition(problem: &InventionProblem, store: &MechanismStore) {
    let mut ablated = store.clone();
    let used = MemoryAssistedInventor
        .invent(problem, store)
        .expect("composition before ablation")
        .composition
        .expect("used modules");
    assert!(ablated.set_status(used.inner_module, ModuleStatus::Quarantined));
    let after = MemoryAssistedInventor.invent(problem, &ablated);
    assert!(
        after.is_err()
            || after.is_ok_and(|outcome| outcome.origin != SolveOrigin::MemoryComposition),
        "composition survived removal of a required learned skill"
    );
}

fn bounded_magnitude_example(value: i64, lower: i64, upper: i64) -> BindingExample {
    example(
        &[
            ("measurement", Value::Int(value)),
            ("minimum", Value::Int(lower)),
            ("maximum", Value::Int(upper)),
        ],
        Value::Int(value.abs().max(lower).min(upper)),
    )
}

fn bounded_count_example(count: u16, offset: i64, lower: i64, upper: i64) -> BindingExample {
    example(
        &[
            ("objects", atom_set(count, 24)),
            ("change", Value::Int(offset)),
            ("floor", Value::Int(lower)),
            ("ceiling", Value::Int(upper)),
        ],
        Value::Int((i64::from(count) + offset).max(lower).min(upper)),
    )
}

fn magnitude_symbol_example(
    value: i64,
    threshold: i64,
    inside: u16,
    outside: u16,
) -> BindingExample {
    example(
        &[
            ("signed_signal", Value::Int(value)),
            ("limit", Value::Int(threshold)),
            ("inside_code", Value::Atom(inside)),
            ("outside_code", Value::Atom(outside)),
        ],
        Value::Atom(if value.abs() < threshold {
            inside
        } else {
            outside
        }),
    )
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
