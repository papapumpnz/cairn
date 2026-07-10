//! Independently owned hidden-law generators.
//!
//! Family labels and expected outputs remain benchmark metadata. Learners only
//! receive the contained generic invention problems and never branch on IDs.

use cairn_invent::{BindingExample, InventionLimits, InventionProblem};
use cairn_rkl::{Binder, ExecLimits, IntRange, RuntimeEnvironment, Type, Value, VarName};
use cairn_skill::{Condition, Fluent, SuccessfulTrace};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum SupportAxis {
    ParameterNames,
    NumericRanges,
    CategoricalValues,
    Cardinalities,
    Positions,
    Layouts,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HeldOutTask {
    pub full_problem: InventionProblem,
    pub initial_problem: InventionProblem,
    pub one_shot_problem: InventionProblem,
    pub probe: BindingExample,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HiddenLawFamily {
    pub benchmark_family: String,
    pub domain: String,
    pub cardinality_minimum: u32,
    pub cardinality_maximum: u32,
    pub varied_axes: BTreeSet<SupportAxis>,
    pub source_problems: Vec<InventionProblem>,
    pub held_out_tasks: Vec<HeldOutTask>,
}

/// Frozen unrelated mechanism families for the ADR-0007 G0 experiment.
#[must_use]
pub fn g0_hidden_law_families() -> Vec<HiddenLawFamily> {
    vec![
        magnitude_family(),
        clamp_family(),
        symbol_family(),
        count_family(),
        translation_family(),
    ]
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SkillCompositionSuite {
    pub independent_skill_traces: Vec<Vec<SuccessfulTrace>>,
    pub scenarios: Vec<(BTreeSet<Fluent>, Condition)>,
}

/// Frozen trace families and never-demonstrated compositions for G3.
#[must_use]
#[allow(clippy::too_many_lines)] // Keep the complete frozen curriculum visible in one generator.
pub fn g3_unseen_skill_compositions() -> SkillCompositionSuite {
    const KEY_VISIBLE: Fluent = Fluent(1);
    const HAS_KEY: Fluent = Fluent(2);
    const AT_START: Fluent = Fluent(3);
    const AT_BARRIER: Fluent = Fluent(4);
    const BARRIER_OPEN: Fluent = Fluent(5);
    const AT_TARGET: Fluent = Fluent(6);
    let steps = ["acquire", "navigate", "open", "reach"]
        .map(|name| cairn_core::Hash32::digest(name.as_bytes()));
    let trace = |episode: &str,
                 required: &[Fluent],
                 added: Fluent,
                 removed: Option<Fluent>,
                 noise: Fluent,
                 step: cairn_core::Hash32| {
        let mut before = required.iter().copied().collect::<BTreeSet<_>>();
        before.insert(noise);
        let mut after = before.clone();
        if let Some(removed) = removed {
            after.remove(&removed);
        }
        after.insert(added);
        SuccessfulTrace {
            episode: cairn_core::Hash32::digest(episode.as_bytes()),
            before,
            primitive_steps: vec![step],
            after,
        }
    };
    let independent_skill_traces = vec![
        vec![
            trace(
                "acquire-a",
                &[KEY_VISIBLE],
                HAS_KEY,
                None,
                Fluent(101),
                steps[0],
            ),
            trace(
                "acquire-b",
                &[KEY_VISIBLE],
                HAS_KEY,
                None,
                Fluent(102),
                steps[0],
            ),
        ],
        vec![
            trace(
                "navigate-a",
                &[AT_START],
                AT_BARRIER,
                Some(AT_START),
                Fluent(103),
                steps[1],
            ),
            trace(
                "navigate-b",
                &[AT_START],
                AT_BARRIER,
                Some(AT_START),
                Fluent(104),
                steps[1],
            ),
        ],
        vec![
            trace(
                "open-a",
                &[HAS_KEY, AT_BARRIER],
                BARRIER_OPEN,
                None,
                Fluent(105),
                steps[2],
            ),
            trace(
                "open-b",
                &[HAS_KEY, AT_BARRIER],
                BARRIER_OPEN,
                None,
                Fluent(106),
                steps[2],
            ),
        ],
        vec![
            trace(
                "reach-a",
                &[BARRIER_OPEN],
                AT_TARGET,
                None,
                Fluent(107),
                steps[3],
            ),
            trace(
                "reach-b",
                &[BARRIER_OPEN],
                AT_TARGET,
                None,
                Fluent(108),
                steps[3],
            ),
        ],
    ];
    let scenarios = (0_u16..100)
        .map(|index| {
            (
                BTreeSet::from([KEY_VISIBLE, AT_START, Fluent(200 + index)]),
                Condition {
                    required: BTreeSet::from([AT_TARGET]),
                    forbidden: BTreeSet::new(),
                },
            )
        })
        .collect();
    SkillCompositionSuite {
        independent_skill_traces,
        scenarios,
    }
}

fn limits(maximum_cost: u16, maximum_candidates: u64) -> InventionLimits {
    InventionLimits {
        maximum_cost,
        maximum_candidates,
        maximum_expression_nodes: 1_000,
        execution: ExecLimits::default(),
    }
}

fn int_type(lower: i64, upper: i64) -> Type {
    Type::Int(IntRange { lower, upper })
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

fn task(full_problem: InventionProblem, probe: BindingExample) -> HeldOutTask {
    let mut initial_problem = full_problem.clone();
    initial_problem.limits = limits(1, 1);
    let mut one_shot_problem = full_problem.clone();
    one_shot_problem.demonstrations.truncate(1);
    HeldOutTask {
        full_problem,
        initial_problem,
        one_shot_problem,
        probe,
    }
}

fn magnitude_problem(name: &str, bound: i64, readings: &[i64]) -> InventionProblem {
    InventionProblem {
        definition_name: "episode_local_definition".to_owned(),
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
        limits: limits(8, 1_000_000),
    }
}

fn magnitude_family() -> HiddenLawFamily {
    HiddenLawFamily {
        benchmark_family: "family-a".to_owned(),
        domain: "bounded-scalar".to_owned(),
        cardinality_minimum: 1,
        cardinality_maximum: 1,
        varied_axes: BTreeSet::from([SupportAxis::ParameterNames, SupportAxis::NumericRanges]),
        source_problems: vec![
            magnitude_problem("reading", 10, &[-9, -4, -1, 0, 2, 7]),
            magnitude_problem("signal", 16, &[-15, -6, -2, 1, 5, 13]),
        ],
        held_out_tasks: vec![
            task(
                magnitude_problem("measurement", 22, &[-21, -7, -2, 0, 3, 11]),
                example(&[("measurement", Value::Int(19))], Value::Int(19)),
            ),
            task(
                magnitude_problem("amplitude", 30, &[-29, -8, -1, 0, 7, 17]),
                example(&[("amplitude", Value::Int(28))], Value::Int(28)),
            ),
        ],
    }
}

fn clamp_problem(names: [&str; 3], bound: i64, cases: &[(i64, i64, i64)]) -> InventionProblem {
    InventionProblem {
        definition_name: "episode_local_definition".to_owned(),
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
        limits: limits(8, 1_000_000),
    }
}

fn clamp_family() -> HiddenLawFamily {
    let cases_a = [(-9, -3, 4), (9, -3, 4), (1, -3, 4), (-8, -5, 2), (7, -5, 2)];
    let cases_b = [
        (-15, -7, 5),
        (14, -7, 5),
        (2, -7, 5),
        (-8, -4, 9),
        (12, -4, 9),
    ];
    HiddenLawFamily {
        benchmark_family: "family-b".to_owned(),
        domain: "bounded-scalar".to_owned(),
        cardinality_minimum: 3,
        cardinality_maximum: 3,
        varied_axes: BTreeSet::from([SupportAxis::ParameterNames, SupportAxis::NumericRanges]),
        source_problems: vec![
            clamp_problem(["value", "lower", "upper"], 10, &cases_a),
            clamp_problem(["sample", "floor", "ceiling"], 16, &cases_b),
        ],
        held_out_tasks: vec![
            task(
                clamp_problem(["reading", "minimum", "maximum"], 24, &cases_b),
                example(
                    &[
                        ("reading", Value::Int(22)),
                        ("minimum", Value::Int(-9)),
                        ("maximum", Value::Int(12)),
                    ],
                    Value::Int(12),
                ),
            ),
            task(
                clamp_problem(["level", "low", "high"], 30, &cases_a),
                example(
                    &[
                        ("level", Value::Int(-27)),
                        ("low", Value::Int(-11)),
                        ("high", Value::Int(17)),
                    ],
                    Value::Int(-11),
                ),
            ),
        ],
    }
}

fn symbol_problem(names: [&str; 4], cases: &[(i64, i64, u16, u16)]) -> InventionProblem {
    InventionProblem {
        definition_name: "episode_local_definition".to_owned(),
        parameters: vec![
            Binder {
                name: VarName(names[0].to_owned()),
                ty: int_type(-30, 30),
            },
            Binder {
                name: VarName(names[1].to_owned()),
                ty: int_type(-30, 30),
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
        limits: limits(8, 1_000_000),
    }
}

fn symbol_family() -> HiddenLawFamily {
    let first = [
        (-4, 0, 11, 23),
        (3, 0, 11, 23),
        (2, 5, 31, 47),
        (8, 5, 31, 47),
    ];
    let second = [
        (-8, -2, 101, 103),
        (4, -2, 101, 103),
        (1, 7, 107, 109),
        (10, 7, 107, 109),
    ];
    HiddenLawFamily {
        benchmark_family: "family-c".to_owned(),
        domain: "symbolic-control".to_owned(),
        cardinality_minimum: 4,
        cardinality_maximum: 4,
        varied_axes: BTreeSet::from([SupportAxis::ParameterNames, SupportAxis::CategoricalValues]),
        source_problems: vec![
            symbol_problem(["reading", "threshold", "below", "other"], &first),
            symbol_problem(["signal", "boundary", "low", "high"], &second),
        ],
        held_out_tasks: vec![
            task(
                symbol_problem(["observed", "cutoff", "cold", "warm"], &second),
                example(
                    &[
                        ("observed", Value::Int(-9)),
                        ("cutoff", Value::Int(2)),
                        ("cold", Value::Atom(401)),
                        ("warm", Value::Atom(409)),
                    ],
                    Value::Atom(401),
                ),
            ),
            task(
                symbol_problem(["sample", "pivot", "left", "right"], &first),
                example(
                    &[
                        ("sample", Value::Int(15)),
                        ("pivot", Value::Int(3)),
                        ("left", Value::Atom(503)),
                        ("right", Value::Atom(509)),
                    ],
                    Value::Atom(509),
                ),
            ),
        ],
    }
}

fn atom_set(count: u16, maximum: u32) -> Value {
    Value::Set {
        element_type: Type::Atom,
        maximum,
        values: (0..count).map(Value::Atom).collect(),
    }
}

fn count_problem(names: [&str; 2], maximum: u16, cases: &[(u16, i64)]) -> InventionProblem {
    InventionProblem {
        definition_name: "episode_local_definition".to_owned(),
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
            .map(|(count, offset)| {
                example(
                    &[
                        (names[0], atom_set(*count, u32::from(maximum))),
                        (names[1], Value::Int(*offset)),
                    ],
                    Value::Int(i64::from(*count) + offset),
                )
            })
            .collect(),
        limits: limits(8, 1_000_000),
    }
}

fn count_family() -> HiddenLawFamily {
    let first = [(0, -2), (1, 3), (3, -1), (5, 2), (8, -3)];
    let second = [(0, 4), (2, -2), (6, 1), (11, -3), (17, 2)];
    HiddenLawFamily {
        benchmark_family: "family-d".to_owned(),
        domain: "bounded-collection".to_owned(),
        cardinality_minimum: 0,
        cardinality_maximum: 28,
        varied_axes: BTreeSet::from([SupportAxis::ParameterNames, SupportAxis::Cardinalities]),
        source_problems: vec![
            count_problem(["items", "offset"], 10, &first),
            count_problem(["members", "adjustment"], 18, &second),
        ],
        held_out_tasks: vec![
            task(
                count_problem(["tokens", "delta"], 24, &second),
                example(
                    &[("tokens", atom_set(23, 24)), ("delta", Value::Int(-4))],
                    Value::Int(19),
                ),
            ),
            task(
                count_problem(["parts", "change"], 30, &first),
                example(
                    &[("parts", atom_set(28, 30)), ("change", Value::Int(5))],
                    Value::Int(33),
                ),
            ),
        ],
    }
}

fn translation_problem(
    names: [&str; 4],
    bound: i64,
    cases: &[(i64, i64, i64, i64)],
) -> InventionProblem {
    InventionProblem {
        definition_name: "episode_local_definition".to_owned(),
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
        limits: limits(8, 1_000_000),
    }
}

fn translation_family() -> HiddenLawFamily {
    let first = [
        (-10, 7, 3, -2),
        (4, -9, -3, 5),
        (0, 0, 2, 4),
        (12, 8, -4, -3),
    ];
    let second = [
        (-24, 17, 6, -3),
        (14, -19, -7, 8),
        (2, 1, 4, 5),
        (22, 18, -8, -6),
    ];
    HiddenLawFamily {
        benchmark_family: "family-e".to_owned(),
        domain: "coordinate-relation".to_owned(),
        cardinality_minimum: 4,
        cardinality_maximum: 4,
        varied_axes: BTreeSet::from([
            SupportAxis::ParameterNames,
            SupportAxis::Positions,
            SupportAxis::Layouts,
        ]),
        source_problems: vec![
            translation_problem(["row", "column", "dy", "dx"], 20, &first),
            translation_problem(["y", "x", "vertical", "horizontal"], 30, &second),
        ],
        held_out_tasks: vec![
            task(
                translation_problem(["origin_y", "origin_x", "shift_y", "shift_x"], 40, &second),
                example(
                    &[
                        ("origin_y", Value::Int(35)),
                        ("origin_x", Value::Int(-31)),
                        ("shift_y", Value::Int(-9)),
                        ("shift_x", Value::Int(7)),
                    ],
                    Value::Tuple(vec![Value::Int(26), Value::Int(-24)]),
                ),
            ),
            task(
                translation_problem(["top", "left", "down", "across"], 50, &first),
                example(
                    &[
                        ("top", Value::Int(-44)),
                        ("left", Value::Int(42)),
                        ("down", Value::Int(8)),
                        ("across", Value::Int(-6)),
                    ],
                    Value::Tuple(vec![Value::Int(-36), Value::Int(36)]),
                ),
            ),
        ],
    }
}
