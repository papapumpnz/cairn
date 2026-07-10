use cairn_arc::{attempt_task, parse_task, ArcAdapterError};
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
    assert!(attempt.learned_expression.contains("Binary"));
}

#[test]
fn malformed_arc_data_is_rejected_at_adapter_boundary() {
    let ragged = br#"{
      "train": [{"input": [[0],[1,2]], "output": [[0],[1]]}],
      "test": [{"input": [[0]]}]
    }"#;
    assert_eq!(parse_task(ragged), Err(ArcAdapterError::RaggedGrid));
}
