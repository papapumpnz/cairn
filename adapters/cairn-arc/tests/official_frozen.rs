use cairn_arc::run_training_batch;
use cairn_invent::InventionLimits;
use cairn_rkl::ExecLimits;
use std::path::Path;

#[test]
fn official_arc_agi_1_frozen_first_ten_are_exact() {
    let corpus =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../datasets/arc-agi-1-v1.0.2/training");
    let report = run_training_batch(
        &corpus,
        10,
        InventionLimits {
            maximum_cost: 9,
            maximum_candidates: 1_000,
            maximum_expression_nodes: 10_000,
            execution: ExecLimits::default(),
        },
        ExecLimits::default(),
    )
    .expect("official frozen batch");

    assert_eq!(report.selected_tasks, 10);
    assert_eq!(report.parsed_tasks, 10);
    assert_eq!(report.attempted_tasks, 10);
    assert_eq!(report.exact_tasks, 10);
    assert_eq!(report.incorrect_tasks, 0);
    assert_eq!(report.unsupported_tasks, 0);
    assert_eq!(report.resource_limited_tasks, 0);
    assert_eq!(report.execution_failures, 0);
}

#[test]
fn official_arc_agi_1_pivot_checkpoint_is_seventeen_of_twenty() {
    let corpus =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../datasets/arc-agi-1-v1.0.2/training");
    let report = run_training_batch(
        &corpus,
        20,
        InventionLimits {
            maximum_cost: 9,
            maximum_candidates: 1_000,
            maximum_expression_nodes: 10_000,
            execution: ExecLimits::default(),
        },
        ExecLimits::default(),
    )
    .expect("official pivot checkpoint");

    assert_eq!(report.selected_tasks, 20);
    assert_eq!(report.exact_tasks, 17);
    assert_eq!(report.resource_limited_tasks, 3);
    assert_eq!(report.incorrect_tasks, 0);
    assert_eq!(report.unsupported_tasks, 0);
    assert_eq!(report.execution_failures, 0);
}
