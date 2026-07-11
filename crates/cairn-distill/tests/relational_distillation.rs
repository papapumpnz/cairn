use cairn_distill::{DistillationVerifier, TeacherPacketCompiler};
use cairn_invent::InventionLimits;

#[test]
fn offline_ai_packet_invents_and_holds_out_a_relational_selector() {
    let packet = TeacherPacketCompiler
        .parse(include_bytes!("fixtures/relational_teacher.json"))
        .expect("packet");
    let verified = DistillationVerifier
        .verify(
            TeacherPacketCompiler
                .compile(&packet, InventionLimits::default())
                .expect("compile"),
        )
        .expect("verify");
    assert_eq!(verified.entity_selectors().len(), 1);
    assert_eq!(verified.origin().kind, "anthropic_offline_teacher_session");
}
