use cairn_arc::{
    attempt_task, attempt_task_with_runtime, parse_task, run_training_batch,
    run_training_batch_with_runtime,
};
use cairn_distill::{
    AgentId, DistillationVerifier, PersistentDistillationPromoter, ScalarExample,
    ScalarMechanismLesson, SharedKnowledgeRegistry, TeacherOrigin, TeacherPacket,
    TeacherPacketCompiler,
};
use cairn_invent::InventionLimits;
use cairn_mechanism::{
    CardinalityRange, DomainClass, InventionEvidence, SupportGeometry as MechanismSupport,
    VariableId,
};
use cairn_memory::{ModuleStatus, SupportGeometry, VariationDimension};
use cairn_rkl::ExecLimits;
use cairn_runtime::CognitionRuntime;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;
use std::time::Instant;

const PARAMETERS: [&str; 13] = [
    "center",
    "north",
    "south",
    "west",
    "east",
    "north_west",
    "north_east",
    "south_west",
    "south_east",
    "row",
    "column",
    "rows",
    "columns",
];

#[derive(Clone, Copy)]
enum GptSpatialLaw {
    FillNorth,
    FillSouth,
    FillWest,
    FillEast,
    MaximumNorthSouth,
    MinimumNorthSouth,
    MaximumWestEast,
    MinimumWestEast,
    VisibleNorth,
    VisibleSouth,
    VisibleWest,
    VisibleEast,
    FillVisibleNorth,
    FillVisibleSouth,
    FillVisibleWest,
    FillVisibleEast,
    MaximumCenterVisibleNorth,
    MaximumCenterVisibleSouth,
    MaximumCenterVisibleWest,
    MaximumCenterVisibleEast,
}

#[allow(clippy::too_many_lines)] // Keep the complete trust and transfer chain auditable.
fn distilled_runtime() -> CognitionRuntime {
    let packet = TeacherPacket {
        packet_version: "1.0".to_owned(),
        domain: "generic_spatial_sensor_fields".to_owned(),
        origin: TeacherOrigin {
            kind: "offline_teacher_fixture".to_owned(),
            identifier: "north-neighbor-curriculum-v1".to_owned(),
        },
        semantic_intents: Vec::new(),
        mechanism_lessons: vec![lesson("sensor_law_a", 10), lesson("sensor_law_b", 200)],
        collection_mechanism_lessons: Vec::new(),
        knowledge_claims: Vec::new(),
        procedures: Vec::new(),
        counterexamples: Vec::new(),
        uncertainties: vec!["not tested with missing neighbor channels".to_owned()],
    };
    let compiled = TeacherPacketCompiler
        .compile(
            &packet,
            InventionLimits {
                maximum_cost: 2,
                maximum_candidates: 100,
                maximum_expression_nodes: 1_000,
                execution: ExecLimits::default(),
            },
        )
        .expect("typed non-ARC lesson");
    let verified = DistillationVerifier
        .verify(compiled)
        .expect("dual-executor teacher lesson replay");
    assert_eq!(verified.mechanisms().len(), 2);

    let mut shared = SharedKnowledgeRegistry::new();
    let artifact = shared
        .publish(verified, AgentId("distillation-agent".to_owned()))
        .expect("publish");
    let replay_receipt = DistillationVerifier
        .verify_peer(
            TeacherPacketCompiler
                .compile(&packet, InventionLimits::default())
                .expect("peer packet"),
            AgentId("spatial-replay-agent".to_owned()),
        )
        .expect("peer receipt");
    shared
        .peer_verify(artifact, replay_receipt)
        .expect("peer replay");
    let counterexample_receipt = DistillationVerifier
        .verify_peer(
            TeacherPacketCompiler
                .compile(&packet, InventionLimits::default())
                .expect("counterexample packet"),
            AgentId("counterexample-agent".to_owned()),
        )
        .expect("counterexample receipt");
    shared
        .peer_verify(artifact, counterexample_receipt)
        .expect("peer falsification");
    shared
        .record_transfer(artifact, 12)
        .expect("changed sensor layouts");
    let accepted = shared.accepted(artifact).expect("swarm-accepted packet");

    let memory = PersistentDistillationPromoter
        .promote(
            accepted.packet(),
            SupportGeometry {
                independent_tasks: 2,
                held_out_rebindings: 12,
                varied_dimensions: BTreeSet::from([
                    VariationDimension::NumericRanges,
                    VariationDimension::CategoricalValues,
                    VariationDimension::Positions,
                    VariationDimension::Layouts,
                ]),
            },
            MechanismSupport {
                observed_domains: BTreeSet::from([
                    DomainClass("spatial-sensor-field".to_owned()),
                    DomainClass("table-neighborhood".to_owned()),
                ]),
                observed_cardinalities: BTreeSet::from([CardinalityRange {
                    minimum: 3,
                    maximum: 12,
                }]),
                varied_variables: PARAMETERS
                    .iter()
                    .map(|name| VariableId((*name).to_owned()))
                    .collect(),
                invariant_variables: BTreeSet::new(),
                unknown_variables: BTreeSet::new(),
                validated_compositions: BTreeSet::new(),
                failed_conditions: BTreeSet::new(),
            },
            &InventionEvidence {
                target_absent_from_named_vocabulary: true,
                source_code_changes: 0,
                independent_experiences: 2,
                initial_success_parts_per_million: 0,
                post_experience_success_parts_per_million: 1_000_000,
                uncompressed_cost: 2,
                compressed_cost: 1,
                held_out_verified_cases: 12,
                candidate_executions_without: 2,
                candidate_executions_with: 1,
                examples_without: 3,
                examples_with: 1,
                false_acceptances_without: 0,
                false_acceptances_with: 0,
                actual_reuse_count: 1,
                necessary_reuse_count: 1,
            },
        )
        .expect("strict persistent general mechanism");
    assert_eq!(
        memory.reusable_modules().next().expect("module").status(),
        ModuleStatus::Persistent
    );
    CognitionRuntime::with_memory(memory)
}

