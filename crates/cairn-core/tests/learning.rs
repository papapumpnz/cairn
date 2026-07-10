use cairn_core::{
    CanonicalValue, Hash32, ModalityId, Observation, ObservationCertainty, ObservationError,
    SourceReference,
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
