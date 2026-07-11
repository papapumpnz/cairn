use cairn_arc::{
    attempt_task_with_runtime, attempt_task_with_runtime_and_partition_bindings, parse_task,
    run_training_batch_with_runtime_and_partition_bindings, ArcAdapterError,
};
use cairn_field::{FiniteField, PartitionBroadcastLearner};
use cairn_invent::{InventionError, InventionLimits};
use cairn_rkl::ExecLimits;
use cairn_runtime::CognitionRuntime;
use std::fs;
use std::path::Path;

#[test]
fn non_arc_partition_knowledge_adds_an_exact_arc40_solve() {
    let binding = non_arc_binding();
    let corpus =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../datasets/arc-agi-1-v1.0.2/training");
    let task = parse_task(&fs::read(corpus.join("09629e4f.json")).expect("task")).expect("parse");
    for pair in &task.train {
        let input = arc_field(&pair.input.rows);
        let output = arc_field(&pair.output.rows);
        assert_eq!(
            PartitionBroadcastLearner.predict(&binding, &input),
            Ok(output)
        );
    }
    assert!(matches!(
        attempt_task_with_runtime(
            &task,
            limits(),
            ExecLimits::default(),
            &CognitionRuntime::new()
        ),
        Err(ArcAdapterError::Field(FieldError::Invention(
            InventionError::CandidateCapacity { .. }
        )))
    ));
    let transferred = attempt_task_with_runtime_and_partition_bindings(
        &task,
        limits(),
        ExecLimits::default(),
        &CognitionRuntime::new(),
        std::slice::from_ref(&binding),
    )
    .expect("transferred solve");
    assert_eq!(transferred.exact_tests, 1);
    assert!(transferred
        .learned_expression
        .starts_with("transferred_partition_controller="));

    let report = run_training_batch_with_runtime_and_partition_bindings(
        &corpus,
        40,
        limits(),
        ExecLimits::default(),
        &CognitionRuntime::new(),
        &[binding],
    )
    .expect("ARC-40 transfer ablation");
    assert_eq!(report.exact_tasks, 18);
    assert_eq!(report.incorrect_tasks, 0);
    assert_eq!(report.resource_limited_tasks, 22);
}

#[test]
#[ignore = "complete 400-task structural-transfer ablation; run explicitly"]
fn complete_arc400_structural_transfer_ablation() {
    let corpus =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../datasets/arc-agi-1-v1.0.2/training");
    let report = run_training_batch_with_runtime_and_partition_bindings(
        &corpus,
        400,
        limits(),
        ExecLimits::default(),
        &CognitionRuntime::new(),
        &[non_arc_binding()],
    )
    .expect("complete structural transfer");
    fs::write(
        "/tmp/cairn_arc400_structural_20260712.json",
        serde_json::to_string_pretty(&report).expect("report JSON"),
    )
    .expect("write report");
    assert!(report.exact_tasks >= 33);
    assert_eq!(report.incorrect_tasks, 0);
}

use cairn_field::FieldError;

fn non_arc_binding() -> cairn_field::LearnedPartitionBroadcastBinding {
    let first = dashboard(2, 9, &[1, 2, 3, 0], 11, 0, 99);
    let second = dashboard(2, 8, &[4, 0, 2, 6], 21, 3, 77);
    PartitionBroadcastLearner
        .learn(&[
            (first.clone(), dashboard_output(&first, 2, 0)),
            (second.clone(), dashboard_output(&second, 2, 3)),
        ])
        .expect("non-ARC curriculum")
}

fn dashboard(
    size: usize,
    separator: i64,
    controller: &[i64],
    seed: i64,
    controller_panel: usize,
    marker: i64,
) -> FiniteField {
    let extent = size * size + size - 1;
    let mut values = vec![separator; extent * extent];
    for panel_row in 0..size {
        for panel_column in 0..size {
            for local_row in 0..size {
                for local_column in 0..size {
                    let row = panel_row * (size + 1) + local_row;
                    let column = panel_column * (size + 1) + local_column;
                    values[row * extent + column] =
                        if panel_row * size + panel_column == controller_panel {
                            controller[local_row * size + local_column]
                        } else if local_row == 0 && local_column == 0 {
                            marker
                        } else {
                            seed + i64::try_from(
                                panel_row * 17 + panel_column * 11 + local_row * 3 + local_column,
                            )
                            .expect("small")
                        };
                }
            }
        }
    }
    FiniteField {
        shape: vec![u32::try_from(extent).expect("small"); 2],
        values,
    }
}

fn dashboard_output(input: &FiniteField, size: usize, controller_panel: usize) -> FiniteField {
    let extent = usize::try_from(input.shape[0]).expect("small");
    let mut values = vec![input.values[size]; input.values.len()];
    let controller_row = (controller_panel / size) * (size + 1);
    let controller_column = (controller_panel % size) * (size + 1);
    for panel_row in 0..size {
        for panel_column in 0..size {
            let control = input.values
                [(controller_row + panel_row) * extent + controller_column + panel_column];
            for local_row in 0..size {
                for local_column in 0..size {
                    let row = panel_row * (size + 1) + local_row;
                    let column = panel_column * (size + 1) + local_column;
                    values[row * extent + column] = control;
                }
            }
        }
    }
    FiniteField {
        shape: input.shape.clone(),
        values,
    }
}

fn limits() -> InventionLimits {
    InventionLimits {
        maximum_cost: 9,
        maximum_candidates: 1_000,
        maximum_expression_nodes: 10_000,
        execution: ExecLimits::default(),
    }
}

fn arc_field(rows: &[Vec<u8>]) -> FiniteField {
    FiniteField {
        shape: vec![
            u32::try_from(rows.len()).expect("small"),
            u32::try_from(rows[0].len()).expect("small"),
        ],
        values: rows
            .iter()
            .flatten()
            .map(|value| i64::from(*value))
            .collect(),
    }
}
