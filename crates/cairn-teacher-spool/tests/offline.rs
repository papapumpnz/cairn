use cairn_curriculum::{
    DeterminismStatus, LearningFailureCore, RedactionStatus, SearchFailure, ValueClass,
};
use cairn_teacher_spool::{
    GapDrivenJobBuilder, OfflineTeacherOrchestrator, OfflineTeachingJob, WireProtocol,
};
use serde_json::json;
use std::collections::BTreeSet;

fn job(model: &str) -> OfflineTeachingJob {
    OfflineTeachingJob {
        job_name: "spatial-causal-curriculum".to_owned(),
        model_identifier: model.to_owned(),
        teaching_contract: "Return only the required packet. Never inspect benchmark data."
            .to_owned(),
        curriculum_request: "Generate varied non-benchmark spatial sensor lessons.".to_owned(),
        packet_schema: json!({"type": "object"}),
        maximum_output_tokens: 8_000,
    }
}

fn packet() -> serde_json::Value {
    json!({
        "packet_version": "1.0",
        "domain": "generic_spatial_sensor_reasoning",
        "origin": {
            "kind": "offline_teacher_fixture",
            "identifier": "spool-import-test"
        },
        "semantic_intents": [],
        "mechanism_lessons": [],
        "knowledge_claims": [],
        "procedures": [],
        "counterexamples": [],
        "uncertainties": ["unverified until the ordinary distillation pipeline runs"]
    })
}

#[test]
fn gpt_orchestrator_exports_only_a_responses_spool() {
    let artifact = OfflineTeacherOrchestrator
        .export(&job("gpt-teacher-model"), WireProtocol::ResponsesSchema)
        .expect("offline request");
    let request: serde_json::Value =
        serde_json::from_slice(&artifact.request_bytes).expect("request JSON");
    assert_eq!(request["store"], false);
    assert_eq!(request["text"]["format"]["strict"], true);
    assert!(!artifact.request_bytes.is_empty());
}

#[test]
fn anthropic_orchestrator_exports_only_a_messages_tool_spool() {
    let artifact = OfflineTeacherOrchestrator
        .export(&job("claude-teacher-model"), WireProtocol::MessagesTool)
        .expect("offline request");
    let request: serde_json::Value =
        serde_json::from_slice(&artifact.request_bytes).expect("request JSON");
    assert_eq!(request["tool_choice"]["type"], "tool");
    assert_eq!(request["tools"][0]["name"], "emit_cairn_teacher_packet");
    assert!(!artifact.request_bytes.is_empty());
}

#[test]
fn gpt_orchestrator_imports_a_saved_response_without_transport() {
    let response = serde_json::to_vec(&json!({"output_parsed": packet()})).expect("response");
    let imported = OfflineTeacherOrchestrator
        .import(&response, WireProtocol::ResponsesSchema)
        .expect("offline import");
    assert_eq!(imported.packet.domain, "generic_spatial_sensor_reasoning");
}

#[test]
fn anthropic_orchestrator_imports_a_saved_tool_response_without_transport() {
    let response = serde_json::to_vec(&json!({
        "content": [{"type": "tool_use", "input": packet()}]
    }))
    .expect("response");
    let imported = OfflineTeacherOrchestrator
        .import(&response, WireProtocol::MessagesTool)
        .expect("offline import");
    assert_eq!(imported.packet.origin.identifier, "spool-import-test");
}

#[test]
fn gap_job_contains_only_redacted_structural_evidence() {
    let core = LearningFailureCore {
        semantics_version: "1.0.0".to_owned(),
        failure: SearchFailure::CandidateCapacity { maximum: 1_000 },
        parameter_types: vec![ValueClass::Integer, ValueClass::Integer],
        return_type: ValueClass::Integer,
        demonstrations: 24,
        varying_parameters: 2,
        distinct_output_classes: 4,
        constant_explanation_possible: false,
        direct_projection_candidates: 0,
        single_parameter_explanations: 0,
        exact_two_parameter_families: BTreeSet::new(),
        exact_three_parameter_families: BTreeSet::new(),
        determinism: DeterminismStatus::Deterministic,
        minimum_dependency_arity_lower_bound: 2,
        redaction: RedactionStatus::ContentFree,
    };
    let job = GapDrivenJobBuilder
        .build(
            "redacted-gap",
            "offline-teacher-model",
            json!({"type": "object"}),
            &[core],
            8_000,
        )
        .expect("safe gap job");
    assert!(job.curriculum_request.contains("minimum_dependency_arity"));
    assert!(job
        .curriculum_request
        .contains("hierarchical entity relations"));
    assert!(!job.curriculum_request.contains("private_"));
    assert!(!job.curriculum_request.contains("007bbfb7"));
    assert!(!job.curriculum_request.contains("grid"));
}
