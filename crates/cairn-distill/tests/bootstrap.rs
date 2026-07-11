use cairn_distill::{
    AgentId, DistillationTrust, DistillationVerifier, SharedKnowledgeRegistry,
    TeacherPacketCompiler,
};
use cairn_invent::InventionLimits;
use cairn_language::TechnicalChatbot;

#[test]
fn offline_packet_becomes_teacher_independent_swarm_knowledge() {
    let bytes = include_bytes!("fixtures/basic_python.json");
    let packet = TeacherPacketCompiler.parse(bytes).expect("schema packet");
    let compiled = TeacherPacketCompiler
        .compile(&packet, InventionLimits::default())
        .expect("typed compilation");
    let verified = DistillationVerifier
        .verify(compiled)
        .expect("independent execution and language replay");
    assert_eq!(verified.mechanisms().len(), 1);
    assert_eq!(verified.verified_mechanism_examples(), 3);
    assert_eq!(verified.knowledge_candidates().len(), 1);
    assert_eq!(verified.procedures().len(), 1);
    assert_eq!(verified.verified_counterexamples(), 1);

    let mut registry = SharedKnowledgeRegistry::new();
    let id = registry
        .publish(verified, AgentId("compiler-agent".to_owned()))
        .expect("publication");
    let replay_receipt = DistillationVerifier
        .verify_peer(
            TeacherPacketCompiler
                .compile(&packet, InventionLimits::default())
                .expect("peer compile"),
            AgentId("replay-agent".to_owned()),
        )
        .expect("peer replay receipt");
    registry
        .peer_verify(id, replay_receipt)
        .expect("first independent peer");
    let falsifier_receipt = DistillationVerifier
        .verify_peer(
            TeacherPacketCompiler
                .compile(&packet, InventionLimits::default())
                .expect("falsifier compile"),
            AgentId("falsifier-agent".to_owned()),
        )
        .expect("falsifier replay receipt");
    registry
        .peer_verify(id, falsifier_receipt)
        .expect("second independent peer");
    registry.record_transfer(id, 2).expect("transfer evidence");
    let accepted = registry.accepted(id).expect("swarm accepted");
    assert_eq!(accepted.trust, DistillationTrust::SwarmAccepted);

    // The raw packet and teacher compiler are no longer consulted from here.
    let mut chatbot = TechnicalChatbot::new();
    for lesson in accepted.packet().language_lessons() {
        chatbot.install_lesson(lesson.clone());
    }
    let response = chatbot
        .respond("Could you make a Python function to sum numbers?")
        .expect("teacher-independent response");
    assert!(response.rendered.contains("def add_numbers"));
    assert!(response.rendered.contains("return a + b"));
}

#[test]
fn author_cannot_supply_its_own_peer_verification() {
    let packet = TeacherPacketCompiler
        .parse(include_bytes!("fixtures/basic_python.json"))
        .expect("packet");
    let verified = DistillationVerifier
        .verify(
            TeacherPacketCompiler
                .compile(&packet, InventionLimits::default())
                .expect("compile"),
        )
        .expect("verify");
    let mut registry = SharedKnowledgeRegistry::new();
    let author = AgentId("same-agent".to_owned());
    let id = registry.publish(verified, author.clone()).expect("publish");
    let self_receipt = DistillationVerifier
        .verify_peer(
            TeacherPacketCompiler
                .compile(&packet, InventionLimits::default())
                .expect("compile self receipt"),
            author,
        )
        .expect("self replay");
    assert!(registry.peer_verify(id, self_receipt).is_err());
    assert!(registry.accepted(id).is_none());
}

#[test]
fn packet_identity_covers_executable_evidence() {
    let packet = TeacherPacketCompiler
        .parse(include_bytes!("fixtures/basic_python.json"))
        .expect("packet");
    let mut changed = packet.clone();
    changed.mechanism_lessons[0].examples[0].output = 99;
    let original = TeacherPacketCompiler
        .compile(&packet, InventionLimits::default())
        .expect("original");
    let changed = TeacherPacketCompiler
        .compile(&changed, InventionLimits::default())
        .expect("changed");
    assert_ne!(original.id(), changed.id());
}

#[test]
fn rejects_forward_procedure_dependency_and_failed_counterexample() {
    let packet = TeacherPacketCompiler
        .parse(include_bytes!("fixtures/basic_python.json"))
        .expect("packet");
    let mut invalid_graph = packet.clone();
    invalid_graph.procedures[0].steps.swap(0, 1);
    assert!(TeacherPacketCompiler
        .compile(&invalid_graph, InventionLimits::default())
        .is_err());

    let mut failed_negative = packet;
    failed_negative.counterexamples[0].forbidden_output = 5;
    let compiled = TeacherPacketCompiler
        .compile(&failed_negative, InventionLimits::default())
        .expect("structurally valid negative");
    assert!(DistillationVerifier.verify(compiled).is_err());
}
