use cairn_distill::{CollectionDistillationProposer, DistillationVerifier, TeacherPacketCompiler};
use cairn_invent::InventionLimits;
use cairn_mechanism::{CardinalityRange, DomainClass, SupportGeometry, VariableId};
use cairn_memory::{MechanismStore, ModuleStatus};
use cairn_runtime::{CognitionRuntime, ProceduralSolveError};
use cairn_sequence::{CollectionSequenceExample, SequenceAdapter};
use std::collections::BTreeSet;

#[test]
fn offline_collection_knowledge_replays_then_solves_unseen_sequence_set() {
    let packet = TeacherPacketCompiler
        .parse(include_bytes!(
            "../../cairn-distill/tests/fixtures/collection.json"
        ))
        .expect("packet");
    let verified = DistillationVerifier
        .verify(
            TeacherPacketCompiler
                .compile(&packet, InventionLimits::default())
                .expect("compile"),
        )
        .expect("verify");
    let procedures = CollectionDistillationProposer
        .propose(
            &verified,
            &SupportGeometry {
                observed_domains: BTreeSet::from([
                    DomainClass("entity-batch".to_owned()),
                    DomainClass("numeric-sequence-set".to_owned()),
                ]),
                observed_cardinalities: BTreeSet::from([
                    CardinalityRange {
                        minimum: 2,
                        maximum: 5,
                    },
                    CardinalityRange {
                        minimum: 7,
                        maximum: 7,
                    },
                ]),
                varied_variables: BTreeSet::from([VariableId("collection".to_owned())]),
                invariant_variables: BTreeSet::new(),
                unknown_variables: BTreeSet::from([VariableId("ordered-duplicates".to_owned())]),
                validated_compositions: BTreeSet::new(),
                failed_conditions: BTreeSet::new(),
            },
        )
        .expect("provisional procedural proposal");
    assert!(procedures
        .modules()
        .all(|module| module.status() == ModuleStatus::Provisional));
    let runtime = CognitionRuntime::with_memories(MechanismStore::new(), procedures);
    let examples = [
        CollectionSequenceExample {
            input: vec![-8, 1, 6],
            output: vec![-15, 3, 13],
        },
        CollectionSequenceExample {
            input: vec![-10, -5, -2, 2, 7, 12],
            output: vec![-19, -9, -3, 5, 15, 25],
        },
    ];
    let prediction_input = vec![-12, -7, -3, 0, 4, 9, 14, 20];
    let problem = SequenceAdapter.lower_collection(32, &examples, prediction_input.clone());
    assert_eq!(
        CognitionRuntime::new()
            .solve_collection(&problem)
            .expect_err("blank memory cannot supply a procedural proposal"),
        ProceduralSolveError::NoVerifiedProposal
    );
    let solved = runtime
        .solve_collection(&problem)
        .expect("current-task replayed proposal");
    assert_eq!(solved.proposals_attempted, 1);
    assert_eq!(solved.demonstration_replays, 2);
    assert_eq!(solved.predictions.len(), 1);
    assert_eq!(
        solved.predictions[0].output,
        prediction_input
            .into_iter()
            .map(|value| value * 2 + 1)
            .collect()
    );

    let incompatible = [
        CollectionSequenceExample {
            input: vec![-8, 1, 6],
            output: vec![-7, 2, 7],
        },
        CollectionSequenceExample {
            input: vec![-10, -5, 2, 7],
            output: vec![-9, -4, 3, 8],
        },
    ];
    let incompatible = SequenceAdapter.lower_collection(32, &incompatible, vec![-3, 4, 9]);
    assert_eq!(
        runtime
            .solve_collection(&incompatible)
            .expect_err("current demonstrations refute the imported theory"),
        ProceduralSolveError::NoVerifiedProposal
    );
}
