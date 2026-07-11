use cairn_distill::{
    AgentId, DistillationVerifier, SharedKnowledgeRegistry, TeacherPacketCompiler,
};
use cairn_invent::InventionLimits;
use cairn_rkl::{
    CheckedTheory, ExecLimits, Quantifier, ReferenceTheoryExecutor, RuntimeEnvironment,
    TheoryExecLimits, TheoryExecutionRequest, Type, TypeEnvironment, Value, VarName,
};
use std::collections::{BTreeMap, BTreeSet};

#[test]
fn offline_collection_lesson_invents_variable_arity_mechanism_and_transfers() {
    let packet = TeacherPacketCompiler
        .parse(include_bytes!("fixtures/collection.json"))
        .expect("closed packet");
    let verified = DistillationVerifier
        .verify(
            TeacherPacketCompiler
                .compile(&packet, InventionLimits::default())
                .expect("compile collection lesson"),
        )
        .expect("verify collection lesson");
    assert!(verified.mechanisms().is_empty());
    assert_eq!(verified.collection_mechanisms().len(), 1);
    assert_eq!(verified.verified_mechanism_examples(), 11);
    let learned = &verified.collection_mechanisms()[0];
    assert_eq!(learned.observed_cardinalities, BTreeSet::from([2, 4, 5]));
    assert_eq!(
        learned.theory.theory().rules[0].binders[0].quantifier,
        Quantifier::ForEach
    );
    let held_out = [-11, -6, -1, 0, 3, 8, 15];
    assert_eq!(
        execute(&learned.theory, &held_out),
        held_out
            .into_iter()
            .map(|value| Value::Int(value * 2 + 1))
            .collect()
    );

    let mut registry = SharedKnowledgeRegistry::new();
    let id = registry
        .publish(verified, AgentId("collection-compiler".to_owned()))
        .expect("publish provisional collection knowledge");
    for peer in ["collection-replay", "collection-falsifier"] {
        let receipt = DistillationVerifier
            .verify_peer(
                TeacherPacketCompiler
                    .compile(&packet, InventionLimits::default())
                    .expect("peer compilation"),
                AgentId(peer.to_owned()),
            )
            .expect("peer verification");
        registry.peer_verify(id, receipt).expect("peer receipt");
    }
    registry
        .record_transfer(id, 7)
        .expect("held-out cardinality");
    assert!(registry.accepted(id).is_some());
}

#[test]
fn collection_packet_rejects_fixed_cardinality_and_duplicate_set_values() {
    let mut packet = TeacherPacketCompiler
        .parse(include_bytes!("fixtures/collection.json"))
        .expect("packet");
    packet.collection_mechanism_lessons[0].episodes[1].inputs = vec![-4, -4, 0, 5];
    assert!(TeacherPacketCompiler
        .compile(&packet, InventionLimits::default())
        .is_err());

    let mut fixed = TeacherPacketCompiler
        .parse(include_bytes!("fixtures/collection.json"))
        .expect("packet");
    fixed.collection_mechanism_lessons[0].episodes.truncate(2);
    fixed.collection_mechanism_lessons[0].episodes[1]
        .inputs
        .truncate(2);
    fixed.collection_mechanism_lessons[0].episodes[1]
        .outputs
        .truncate(2);
    assert!(TeacherPacketCompiler
        .compile(&fixed, InventionLimits::default())
        .is_err());
}

fn execute(theory: &CheckedTheory, inputs: &[i64]) -> BTreeSet<Value> {
    let integer = Type::Int(cairn_rkl::IntRange {
        lower: -i64::from(u16::MAX),
        upper: i64::from(u16::MAX),
    });
    let mut types = TypeEnvironment::new();
    types.insert(
        VarName("collection".to_owned()),
        Type::List {
            element: Box::new(integer.clone()),
            maximum: 32,
        },
    );
    let mut values = RuntimeEnvironment::new();
    values.insert(
        VarName("collection".to_owned()),
        Value::Set {
            element_type: integer,
            maximum: 32,
            values: inputs.iter().copied().map(Value::Int).collect(),
        },
    );
    ReferenceTheoryExecutor
        .execute(
            theory,
            &TheoryExecutionRequest {
                types,
                values,
                initial_facts: BTreeSet::new(),
                initial_state: BTreeMap::new(),
            },
            TheoryExecLimits {
                expression: ExecLimits::default(),
                maximum_bindings: 32,
                maximum_rule_firings: 32,
            },
        )
        .expect("held-out collection replay")
        .rendered
        .facts
}
