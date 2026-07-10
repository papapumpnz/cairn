use cairn_arc::{
    attempt_task, lower_learning_problem, parse_task, run_training_batch, ArcAdapterError,
};
use cairn_invent::InventionLimits;
use cairn_rkl::ExecLimits;

const TASK: &[u8] = br#"{
  "train": [
    {"input": [[0,1],[2,3]], "output": [[1,2],[3,4]]},
    {"input": [[4,5,6]], "output": [[5,6,7]]}
  ],
  "test": [
    {"input": [[7,0,2],[3,1,5]], "output": [[8,1,3],[4,2,6]]}
  ]
}"#;

#[test]
fn first_quarantined_attempt_learns_without_task_specific_operation() {
    let task = parse_task(TASK).expect("task");
    let attempt = attempt_task(
        &task,
        InventionLimits {
            maximum_cost: 5,
            maximum_candidates: 100_000,
            maximum_expression_nodes: 10_000,
            execution: ExecLimits::default(),
        },
        ExecLimits::default(),
    )
    .expect("attempt");
    assert_eq!(attempt.evaluated_tests, 1);
    assert_eq!(attempt.exact_tests, 1);
    assert_eq!(attempt.generic_observations, 5);
    assert_eq!(attempt.relational_nodes, 20);
    assert!(attempt.relational_facts > 0);
    assert!(attempt.learned_expression.contains("Binary"));
}

#[test]
fn static_task_lowers_to_generic_observations_without_test_answers() {
    let task = parse_task(TASK).expect("task");
    let problem = lower_learning_problem(&task, "urn:test:task").expect("generic problem");
    assert_eq!(problem.observations.len(), 5);
    assert_eq!(problem.demonstrations.len(), 2);
    assert_eq!(problem.requested_predictions.len(), 1);
    assert!(problem.observations.iter().all(|observation| {
        observation.modality.0 == "urn:cairn:finite-field:v1" && observation.verify().is_ok()
    }));
}

#[test]
fn malformed_arc_data_is_rejected_at_adapter_boundary() {
    let ragged = br#"{
      "train": [{"input": [[0],[1,2]], "output": [[0],[1]]}],
      "test": [{"input": [[0]]}]
    }"#;
    assert_eq!(parse_task(ragged), Err(ArcAdapterError::RaggedGrid));
}

#[test]
fn batch_runner_scores_only_after_sealing_test_outputs() {
    let fixtures = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let report = run_training_batch(
        &fixtures,
        1,
        InventionLimits {
            maximum_cost: 5,
            maximum_candidates: 100_000,
            maximum_expression_nodes: 10_000,
            execution: ExecLimits::default(),
        },
        ExecLimits::default(),
    )
    .expect("batch");
    assert_eq!(report.selected_tasks, 1);
    assert_eq!(report.exact_tasks, 1);
    assert_eq!(report.reports[0].exact_tests, 1);
}
