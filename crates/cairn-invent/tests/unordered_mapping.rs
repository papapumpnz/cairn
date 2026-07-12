use cairn_invent::{
    DefinitionInventor, InventionError, InventionLimits, UnorderedMappingEpisode,
    UnorderedMappingProblem,
};
use cairn_rkl::{Binder, ExecLimits, IntRange, RuntimeEnvironment, Type, Value, VarName};
use std::collections::BTreeSet;

#[test]
fn unordered_observations_invent_the_same_expression_under_reordering() {
    let first = problem(false, InventionLimits::default());
    let second = problem(true, InventionLimits::default());
    let left = DefinitionInventor
        .invent_unordered_mapping(&first)
        .expect("first")
        .proposal;
    let right = DefinitionInventor
        .invent_unordered_mapping(&second)
        .expect("reordered")
        .proposal;
    assert_eq!(left.semantic_hash, right.semantic_hash);
    assert_eq!(left.definition.body, right.definition.body);
}

#[test]
fn unordered_invention_caps_and_missing_outputs_fail_closed() {
    let mut capped = problem(false, InventionLimits::default());
    capped.limits.maximum_candidates = 1;
    assert!(matches!(
        DefinitionInventor.invent_unordered_mapping(&capped),
        Err(InventionError::CandidateCapacity { maximum: 1 })
    ));
    let mut empty = problem(false, InventionLimits::default());
    empty.episodes[0].expected_outputs.clear();
    assert_eq!(
        DefinitionInventor.invent_unordered_mapping(&empty),
        Err(InventionError::EmptyDemonstrations)
    );
}

fn problem(reverse: bool, limits: InventionLimits) -> UnorderedMappingProblem {
    let mut episodes = vec![
        episode(&[2, 3, 5], &[7, 8, 10]),
        episode(&[11, 14], &[16, 19]),
    ];
    if reverse {
        episodes.reverse();
        for episode in &mut episodes {
            episode.inputs.reverse();
        }
    }
    UnorderedMappingProblem {
        definition_name: "invented_unordered_shift".to_owned(),
        parameters: vec![Binder {
            name: VarName("source".to_owned()),
            ty: Type::Int(IntRange {
                lower: -100,
                upper: 100,
            }),
        }],
        return_type: Type::Int(IntRange {
            lower: -100,
            upper: 100,
        }),
        literals: vec![Value::Int(5)],
        episodes,
        limits: InventionLimits {
            maximum_cost: 4,
            maximum_candidates: limits.maximum_candidates,
            maximum_expression_nodes: 100,
            execution: ExecLimits::default(),
        },
    }
}

fn episode(inputs: &[i64], expected: &[i64]) -> UnorderedMappingEpisode {
    UnorderedMappingEpisode {
        inputs: inputs
            .iter()
            .map(|value| {
                let mut environment = RuntimeEnvironment::new();
                environment.insert(VarName("source".to_owned()), Value::Int(*value));
                environment
            })
            .collect(),
        expected_outputs: expected
            .iter()
            .copied()
            .map(Value::Int)
            .collect::<BTreeSet<_>>(),
    }
}