#[test]
fn non_arc_teacher_lesson_transfers_as_verified_arc_proposal() {
    let runtime = distilled_runtime();
    let path = format!(
        "{}/../../datasets/arc-agi-1-v1.0.2/training/25ff71a9.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let task = parse_task(&fs::read(path).expect("pinned official task")).expect("valid task");
    let limits = InventionLimits {
        maximum_cost: 5,
        maximum_candidates: 1,
        maximum_expression_nodes: 10_000,
        execution: ExecLimits::default(),
    };
    assert!(attempt_task(&task, limits, ExecLimits::default()).is_err());
    let transferred = attempt_task_with_runtime(&task, limits, ExecLimits::default(), &runtime)
        .expect("transferred proposal solves under equal one-candidate budget");
    assert_eq!(transferred.exact_tests, transferred.evaluated_tests);
    assert!(transferred.learned_expression.contains("MemoryProposal"));
    assert!(transferred.learned_expression.contains("search_work=1"));
}

#[test]
#[ignore = "complete 400-task scientific ablation; run explicitly"]
fn complete_distilled_memory_ablation() {
    let corpus =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../datasets/arc-agi-1-v1.0.2/training");
    let artifacts = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../artifacts/arc-agi-1-v1.0.2");
    let limits = InventionLimits {
        maximum_cost: 9,
        maximum_candidates: 1_000,
        maximum_expression_nodes: 10_000,
        execution: ExecLimits::default(),
    };
    let without = run_training_batch(&corpus, 400, limits, ExecLimits::default())
        .expect("complete memory-off run");
    fs::write(
        artifacts.join("full-400-distill-memory-off-v1.15.0.json"),
        format!(
            "{}\n",
            serde_json::to_string_pretty(&without).expect("serialize memory-off")
        ),
    )
    .expect("write memory-off report");
    let runtime = distilled_runtime();
    let with =
        run_training_batch_with_runtime(&corpus, 400, limits, ExecLimits::default(), &runtime)
            .expect("complete memory-on run");
    fs::write(
        artifacts.join("full-400-distill-memory-on-v1.15.0.json"),
        format!(
            "{}\n",
            serde_json::to_string_pretty(&with).expect("serialize memory-on")
        ),
    )
    .expect("write memory-on report");
    assert_eq!(without.selected_tasks, 400);
    assert_eq!(with.selected_tasks, 400);
    assert_eq!(without.incorrect_tasks, 0);
    assert_eq!(with.incorrect_tasks, 0);
    assert!(with.exact_tasks >= without.exact_tasks);
    eprintln!(
        "DISTILLED_ARC_ABLATION exact={}->{} resource_limited={}->{} incorrect={}->{}",
        without.exact_tasks,
        with.exact_tasks,
        without.resource_limited_tasks,
        with.resource_limited_tasks,
        without.incorrect_tasks,
        with.incorrect_tasks
    );
}

#[test]
#[ignore = "complete 400-task contextual-retrieval audit; run explicitly"]
fn complete_contextual_distilled_memory_run() {
    let corpus =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../datasets/arc-agi-1-v1.0.2/training");
    let artifacts = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../artifacts/arc-agi-1-v1.0.2");
    let limits = InventionLimits {
        maximum_cost: 9,
        maximum_candidates: 1_000,
        maximum_expression_nodes: 10_000,
        execution: ExecLimits::default(),
    };
    let runtime = distilled_runtime();
    let started = Instant::now();
    let report =
        run_training_batch_with_runtime(&corpus, 400, limits, ExecLimits::default(), &runtime)
            .expect("complete contextual memory run");
    let elapsed = started.elapsed().as_secs_f64();
    fs::write(
        artifacts.join("full-400-distill-contextual-memory-v1.15.0-memory1.1.0.json"),
        format!(
            "{}\n",
            serde_json::to_string_pretty(&report).expect("serialize contextual report")
        ),
    )
    .expect("write contextual report");
    assert_eq!(report.selected_tasks, 400);
    assert_eq!(report.exact_tasks, 32);
    assert_eq!(report.incorrect_tasks, 0);
    eprintln!(
        "DISTILLED_ARC_CONTEXTUAL exact={} resource_limited={} incorrect={} elapsed_seconds={elapsed:.2}",
        report.exact_tasks, report.resource_limited_tasks, report.incorrect_tasks
    );
}

#[test]
fn gpt_teacher_curriculum_compiles_to_provisional_non_arc_library() {
    let runtime = gpt_teacher_runtime();
    assert!(runtime.memory().len() >= 16);
    assert!(runtime
        .memory()
        .reusable_modules()
        .all(|module| module.status() == ModuleStatus::Provisional));
}

#[test]
#[ignore = "complete 400-task GPT-teacher transfer audit; run explicitly"]
fn complete_gpt_teacher_arc_run() {
    let corpus =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../datasets/arc-agi-1-v1.0.2/training");
    let artifacts = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../artifacts/arc-agi-1-v1.0.2");
    let packet = gpt_teacher_packet();
    let packet_bytes = serde_json::to_vec_pretty(&packet).expect("serialize GPT packet");
    fs::create_dir_all(artifacts.join("distillation")).expect("distillation artifact directory");
    fs::write(
        artifacts.join("distillation/gpt-spatial-priors-v1.json"),
        &packet_bytes,
    )
    .expect("write GPT teacher packet");
    let runtime = gpt_teacher_runtime();
    let limits = InventionLimits {
        maximum_cost: 9,
        maximum_candidates: 1_000,
        maximum_expression_nodes: 10_000,
        execution: ExecLimits::default(),
    };
    let started = Instant::now();
    let report =
        run_training_batch_with_runtime(&corpus, 400, limits, ExecLimits::default(), &runtime)
            .expect("complete GPT-teacher ARC run");
    let elapsed = started.elapsed().as_secs_f64();
    fs::write(
        artifacts.join("full-400-gpt-teacher-memory-v1.15.0.json"),
        format!(
            "{}\n",
            serde_json::to_string_pretty(&report).expect("serialize GPT report")
        ),
    )
    .expect("write GPT report");
    assert_eq!(report.selected_tasks, 400);
    assert_eq!(report.incorrect_tasks, 0);
    assert!(report.exact_tasks >= 32);
    eprintln!(
        "GPT_TEACHER_ARC exact={} resource_limited={} incorrect={} elapsed_seconds={elapsed:.2} modules={}",
        report.exact_tasks,
        report.resource_limited_tasks,
        report.incorrect_tasks,
        runtime.memory().len()
    );
}

fn gpt_teacher_runtime() -> CognitionRuntime {
    let packet = gpt_teacher_packet();
    let limits = InventionLimits {
        maximum_cost: 7,
        maximum_candidates: 2_000_000,
        maximum_expression_nodes: 10_000,
        execution: ExecLimits::default(),
    };
    let verified = DistillationVerifier
        .verify(
            TeacherPacketCompiler
                .compile(&packet, limits)
                .expect("compile GPT teacher curriculum"),
        )
        .expect("verify GPT teacher curriculum");
    let mut shared = SharedKnowledgeRegistry::new();
    let artifact = shared
        .publish(verified, AgentId("gpt-teacher-compiler".to_owned()))
        .expect("publish GPT curriculum");
    for peer in ["gpt-replay-peer", "gpt-falsifier-peer"] {
        let receipt = DistillationVerifier
            .verify_peer(
                TeacherPacketCompiler
                    .compile(&packet, limits)
                    .expect("peer compile GPT curriculum"),
                AgentId(peer.to_owned()),
            )
            .expect("peer replay GPT curriculum");
        shared
            .peer_verify(artifact, receipt)
            .expect("record GPT peer");
    }
    shared
        .record_transfer(artifact, 64)
        .expect("record GPT non-ARC transfer cases");
    let accepted = shared.accepted(artifact).expect("accepted GPT curriculum");
    let memory = PersistentDistillationPromoter
        .propose_pairs(
            accepted.packet(),
            &SupportGeometry {
                independent_tasks: 2,
                held_out_rebindings: 64,
                varied_dimensions: BTreeSet::from([
                    VariationDimension::NumericRanges,
                    VariationDimension::CategoricalValues,
                    VariationDimension::Counts,
                    VariationDimension::Positions,
                    VariationDimension::Layouts,
                ]),
            },
        )
        .expect("verified provisional GPT mechanism library");
    CognitionRuntime::with_memory(memory)
}

fn gpt_teacher_packet() -> TeacherPacket {
    let laws = [
        GptSpatialLaw::FillNorth,
        GptSpatialLaw::FillSouth,
        GptSpatialLaw::FillWest,
        GptSpatialLaw::FillEast,
        GptSpatialLaw::MaximumNorthSouth,
        GptSpatialLaw::MinimumNorthSouth,
        GptSpatialLaw::MaximumWestEast,
        GptSpatialLaw::MinimumWestEast,
        GptSpatialLaw::VisibleNorth,
        GptSpatialLaw::VisibleSouth,
        GptSpatialLaw::VisibleWest,
        GptSpatialLaw::VisibleEast,
        GptSpatialLaw::FillVisibleNorth,
        GptSpatialLaw::FillVisibleSouth,
        GptSpatialLaw::FillVisibleWest,
        GptSpatialLaw::FillVisibleEast,
        GptSpatialLaw::MaximumCenterVisibleNorth,
        GptSpatialLaw::MaximumCenterVisibleSouth,
        GptSpatialLaw::MaximumCenterVisibleWest,
        GptSpatialLaw::MaximumCenterVisibleEast,
    ];
    let mechanism_lessons = laws
        .into_iter()
        .enumerate()
        .flat_map(|(index, law)| {
            [
                gpt_lesson(&format!("gpt_spatial_{index}_a"), 101, law),
                gpt_lesson(&format!("gpt_spatial_{index}_b"), 503, law),
            ]
        })
        .collect();
    TeacherPacket {
        packet_version: "1.0".to_owned(),
        domain: "non_benchmark_spatial_sensor_reasoning".to_owned(),
        origin: TeacherOrigin {
            kind: "gpt_offline_teacher_session".to_owned(),
            identifier: "cairn-gpt-spatial-curriculum-2026-07-11".to_owned(),
        },
        semantic_intents: Vec::new(),
        mechanism_lessons,
        collection_mechanism_lessons: Vec::new(),
        knowledge_claims: Vec::new(),
        procedures: Vec::new(),
        counterexamples: Vec::new(),
        uncertainties: vec![
            "not validated for overlapping or missing sensor channels".to_owned(),
            "not claimed to encode any benchmark-specific transformation".to_owned(),
        ],
    }
}

fn gpt_lesson(name: &str, seed: i64, law: GptSpatialLaw) -> ScalarMechanismLesson {
    let parameters = gpt_parameters(law);
    let centers = [0, 5, 0, -4, 0, 2];
    let examples = (0..6)
        .map(|example| {
            let mut inputs = parameters
                .iter()
                .enumerate()
                .map(|(index, parameter)| {
                    let mixed = seed
                        .saturating_add(i64::try_from(index).expect("small") * 17)
                        .saturating_add(i64::from(example) * 23);
                    ((*parameter).to_owned(), mixed.rem_euclid(31) - 15)
                })
                .collect::<BTreeMap<_, _>>();
            if inputs.contains_key("center") {
                inputs.insert(
                    "center".to_owned(),
                    centers[usize::try_from(example).expect("small")],
                );
            }
            for extent in [
                "row",
                "column",
                "rows",
                "columns",
                "row_occupied_count",
                "column_occupied_count",
            ] {
                if inputs.contains_key(extent) {
                    inputs.insert(extent.to_owned(), i64::from(example) + 2);
                }
            }
            ScalarExample {
                output: gpt_output(law, &inputs),
                inputs,
            }
        })
        .collect();
    ScalarMechanismLesson {
        name: name.to_owned(),
        parameters: parameters.iter().map(|name| (*name).to_owned()).collect(),
        examples,
        literals: vec![-1, 0, 1, 2],
    }
}

fn gpt_parameters(law: GptSpatialLaw) -> &'static [&'static str] {
    match law {
        GptSpatialLaw::FillNorth => &["center", "north"],
        GptSpatialLaw::FillSouth => &["center", "south"],
        GptSpatialLaw::FillWest => &["center", "west"],
        GptSpatialLaw::FillEast => &["center", "east"],
        GptSpatialLaw::MaximumNorthSouth | GptSpatialLaw::MinimumNorthSouth => &["north", "south"],
        GptSpatialLaw::MaximumWestEast | GptSpatialLaw::MinimumWestEast => &["west", "east"],
        GptSpatialLaw::VisibleNorth => &["visible_north"],
        GptSpatialLaw::VisibleSouth => &["visible_south"],
        GptSpatialLaw::VisibleWest => &["visible_west"],
        GptSpatialLaw::VisibleEast => &["visible_east"],
        GptSpatialLaw::FillVisibleNorth | GptSpatialLaw::MaximumCenterVisibleNorth => {
            &["center", "visible_north"]
        }
        GptSpatialLaw::FillVisibleSouth | GptSpatialLaw::MaximumCenterVisibleSouth => {
            &["center", "visible_south"]
        }
        GptSpatialLaw::FillVisibleWest | GptSpatialLaw::MaximumCenterVisibleWest => {
            &["center", "visible_west"]
        }
        GptSpatialLaw::FillVisibleEast | GptSpatialLaw::MaximumCenterVisibleEast => {
            &["center", "visible_east"]
        }
    }
}

