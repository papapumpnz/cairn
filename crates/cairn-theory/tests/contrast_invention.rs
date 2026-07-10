use cairn_invent::{DefinitionInventor, InventionLimits};
use cairn_rkl::{ExecLimits, IntRange, RuntimeEnvironment, Type, Value, VarName};
use cairn_theory::{ContrastExample, ContrastPredicateInventor};
use std::collections::BTreeMap;

#[test]
fn invents_an_omitted_selector_from_positive_and_hard_negative_entities() {
    let features = [
        feature("value", 0, 20),
        feature("size", 0, 100),
        feature("enclosed", 0, 1),
        feature("has_marker", 0, 1),
    ];
    let examples = vec![
        example([2, 3, 1, 1], true),
        example([9, 18, 1, 1], true),
        example([2, 3, 0, 1], false),
        example([9, 18, 1, 0], false),
        example([4, 40, 0, 0], false),
    ];
    let learned = ContrastPredicateInventor
        .invent(
            "invented_eligibility",
            &features,
            &examples,
            vec![Value::Int(1)],
            InventionLimits {
                maximum_cost: 8,
                maximum_candidates: 1_000_000,
                maximum_expression_nodes: 2_000,
                execution: ExecLimits::default(),
            },
        )
        .expect("contrast predicate");
    for (values, expected) in [
        ([17, 1, 1, 1], true),
        ([1, 99, 0, 1], false),
        ([17, 1, 1, 0], false),
        ([1, 99, 0, 0], false),
    ] {
        let inputs = environment(values);
        assert_eq!(
            DefinitionInventor
                .predict(&learned.proposal, &inputs, ExecLimits::default(),)
                .expect("held-out predicate"),
            Value::Bool(expected)
        );
    }
    assert!(!format!("{:?}", learned.proposal.definition.body).contains("eligible"));
    eprintln!(
        "CONTRAST_METRICS candidates={} cost={} training=5 held_out=4 body={:?}",
        learned.proposal.candidates_evaluated,
        learned.proposal.structural_cost,
        learned.proposal.definition.body
    );
}

fn feature(name: &str, lower: i64, upper: i64) -> (VarName, Type) {
    (
        VarName(name.to_owned()),
        Type::Int(IntRange { lower, upper }),
    )
}

fn example(values: [i64; 4], selected: bool) -> ContrastExample {
    ContrastExample {
        features: bindings(values),
        selected,
    }
}

fn environment(values: [i64; 4]) -> RuntimeEnvironment {
    let mut environment = RuntimeEnvironment::new();
    for (name, value) in bindings(values) {
        environment.insert(name, value);
    }
    environment
}

fn bindings(values: [i64; 4]) -> BTreeMap<VarName, Value> {
    ["value", "size", "enclosed", "has_marker"]
        .into_iter()
        .zip(values)
        .map(|(name, value)| (VarName(name.to_owned()), Value::Int(value)))
        .collect()
}
