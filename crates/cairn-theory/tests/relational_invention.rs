use cairn_rkl::{
    ExecLimits, IntRange, ReferenceExecutor, RuntimeEnvironment, TraceLevel, Type, TypeChecker,
    TypeEnvironment, Value, VarName,
};
use cairn_theory::{GuardLimits, RelationExample, RelationalPredicateInventor};
use std::collections::BTreeMap;

#[test]
fn invents_strict_enclosure_from_generic_coordinate_contrasts() {
    let left = features("outer");
    let right = features("inner");
    let examples = vec![
        example([0, 10, 0, 10], [2, 8, 3, 7], true),
        example([-5, 20, 10, 30], [0, 15, 12, 25], true),
        example([100, 200, -50, 50], [110, 190, -40, 40], true),
        example([0, 10, 0, 10], [-1, 8, 3, 7], false),
        example([0, 10, 0, 10], [2, 11, 3, 7], false),
        example([0, 10, 0, 10], [2, 8, -1, 7], false),
        example([0, 10, 0, 10], [2, 8, 3, 11], false),
        example([0, 10, 0, 10], [0, 8, 3, 7], false),
        example([0, 10, 0, 10], [2, 10, 3, 7], false),
    ];
    let learned = RelationalPredicateInventor
        .invent(
            &left,
            &right,
            &examples,
            GuardLimits {
                maximum_conjuncts: 4,
                maximum_candidates: 200_000,
            },
        )
        .expect("invent enclosure");
    assert_eq!(learned.guard.predicates.len(), 4);
    for (outer, inner, expected) in [
        ([-100, 100, -100, 100], [-90, 90, -80, 80], true),
        ([-100, 100, -100, 100], [-101, 90, -80, 80], false),
        ([-100, 100, -100, 100], [-90, 101, -80, 80], false),
        ([-100, 100, -100, 100], [-90, 90, -101, 80], false),
        ([-100, 100, -100, 100], [-90, 90, -80, 101], false),
    ] {
        assert_eq!(execute(&learned, outer, inner), expected);
    }
    eprintln!(
        "RELATION_METRICS atoms={} conjunction_candidates={} conjuncts={} training={} held_out=5 cost={} body={:?}",
        learned.atomic_predicates_evaluated,
        learned.guard.candidates_evaluated,
        learned.guard.predicates.len(),
        examples.len(),
        learned.guard.structural_cost,
        learned.guard.expression
    );
}

fn execute(learned: &cairn_theory::InventedRelation, outer: [i64; 4], inner: [i64; 4]) -> bool {
    let mut type_environment = TypeEnvironment::new();
    for parameter in &learned.parameters {
        type_environment.insert(parameter.name.clone(), parameter.ty.clone());
    }
    let typed = TypeChecker::new(1_000)
        .check(&learned.guard.expression, &type_environment)
        .expect("checked relation");
    let inputs = bindings(outer, inner).into_iter().fold(
        RuntimeEnvironment::new(),
        |mut environment, (name, value)| {
            environment.insert(name, Value::Int(value));
            environment
        },
    );
    match ReferenceExecutor::without_definitions(ExecLimits::default(), TraceLevel::Off)
        .execute(&typed, &inputs)
        .expect("relation execution")
        .result
    {
        Value::Bool(value) => value,
        other => panic!("unexpected result {other:?}"),
    }
}

fn features(prefix: &str) -> Vec<(VarName, Type)> {
    ["min_row", "max_row", "min_column", "max_column"]
        .into_iter()
        .map(|feature| {
            (
                VarName(format!("{prefix}_{feature}")),
                Type::Int(IntRange {
                    lower: -1_000,
                    upper: 1_000,
                }),
            )
        })
        .collect()
}

fn example(outer: [i64; 4], inner: [i64; 4], related: bool) -> RelationExample {
    RelationExample {
        bindings: bindings(outer, inner),
        related,
    }
}

fn bindings(outer: [i64; 4], inner: [i64; 4]) -> BTreeMap<VarName, i64> {
    features("outer")
        .into_iter()
        .map(|(name, _)| name)
        .zip(outer)
        .chain(
            features("inner")
                .into_iter()
                .map(|(name, _)| name)
                .zip(inner),
        )
        .collect()
}
