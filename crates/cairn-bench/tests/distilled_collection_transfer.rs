use cairn_distill::{CollectionDistillationProposer, DistillationVerifier, TeacherPacketCompiler};
use cairn_invent::InventionLimits;
use cairn_mechanism::{CardinalityRange, DomainClass, SupportGeometry, VariableId};
use cairn_memory::{MechanismStore, ModuleStatus};
use cairn_runtime::{CognitionRuntime, ProceduralSolveError};
use cairn_sequence::{CollectionSequenceExample, SequenceAdapter};
use cairn_vector::{VectorAdapter, VectorAttributeBatch};
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

#[test]
fn one_distilled_theory_rebinds_through_sequence_and_vector_frontends() {
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
                    DomainClass("sequence".to_owned()),
                    DomainClass("vector-graphics".to_owned()),
                ]),
                observed_cardinalities: BTreeSet::from([CardinalityRange {
                    minimum: 2,
                    maximum: 8,
                }]),
                varied_variables: BTreeSet::from([VariableId("collection".to_owned())]),
                invariant_variables: BTreeSet::new(),
                unknown_variables: BTreeSet::new(),
                validated_compositions: BTreeSet::new(),
                failed_conditions: BTreeSet::new(),
            },
        )
        .expect("proposal");
    let runtime = CognitionRuntime::with_memories(MechanismStore::new(), procedures);
    let sequence = SequenceAdapter.lower_collection(
        32,
        &[
            CollectionSequenceExample {
                input: vec![-2, 4, 7],
                output: vec![-3, 9, 15],
            },
            CollectionSequenceExample {
                input: vec![-6, 0, 3, 10],
                output: vec![-11, 1, 7, 21],
            },
        ],
        vec![-9, 2, 5],
    );
    let vector = VectorAdapter.lower_attribute_collection(
        32,
        &[
            VectorAttributeBatch {
                input_attributes: vec![-8, 1],
                output_attributes: vec![-15, 3],
            },
            VectorAttributeBatch {
                input_attributes: vec![-5, -1, 6, 12, 18],
                output_attributes: vec![-9, -1, 13, 25, 37],
            },
        ],
        vec![-11, -3, 0, 8, 15, 21],
    );
    let sequence_solve = runtime
        .solve_collection(&sequence)
        .expect("sequence replay");
    let vector_solve = runtime.solve_collection(&vector).expect("vector replay");
    assert_eq!(
        sequence_solve.predictions[0].module,
        vector_solve.predictions[0].module
    );
    assert_eq!(
        sequence_solve.predictions[0].output,
        BTreeSet::from([-17, 5, 11])
    );
    assert_eq!(
        vector_solve.predictions[0].output,
        BTreeSet::from([-21, -5, 1, 17, 31, 43])
    );
}

#[test]
fn runtime_composes_independently_distilled_map_and_filter_without_a_composite() {
    let mut packet = TeacherPacketCompiler
        .parse(include_bytes!(
            "../../cairn-distill/tests/fixtures/collection.json"
        ))
        .expect("map packet");
    let filter = TeacherPacketCompiler
        .parse(include_bytes!(
            "../../cairn-distill/tests/fixtures/filter.json"
        ))
        .expect("filter packet");
    packet
        .collection_mechanism_lessons
        .extend(filter.collection_mechanism_lessons);
    let verified = DistillationVerifier
        .verify(
            TeacherPacketCompiler
                .compile(&packet, InventionLimits::default())
                .expect("compile"),
        )
        .expect("verify independent lessons");
    assert_eq!(verified.collection_mechanisms().len(), 2);
    let procedures = CollectionDistillationProposer
        .propose(
            &verified,
            &SupportGeometry {
                observed_domains: BTreeSet::from([DomainClass("generic-collection".to_owned())]),
                observed_cardinalities: BTreeSet::from([CardinalityRange {
                    minimum: 2,
                    maximum: 8,
                }]),
                varied_variables: BTreeSet::from([VariableId("collection".to_owned())]),
                invariant_variables: BTreeSet::new(),
                unknown_variables: BTreeSet::new(),
                validated_compositions: BTreeSet::new(),
                failed_conditions: BTreeSet::new(),
            },
        )
        .expect("two provisional procedures");
    let runtime = CognitionRuntime::with_memories(MechanismStore::new(), procedures);
    let problem = SequenceAdapter.lower_collection(
        32,
        &[
            CollectionSequenceExample {
                input: vec![-8, -2, 1, 5],
                output: vec![-15, -3],
            },
            CollectionSequenceExample {
                input: vec![-10, -4, -1, 0, 7],
                output: vec![-19, -7, -1],
            },
        ],
        vec![-12, -5, 0, 3, 9],
    );
    assert_eq!(
        runtime.solve_collection(&problem),
        Err(ProceduralSolveError::NoVerifiedProposal)
    );
    let composed = runtime
        .solve_collection_pair(&problem)
        .expect("pair composition");
    assert_eq!(composed.predictions.len(), 1);
    assert_eq!(composed.predictions[0].modules.len(), 2);
    assert_eq!(composed.predictions[0].output, BTreeSet::from([-23, -9]));
}