fn gpt_output(law: GptSpatialLaw, inputs: &BTreeMap<String, i64>) -> i64 {
    let fill = |name: &str| {
        let center = inputs["center"];
        if center == 0 {
            inputs[name]
        } else {
            center
        }
    };
    match law {
        GptSpatialLaw::FillNorth => fill("north"),
        GptSpatialLaw::FillSouth => fill("south"),
        GptSpatialLaw::FillWest => fill("west"),
        GptSpatialLaw::FillEast => fill("east"),
        GptSpatialLaw::MaximumNorthSouth => inputs["north"].max(inputs["south"]),
        GptSpatialLaw::MinimumNorthSouth => inputs["north"].min(inputs["south"]),
        GptSpatialLaw::MaximumWestEast => inputs["west"].max(inputs["east"]),
        GptSpatialLaw::MinimumWestEast => inputs["west"].min(inputs["east"]),
        GptSpatialLaw::VisibleNorth => inputs["visible_north"],
        GptSpatialLaw::VisibleSouth => inputs["visible_south"],
        GptSpatialLaw::VisibleWest => inputs["visible_west"],
        GptSpatialLaw::VisibleEast => inputs["visible_east"],
        GptSpatialLaw::FillVisibleNorth => fill("visible_north"),
        GptSpatialLaw::FillVisibleSouth => fill("visible_south"),
        GptSpatialLaw::FillVisibleWest => fill("visible_west"),
        GptSpatialLaw::FillVisibleEast => fill("visible_east"),
        GptSpatialLaw::MaximumCenterVisibleNorth => inputs["center"].max(inputs["visible_north"]),
        GptSpatialLaw::MaximumCenterVisibleSouth => inputs["center"].max(inputs["visible_south"]),
        GptSpatialLaw::MaximumCenterVisibleWest => inputs["center"].max(inputs["visible_west"]),
        GptSpatialLaw::MaximumCenterVisibleEast => inputs["center"].max(inputs["visible_east"]),
    }
}

fn lesson(name: &str, seed: i64) -> ScalarMechanismLesson {
    let examples = (0..3)
        .map(|example| {
            let inputs = PARAMETERS
                .iter()
                .enumerate()
                .map(|(index, name)| {
                    (
                        (*name).to_owned(),
                        seed + i64::try_from(index).expect("small") * 10 + example,
                    )
                })
                .collect::<BTreeMap<_, _>>();
            ScalarExample {
                output: inputs["north"],
                inputs,
            }
        })
        .collect();
    ScalarMechanismLesson {
        name: name.to_owned(),
        parameters: PARAMETERS.iter().map(|name| (*name).to_owned()).collect(),
        examples,
        literals: vec![0, 1],
    }
}
