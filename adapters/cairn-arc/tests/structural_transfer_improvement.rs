use cairn_arc::{
    attempt_task_with_runtime, attempt_task_with_runtime_and_structural_bindings, parse_task,
    run_training_batch_with_runtime_and_structural_bindings, ArcAdapterError,
};
use cairn_distill::{DistillationVerifier, TeacherPacketCompiler};
use cairn_field::{FiniteField, LearnedStructuralFieldBinding, PartitionBroadcastLearner};
use cairn_invent::{InventionError, InventionLimits};
use cairn_memory::LayerMechanismStore;
use cairn_rkl::ExecLimits;
use cairn_runtime::CognitionRuntime;
use cairn_theory::{
    CoordinateLayer, EntityCorrespondenceExample, EntityCorrespondenceLimits, EntityGeometryRender,
    EntityGeometryScene, GeometricEntity, LayerLearningExample, LayerLearningLimits, LayerRender,
    LayerScene, RenderedEntity,
};
use cairn_vector::VectorAdapter;
use cairn_verifier::LayerMechanismVerifier;
use std::collections::BTreeSet;
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
    let transferred = attempt_task_with_runtime_and_structural_bindings(
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

    let report = run_training_batch_with_runtime_and_structural_bindings(
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
    let transferred = attempt_task_with_runtime_and_structural_bindings(
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
fn offline_ai_entity_role_rebinds_to_an_unseen_field_frontend() {
    let corpus =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../datasets/arc-agi-1-v1.0.2/training");
    let task = parse_task(&fs::read(corpus.join("be94b721.json")).expect("task")).expect("parse");
    assert!(attempt_task_with_runtime(
        &task,
        limits(),
        ExecLimits::default(),
        &CognitionRuntime::new()
    )
    .is_err());
    let transferred = attempt_task_with_runtime_and_structural_bindings(
        &task,
        limits(),
        ExecLimits::default(),
        &CognitionRuntime::new(),
        &[LearnedStructuralFieldBinding::EntityFrameExtraction(
            non_arc_entity_frame_binding(),
        )],
    )
    .expect("cross-frontend entity-role transfer");
    assert_eq!(transferred.exact_tests, 1);
    assert!(transferred
        .learned_expression
        .starts_with("transferred_structural_binding="));
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
    assert!(exact.contains(&"be94b721".to_owned()));
}

#[test]
#[ignore = "complete 400-task structural-transfer ablation; run explicitly"]
fn complete_arc400_structural_transfer_ablation() {
    let corpus =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../datasets/arc-agi-1-v1.0.2/training");
    let report = run_training_batch_with_runtime_and_structural_bindings(
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
    assert!(report.exact_tasks >= 36);
    assert_eq!(report.incorrect_tasks, 0);
}

#[test]
#[ignore = "frozen first-40 all-structural-mechanism ablation; run explicitly"]
fn frozen_arc40_all_structural_mechanisms() {
    let corpus =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../datasets/arc-agi-1-v1.0.2/training");
    let report = run_training_batch_with_runtime_and_structural_bindings(
        &corpus,
        40,
        limits(),
        ExecLimits::default(),
        &CognitionRuntime::new(),
        &structural_bindings(),
    )
    .expect("frozen ARC-40 structural transfer");
    fs::write(
        "/tmp/cairn_arc40_layer_structural_20260713.json",
        serde_json::to_string_pretty(&report).expect("report JSON"),
    )
    .expect("write report");
    assert!(report.exact_tasks >= 18);
    assert_eq!(report.incorrect_tasks, 0);
}

#[test]
#[ignore = "post-result causal ablation; compares blank and layer-only first-40"]
fn posthoc_arc40_layer_memory_causal_ablation() {
    let corpus =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../datasets/arc-agi-1-v1.0.2/training");
    let runtime = CognitionRuntime::new();
    let blank = run_training_batch_with_runtime_and_structural_bindings(
        &corpus,
        40,
        limits(),
        ExecLimits::default(),
        &runtime,
        &[],
    )
    .expect("blank ARC-40 ablation");
    let layer = run_training_batch_with_runtime_and_structural_bindings(
        &corpus,
        40,
        limits(),
        ExecLimits::default(),
        &runtime,
        &[LearnedStructuralFieldBinding::LayerMechanism(
            non_arc_layer_mechanism(),
        )],
    )
    .expect("layer-only ARC-40 ablation");
    fs::write(
        "/tmp/cairn_arc40_layer_causal_ablation_20260713.json",
        serde_json::to_string_pretty(&(blank.clone(), layer.clone())).expect("report JSON"),
    )
    .expect("write report");
    assert!(layer.exact_tasks > blank.exact_tasks);
    assert_eq!(layer.incorrect_tasks, 0);
}

#[test]
#[ignore = "frozen correspondence-only versus blank first-40 causal gate"]
fn frozen_arc40_entity_correspondence_causal_ablation() {
    let corpus =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../datasets/arc-agi-1-v1.0.2/training");
    let runtime = CognitionRuntime::new();
    let blank = run_training_batch_with_runtime_and_structural_bindings(
        &corpus,
        40,
        limits(),
        ExecLimits::default(),
        &runtime,
        &[],
    )
    .expect("blank ARC-40 correspondence ablation");
    let correspondence = run_training_batch_with_runtime_and_structural_bindings(
        &corpus,
        40,
        limits(),
        ExecLimits::default(),
        &runtime,
        &[LearnedStructuralFieldBinding::EntityCorrespondence(
            non_arc_entity_correspondence(),
        )],
    )
    .expect("correspondence-only ARC-40 ablation");
    fs::write(
        "/tmp/cairn_arc40_entity_correspondence_ablation_20260713.json",
        serde_json::to_string_pretty(&(blank.clone(), correspondence.clone()))
            .expect("report JSON"),
    )
    .expect("write report");
    assert!(correspondence.exact_tasks > blank.exact_tasks);
    assert_eq!(correspondence.incorrect_tasks, 0);
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
    let verified = verified_relational_packet();
    cairn_field::LearnedPartitionExtractionBinding {
        selector: verified.entity_selectors()[0].clone(),
    }
}

fn non_arc_entity_frame_binding() -> cairn_field::LearnedEntityFrameExtractionBinding {
    let verified = verified_relational_packet();
    cairn_field::LearnedEntityFrameExtractionBinding {
        selector: verified.entity_selectors()[0].clone(),
    }
}

fn verified_relational_packet() -> cairn_distill::VerifiedTeacherPacket {
    let packet = TeacherPacketCompiler
        .parse(include_bytes!(
            "../../../crates/cairn-distill/tests/fixtures/relational_teacher.json"
        ))
        .expect("offline AI teacher packet");
    DistillationVerifier
        .verify(
            TeacherPacketCompiler
                .compile(&packet, InventionLimits::default())
                .expect("compile teacher packet"),
        )
        .expect("independent held-out verification")
}

fn structural_bindings() -> Vec<LearnedStructuralFieldBinding> {
    vec![
        LearnedStructuralFieldBinding::PartitionBroadcast(non_arc_binding()),
        LearnedStructuralFieldBinding::PartitionExtraction(non_arc_extraction_binding()),
        LearnedStructuralFieldBinding::EntityFrameExtraction(non_arc_entity_frame_binding()),
        LearnedStructuralFieldBinding::LayerMechanism(non_arc_layer_mechanism()),
        LearnedStructuralFieldBinding::EntityCorrespondence(non_arc_entity_correspondence()),
    ]
}

fn non_arc_entity_correspondence() -> cairn_theory::EntityCorrespondenceMechanism {
    let first = [
        graphics_correspondence_example(12, 15, 2, 3, 2),
        graphics_correspondence_example(16, 13, 4, 20, 3),
    ];
    let second = [
        graphics_correspondence_example(18, 20, 5, 40, 3),
        graphics_correspondence_example(21, 17, 7, 60, 4),
    ];
    let learned = VectorAdapter
        .learn_entity_correspondence(&first, EntityCorrespondenceLimits::default())
        .expect("non-ARC correspondence curriculum");
    assert_eq!(
        VectorAdapter
            .learn_entity_correspondence(&second, EntityCorrespondenceLimits::default())
            .expect("independent correspondence curriculum")
            .mechanism,
        learned.mechanism
    );
    let mut store = cairn_memory::EntityCorrespondenceStore::new();
    for examples in [&first[..], &second[..]] {
        let receipt = cairn_verifier::EntityCorrespondenceVerifier
            .verify(&learned.mechanism, examples, 65_536)
            .expect("independent correspondence verification");
        store
            .record_verified_cohort(&learned.mechanism, receipt)
            .expect("correspondence promotion evidence");
    }
    let persistent = store
        .persistent_mechanisms()
        .next()
        .expect("persistent correspondence")
        .clone();
    persistent
}

fn graphics_correspondence_example(
    rows: u32,
    columns: u32,
    shift: i64,
    value: i64,
    count: usize,
) -> EntityCorrespondenceExample {
    let mut entities = vec![
        graphics_entity(
            value,
            &[(shift, shift), (shift + 1, shift), (shift + 1, shift + 1)],
            &format!("hook:{rows}"),
        ),
        graphics_entity(
            value + 1,
            &[(shift + 3, shift + 4), (shift + 3, shift + 5)],
            &format!("bar:{columns}"),
        ),
    ];
    for index in 2..count {
        let offset = i64::try_from(index).expect("small");
        entities.push(graphics_entity(
            value + offset,
            &[(shift + 4 + offset, shift + offset)],
            &format!("extra:{rows}:{index}"),
        ));
    }
    let source = EntityGeometryScene {
        extents: vec![rows, columns],
        background: 0,
        entities,
    };
    let minimum_row = source
        .entities
        .iter()
        .flat_map(|entity| &entity.points)
        .map(|point| point[0])
        .min()
        .expect("row");
    let maximum_row = source
        .entities
        .iter()
        .flat_map(|entity| &entity.points)
        .map(|point| point[0])
        .max()
        .expect("row");
    let minimum_column = source
        .entities
        .iter()
        .flat_map(|entity| &entity.points)
        .map(|point| point[1])
        .min()
        .expect("column");
    let maximum_column = source
        .entities
        .iter()
        .flat_map(|entity| &entity.points)
        .map(|point| point[1])
        .max()
        .expect("column");
    let mut expected_entities = source
        .entities
        .iter()
        .map(|entity| RenderedEntity {
            value: entity.value,
            points: entity
                .points
                .iter()
                .map(|point| vec![point[0] - minimum_row, maximum_column - point[1]])
                .collect(),
        })
        .collect::<Vec<_>>();
    expected_entities.sort_by(|left, right| {
        left.value
            .cmp(&right.value)
            .then_with(|| left.points.cmp(&right.points))
    });
    EntityCorrespondenceExample {
        source,
        expected: EntityGeometryRender {
            extents: vec![
                u32::try_from(maximum_row - minimum_row + 1).expect("extent"),
                u32::try_from(maximum_column - minimum_column + 1).expect("extent"),
            ],
            background: 0,
            entities: expected_entities,
        },
    }
}

fn graphics_entity(value: i64, geometry: &[(i64, i64)], name: &str) -> GeometricEntity {
    GeometricEntity {
        identity: cairn_core::Hash32::digest(name.as_bytes()),
        value,
        points: geometry
            .iter()
            .map(|(row, column)| vec![*row, *column])
            .collect(),
    }
}

fn non_arc_layer_mechanism() -> cairn_theory::LayerMechanism {
    let first = [
        graphics_union_example(9, 2, 11, 30),
        graphics_union_example(13, 4, 17, 50),
    ];
    let second = [
        graphics_union_example(16, 6, 23, 70),
        graphics_union_example(20, 8, 31, 90),
    ];
    let mechanism = VectorAdapter
        .learn_layer_mechanism(&first, LayerLearningLimits::default())
        .expect("non-ARC layer curriculum");
    assert_eq!(
        VectorAdapter
            .learn_layer_mechanism(&second, LayerLearningLimits::default())
            .expect("independent non-ARC layer curriculum"),
        mechanism
    );
    let mut store = LayerMechanismStore::new();
    for examples in [&first[..], &second[..]] {
        let receipt = LayerMechanismVerifier
            .verify(&mechanism, examples, 65_536)
            .expect("independent graphics verification");
        store
            .record_verified_cohort(&mechanism, receipt)
            .expect("graphics promotion evidence");
    }
    let persistent = store
        .persistent_mechanisms()
        .next()
        .expect("persistent graphics mechanism")
        .clone();
    persistent
}

fn graphics_union_example(
    extent: u32,
    shift: i64,
    minimum_value: i64,
    maximum_value: i64,
) -> LayerLearningExample {
    let minimum = coordinate_points(&[
        (shift, shift),
        (shift + 1, shift + 1),
        (shift + 2, shift + 2),
    ]);
    let maximum = coordinate_points(&[
        (shift - 1, shift),
        (shift, shift),
        (shift + 1, shift + 1),
        (shift + 3, shift + 1),
        (shift + 4, shift + 2),
    ]);
    let union = minimum.union(&maximum).cloned().collect();
    LayerLearningExample {
        scene: LayerScene {
            extents: vec![extent, extent],
            background: -1,
            layers: vec![
                CoordinateLayer {
                    value: maximum_value,
                    points: maximum,
                },
                CoordinateLayer {
                    value: minimum_value,
                    points: minimum,
                },
            ],
        },
        expected: LayerRender {
            extents: vec![extent, extent],
            background: -1,
            foreground: maximum_value,
            points: union,
        },
    }
}

fn coordinate_points(values: &[(i64, i64)]) -> BTreeSet<Vec<i64>> {
    values
        .iter()
        .map(|(row, column)| vec![*row, *column])
        .collect()
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
