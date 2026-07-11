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
