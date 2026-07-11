use cairn_arc::{
    attempt_task_with_runtime, attempt_task_with_runtime_and_partition_bindings, parse_task,
    run_training_batch_with_runtime_and_partition_bindings, ArcAdapterError,
};
use cairn_field::{
    FiniteField, LearnedStructuralFieldBinding, PartitionBroadcastLearner,
    PartitionExtractionLearner,
};
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
        &[LearnedStructuralFieldBinding::PartitionBroadcast(
            binding.clone(),
        )],
    )
    .expect("transferred solve");
    assert_eq!(transferred.exact_tests, 1);
    assert!(transferred
        .learned_expression
        .starts_with("transferred_structural_binding="));

    let report = run_training_batch_with_runtime_and_partition_bindings(
        &corpus,
        40,
        limits(),
        ExecLimits::default(),
        &CognitionRuntime::new(),
        &[LearnedStructuralFieldBinding::PartitionBroadcast(binding)],
    )
    .expect("ARC-40 transfer ablation");
    assert_eq!(report.exact_tasks, 18);
    assert_eq!(report.incorrect_tasks, 0);
    assert_eq!(report.resource_limited_tasks, 22);
}

#[test]
fn non_arc_relational_panel_selection_adds_an_arc_solve() {
    let corpus =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../datasets/arc-agi-1-v1.0.2/training");
    let task = parse_task(&fs::read(corpus.join("2dc579da.json")).expect("task")).expect("parse");
    let binding = LearnedStructuralFieldBinding::PartitionExtraction(non_arc_extraction_binding());
    let transferred = attempt_task_with_runtime_and_partition_bindings(
        &task,
        limits(),
        ExecLimits::default(),
        &CognitionRuntime::new(),
        &[binding],
    )
    .expect("relational panel extraction transfer");
    assert_eq!(transferred.exact_tests, 1);
}

#[test]
fn complete_structural_binding_applicability_has_no_false_acceptance() {
    let corpus =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../datasets/arc-agi-1-v1.0.2/training");
    let bindings = structural_bindings();
    let mut applicable = Vec::new();
    let mut exact = Vec::new();
    let mut paths = fs::read_dir(&corpus)
        .expect("corpus")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "json")
        })
        .collect::<Vec<_>>();
    paths.sort();
    for path in paths {
        let task = parse_task(&fs::read(&path).expect("task")).expect("parse");
        for binding in &bindings {
            let admitted = task.train.iter().all(|pair| {
                binding.predict(&arc_field(&pair.input.rows)).as_ref()
                    == Ok(&arc_field(&pair.output.rows))
            });
            if !admitted {
                continue;
            }
            let name = path
                .file_stem()
                .and_then(|name| name.to_str())
                .expect("name")
                .to_owned();
            applicable.push(name.clone());
            if task.test.iter().all(|case| {
                case.output.as_ref().is_some_and(|expected| {
                    binding.predict(&arc_field(&case.input.rows)).as_ref()
                        == Ok(&arc_field(&expected.rows))
                })
            }) {
                exact.push(name);
            }
        }
    }
    applicable.sort();
    applicable.dedup();
    exact.sort();
    exact.dedup();
    eprintln!("STRUCTURAL_COVERAGE applicable={applicable:?} exact={exact:?}");
    assert_eq!(applicable, exact);
    assert!(exact.contains(&"09629e4f".to_owned()));
    assert!(exact.contains(&"2dc579da".to_owned()));
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
        &structural_bindings(),
    )
    .expect("complete structural transfer");
    fs::write(
        "/tmp/cairn_arc400_structural_20260712.json",
        serde_json::to_string_pretty(&report).expect("report JSON"),
    )
    .expect("write report");
    assert!(report.exact_tasks >= 35);
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

fn non_arc_extraction_binding() -> cairn_field::LearnedPartitionExtractionBinding {
    let first = extraction_field(2, 9, 0);
    let second = extraction_field(2, 8, 3);
    PartitionExtractionLearner
        .learn(
            &[
                (first.clone(), extraction_output(&first, 2, 0)),
                (second.clone(), extraction_output(&second, 2, 3)),
            ],
            InventionLimits::default(),
        )
        .expect("non-ARC relational extraction curriculum")
}

fn structural_bindings() -> Vec<LearnedStructuralFieldBinding> {
    vec![
        LearnedStructuralFieldBinding::PartitionBroadcast(non_arc_binding()),
        LearnedStructuralFieldBinding::PartitionExtraction(non_arc_extraction_binding()),
    ]
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

fn extraction_field(size: usize, separator: i64, selected: usize) -> FiniteField {
    let panel = size + 1;
    let extent = size * panel + size - 1;
    let mut values = vec![separator; extent * extent];
    for panel_row in 0..size {
        for panel_column in 0..size {
            let index = panel_row * size + panel_column;
            for row in 0..panel {
                for column in 0..panel {
                    let target_row = panel_row * (panel + 1) + row;
                    let target_column = panel_column * (panel + 1) + column;
                    values[target_row * extent + target_column] = if index == selected {
                        1 + i64::try_from(row * panel + column).expect("small")
                    } else if row == column && row < index % panel + 1 {
                        2
                    } else {
                        0
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

fn extraction_output(input: &FiniteField, grid: usize, selected: usize) -> FiniteField {
    let panel = grid + 1;
    let extent = usize::try_from(input.shape[0]).expect("small");
    let panel_row = selected / grid;
    let panel_column = selected % grid;
    let values = (0..panel)
        .flat_map(|row| {
            (0..panel).map(move |column| {
                input.values
                    [(panel_row * (panel + 1) + row) * extent + panel_column * (panel + 1) + column]
            })
        })
        .collect();
    FiniteField {
        shape: vec![u32::try_from(panel).expect("small"); 2],
        values,
    }
}
