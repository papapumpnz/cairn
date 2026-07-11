use cairn_arc::{
    attempt_task_with_runtime, run_training_batch_with_runtime, ArcGrid, ArcPair, ArcTask,
    ArcTestCase,
};
use cairn_distill::{CollectionDistillationProposer, DistillationVerifier, TeacherPacketCompiler};
use cairn_invent::InventionLimits;
use cairn_mechanism::{CardinalityRange, DomainClass, SupportGeometry, VariableId};
use cairn_memory::MechanismStore;
use cairn_rkl::ExecLimits;
use cairn_runtime::CognitionRuntime;
use std::collections::BTreeSet;
use std::path::Path;

#[test]
fn generic_value_collection_reconstructs_a_finite_field_exactly() {
    let runtime = distilled_runtime();
    let task = ArcTask {
        train: vec![
            pair(&[&[0, 1], &[2, 3]], &[&[1, 3], &[5, 7]]),
            pair(&[&[4, 1, 0]], &[&[9, 3, 1]]),
        ],
        test: vec![ArcTestCase {
            input: grid(&[&[2, 4], &[0, 3]]),
            output: None,
        }],
    };
    let attempt = attempt_task_with_runtime(&task, limits(), ExecLimits::default(), &runtime)
        .expect("field transfer");
    assert_eq!(attempt.predictions, vec![grid(&[&[5, 9], &[1, 7]])]);
    assert!(attempt
        .learned_expression
        .starts_with("generic_collection_module="));
}

#[test]
fn frozen_arc_slice_has_no_negative_transfer_from_collection_memory() {
    let corpus =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../datasets/arc-agi-1-v1.0.2/training");
    let report = run_training_batch_with_runtime(
        &corpus,
        20,
        limits(),
        ExecLimits::default(),
        &distilled_runtime(),
    )
    .expect("frozen batch");
    assert_eq!(report.exact_tasks, 17);
    assert_eq!(report.incorrect_tasks, 0);
    assert_eq!(report.resource_limited_tasks, 3);
}

fn distilled_runtime() -> CognitionRuntime {
    let packet = TeacherPacketCompiler
        .parse(include_bytes!(
            "../../../crates/cairn-distill/tests/fixtures/collection.json"
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
                    DomainClass("generic-collection".to_owned()),
                    DomainClass("finite-field".to_owned()),
                ]),
                observed_cardinalities: BTreeSet::from([CardinalityRange {
                    minimum: 1,
                    maximum: 7,
                }]),
                varied_variables: BTreeSet::from([VariableId("collection".to_owned())]),
                invariant_variables: BTreeSet::new(),
                unknown_variables: BTreeSet::new(),
                validated_compositions: BTreeSet::new(),
                failed_conditions: BTreeSet::new(),
            },
        )
        .expect("procedures");
    CognitionRuntime::with_memories(MechanismStore::new(), procedures)
}

fn limits() -> InventionLimits {
    InventionLimits {
        maximum_cost: 9,
        maximum_candidates: 1_000,
        maximum_expression_nodes: 10_000,
        execution: ExecLimits::default(),
    }
}

fn pair(input: &[&[u8]], output: &[&[u8]]) -> ArcPair {
    ArcPair {
        input: grid(input),
        output: grid(output),
    }
}
fn grid(rows: &[&[u8]]) -> ArcGrid {
    ArcGrid {
        rows: rows.iter().map(|row| row.to_vec()).collect(),
    }
}
