use cairn_core::{
    CanonicalValue, EvidenceElement, EvidenceElementId, EvidenceRelation, EvidenceValueType,
    Hash32, ModalityId, Observation, ObservationCertainty, ObservationError, RelationalObservation,
    RelationalObservationError, RelationalObservationLimits, SourceReference,
};

fn observation(payload: CanonicalValue) -> Observation {
    Observation::new(
        ModalityId("urn:cairn:test:points:v1".to_owned()),
        Some(7),
        SourceReference {
            locator: "generated:test/7".to_owned(),
            content_hash: Hash32::digest(b"source"),
        },
        payload,
        ObservationCertainty::Exact,
    )
    .expect("observation")
}

fn element(name: &str, value_type: EvidenceValueType, value: &str) -> EvidenceElement {
    EvidenceElement {
        id: EvidenceElementId(Hash32::digest(name.as_bytes())),
        value_type,
        value: Hash32::digest(value.as_bytes()),
    }
}

fn relational(
    elements: Vec<EvidenceElement>,
    relations: Vec<EvidenceRelation>,
) -> Result<RelationalObservation, RelationalObservationError> {
    RelationalObservation::new(
        ModalityId("urn:cairn:test:relations:v1".to_owned()),
        Some(9),
        SourceReference {
            locator: "generated:relations/9".to_owned(),
            content_hash: Hash32::digest(b"relational-source"),
        },
        ObservationCertainty::Exact,
        elements,
        relations,
        RelationalObservationLimits::default(),
    )
}

#[test]
fn observation_identity_is_schema_neutral_and_payload_bound() {
    let first = observation(CanonicalValue::Array(vec![CanonicalValue::Unsigned(1)]));
    let same = observation(CanonicalValue::Array(vec![CanonicalValue::Unsigned(1)]));
    let changed = observation(CanonicalValue::Array(vec![CanonicalValue::Unsigned(2)]));
    assert_eq!(first.id, same.id);
    assert_ne!(first.id, changed.id);
    first.verify().expect("identity");
}

#[test]
fn malformed_certainty_and_tampering_fail_closed() {
    let invalid = Observation::new(
        ModalityId("test".to_owned()),
        None,
        SourceReference {
            locator: "generated".to_owned(),
            content_hash: Hash32::digest(b"source"),
        },
        CanonicalValue::Null,
        ObservationCertainty::PartsPerMillion(1_000_001),
    );
    assert_eq!(invalid, Err(ObservationError::InvalidConfidence(1_000_001)));

    let mut tampered = observation(CanonicalValue::Bool(true));
    tampered.payload = CanonicalValue::Bool(false);
    assert_eq!(tampered.verify(), Err(ObservationError::IdentityMismatch));
}

#[test]
fn relational_observation_is_order_invariant_and_payload_bound() {
    let actor = element("actor", EvidenceValueType::Entity, "agent-a");
    let effect = element("effect", EvidenceValueType::Effect, "delta-a");
    let relation = EvidenceRelation {
        predicate: Hash32::digest(b"opaque-link"),
        participants: vec![actor.id, effect.id],
    };
    let first = relational(vec![actor, effect], vec![relation.clone()]).expect("first");
    let reordered = relational(vec![effect, actor], vec![relation]).expect("reordered");
    assert_eq!(first, reordered);
    first
        .verify(RelationalObservationLimits::default())
        .expect("verified");

    let mut tampered = first.clone();
    tampered.elements[0].value = Hash32::digest(b"changed");
    assert_eq!(
        tampered.verify(RelationalObservationLimits::default()),
        Err(RelationalObservationError::PayloadMismatch)
    );
}

#[test]
fn relational_observation_rejects_invalid_graphs_and_bounds() {
    let actor = element("actor", EvidenceValueType::Entity, "agent-a");
    assert_eq!(
        relational(vec![actor, actor], Vec::new()),
        Err(RelationalObservationError::DuplicateElement(actor.id))
    );
    assert_eq!(
        relational(
            vec![actor],
            vec![EvidenceRelation {
                predicate: Hash32::digest(b"empty"),
                participants: Vec::new(),
            }],
        ),
        Err(RelationalObservationError::EmptyRelation)
    );
    let unknown = EvidenceElementId(Hash32::digest(b"unknown"));
    assert_eq!(
        relational(
            vec![actor],
            vec![EvidenceRelation {
                predicate: Hash32::digest(b"link"),
                participants: vec![actor.id, unknown],
            }],
        ),
        Err(RelationalObservationError::UnknownParticipant(unknown))
    );
    let limited = RelationalObservation::new(
        ModalityId("urn:cairn:test:relations:v1".to_owned()),
        None,
        SourceReference {
            locator: "generated:limited".to_owned(),
            content_hash: Hash32::digest(b"limited"),
        },
        ObservationCertainty::Exact,
        vec![actor],
        Vec::new(),
        RelationalObservationLimits {
            maximum_elements: 0,
            ..RelationalObservationLimits::default()
        },
    );
    assert!(matches!(
        limited,
        Err(RelationalObservationError::Capacity {
            resource: "elements",
            ..
        })
    ));
}
