use cairn_arc::{attempt_task, attempt_task_with_runtime, parse_task};
use cairn_distill::{
    AgentId, DistillationVerifier, ScalarExample, ScalarMechanismLesson, SharedKnowledgeRegistry,
    TeacherOrigin, TeacherPacket, TeacherPacketCompiler,
};
use cairn_invent::InventionLimits;
use cairn_memory::{MechanismStore, PromotionRunner, SupportGeometry, VariationDimension};
use cairn_rkl::ExecLimits;
use cairn_runtime::CognitionRuntime;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;

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

#[test]
fn non_arc_teacher_lesson_transfers_as_verified_arc_proposal() {
    let packet = TeacherPacket {
        packet_version: "1.0".to_owned(),
        domain: "generic_spatial_sensor_fields".to_owned(),
        origin: TeacherOrigin {
            kind: "offline_teacher_fixture".to_owned(),
            identifier: "north-neighbor-curriculum-v1".to_owned(),
        },
        semantic_intents: Vec::new(),
        mechanism_lessons: vec![lesson("sensor_law_a", 10), lesson("sensor_law_b", 200)],
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

    let module = PromotionRunner
        .promote(
            accepted.packet().mechanisms(),
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
        )
        .expect("provisional general mechanism");
    let mut memory = MechanismStore::new();
    memory.insert(module);
    let runtime = CognitionRuntime::with_memory(memory);

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
