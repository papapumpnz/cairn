use cairn_core::{
    CanonicalValue, Hash32, ModalityId, Observation, ObservationCertainty, SourceReference,
};
use cairn_knowledge::{
    ClaimDraft, ClaimPredicate, ClaimTerm, EntityId, EntityMergeProposal, KnowledgeBase,
    RelationKind, ValidityInterval,
};
use std::collections::BTreeSet;

#[test]
fn source_bound_claims_retain_temporal_contradictions_and_executable_schemas() {
    let source_a = observation("source-a", "document-a");
    let source_b = observation("source-b", "document-b");
    let mut knowledge = KnowledgeBase::new();
    assert!(knowledge.observe(&source_a).expect("source a"));
    assert!(knowledge.observe(&source_b).expect("source b"));
    let entity = EntityId(Hash32::digest(b"reactor-7"));
    let status = ClaimPredicate {
        kind: RelationKind::Attribute,
        name: "operating_mode".to_owned(),
    };
    let first = knowledge
        .insert(draft(
            entity,
            status.clone(),
            ClaimTerm::Text("nominal".to_owned()),
            ValidityInterval {
                start: Some(10),
                end: Some(20),
            },
            source_a.id,
        ))
        .expect("first claim");
    assert!(first.contradictions.is_empty());
    let second = knowledge
        .insert(draft(
            entity,
            status.clone(),
            ClaimTerm::Text("maintenance".to_owned()),
            ValidityInterval {
                start: Some(15),
                end: Some(25),
            },
            source_b.id,
        ))
        .expect("contradictory claim");
    assert_eq!(second.contradictions, vec![first.claim]);
    assert_eq!(knowledge.claims_for(entity, &status, Some(17)).len(), 2);
    assert_eq!(knowledge.contradictions().count(), 1);

    let procedure = ClaimPredicate {
        kind: RelationKind::Procedure,
        name: "stabilization_schema".to_owned(),
    };
    let mechanism = Hash32::digest(b"verified-mechanism");
    knowledge
        .insert(draft(
            entity,
            procedure.clone(),
            ClaimTerm::Mechanism(mechanism),
            ValidityInterval {
                start: None,
                end: None,
            },
            source_a.id,
        ))
        .expect("executable schema claim");
    assert_eq!(
        knowledge.claims_for(entity, &procedure, None)[0].object,
        ClaimTerm::Mechanism(mechanism)
    );

    let merge = knowledge
        .propose_merge(EntityMergeProposal {
            left: entity,
            right: EntityId(Hash32::digest(b"reactor-seven")),
            confidence_ppm: 900_000,
            evidence: BTreeSet::from([source_a.id, source_b.id]),
        })
        .expect("reversible merge proposal");
    assert_ne!(merge.left, merge.right);
    assert_eq!(knowledge.len(), 3);
}

fn draft(
    subject: EntityId,
    predicate: ClaimPredicate,
    object: ClaimTerm,
    validity: ValidityInterval,
    source: cairn_core::ObservationId,
) -> ClaimDraft {
    ClaimDraft {
        subject,
        predicate,
        object,
        validity,
        confidence_ppm: 950_000,
        sources: BTreeSet::from([source]),
    }
}

fn observation(locator: &str, text: &str) -> Observation {
    Observation::new(
        ModalityId("urn:cairn:structured-document:v1".to_owned()),
        None,
        SourceReference {
            locator: locator.to_owned(),
            content_hash: Hash32::digest(text.as_bytes()),
        },
        CanonicalValue::Text(text.to_owned()),
        ObservationCertainty::Exact,
    )
    .expect("source observation")
}
