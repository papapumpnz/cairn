use cairn_core::{
    decode_canonical, ArtifactEnvelope, ArtifactError, ArtifactKind, BudgetRecord, CanonicalEncode,
    CanonicalError, CanonicalLimits, CanonicalValue, ProducerRecord, SchemaDecision,
    SchemaDescriptor, SchemaRef, SchemaRegistry, SemanticsVersion,
};
use std::collections::BTreeSet;

#[test]
fn rfc8949_golden_bytes_match() {
    let cases = [
        (CanonicalValue::Unsigned(0), "00"),
        (CanonicalValue::Unsigned(23), "17"),
        (CanonicalValue::Unsigned(24), "1818"),
        (CanonicalValue::Unsigned(1000), "1903e8"),
        (CanonicalValue::Negative(-1), "20"),
        (CanonicalValue::Negative(-10), "29"),
        (CanonicalValue::Bytes(vec![1, 2, 3]), "43010203"),
        (CanonicalValue::Text("a".to_owned()), "6161"),
        (
            CanonicalValue::Array(vec![
                CanonicalValue::Unsigned(1),
                CanonicalValue::Unsigned(2),
            ]),
            "820102",
        ),
        (CanonicalValue::Float(1.5), "f93e00"),
    ];
    for (value, expected) in cases {
        assert_eq!(hex(&value.encode_canonical().expect("encode")), expected);
    }
}

#[test]
fn map_order_is_canonical_not_insertion_order() {
    let left = CanonicalValue::Map(vec![
        (
            CanonicalValue::Text("long".to_owned()),
            CanonicalValue::Unsigned(2),
        ),
        (
            CanonicalValue::Text("a".to_owned()),
            CanonicalValue::Unsigned(1),
        ),
    ]);
    let right = CanonicalValue::Map(vec![
        (
            CanonicalValue::Text("a".to_owned()),
            CanonicalValue::Unsigned(1),
        ),
        (
            CanonicalValue::Text("long".to_owned()),
            CanonicalValue::Unsigned(2),
        ),
    ]);
    assert_eq!(
        left.encode_canonical().expect("left"),
        right.encode_canonical().expect("right")
    );
}

#[test]
fn duplicate_map_keys_fail_closed() {
    let value = CanonicalValue::Map(vec![
        (CanonicalValue::Unsigned(1), CanonicalValue::Null),
        (CanonicalValue::Unsigned(1), CanonicalValue::Bool(true)),
    ]);
    assert_eq!(
        value.encode_canonical(),
        Err(CanonicalError::DuplicateMapKey)
    );
}

#[test]
fn nonpreferred_integer_width_is_rejected() {
    assert_eq!(
        decode_canonical(&[0x18, 0x00], CanonicalLimits::default()),
        Err(CanonicalError::NonCanonical)
    );
}

#[test]
fn hostile_lengths_and_depth_stop_before_allocation() {
    let limits = CanonicalLimits {
        max_scalar_bytes: 16,
        max_depth: 2,
        ..CanonicalLimits::default()
    };
    assert_eq!(
        decode_canonical(&[0x5a, 0xff, 0xff, 0xff, 0xff], limits),
        Err(CanonicalError::ScalarLimit {
            actual: u64::from(u32::MAX),
            maximum: 16
        })
    );
    assert_eq!(
        decode_canonical(&[0x81, 0x81, 0x81, 0x00], limits),
        Err(CanonicalError::DepthLimit { maximum: 2 })
    );
}

#[test]
fn one_hundred_thousand_random_values_round_trip() {
    let mut state = 0x9e37_79b9_7f4a_7c15_u64;
    for _ in 0..100_000 {
        let value = random_value(&mut state, 0);
        let bytes = value.encode_canonical().expect("encode");
        let decoded = decode_canonical(&bytes, CanonicalLimits::default()).expect("decode");
        assert_eq!(decoded.encode_canonical().expect("reencode"), bytes);
    }
}

#[test]
fn semantic_identity_ignores_producer_and_resource_observation() {
    let first = envelope("worker-a", 10);
    let second = envelope("worker-b", 999);
    assert_eq!(first.artifact_id, second.artifact_id);
    assert_eq!(first.payload_hash, second.payload_hash);
    assert_ne!(
        first.encode_envelope().expect("first envelope"),
        second.encode_envelope().expect("second envelope")
    );
    first.validate().expect("first valid");
    second.validate().expect("second valid");
}

#[test]
fn schema_registry_only_allows_declared_additive_reads() {
    let mut registry = SchemaRegistry::new();
    registry
        .register(SchemaDescriptor {
            name: "theory.ir".to_owned(),
            current_version: 3,
            additive_read_versions: BTreeSet::from([2]),
            maximum_payload_bytes: 1024,
        })
        .expect("register");
    assert_eq!(
        registry.resolve(&SchemaRef::new("theory.ir", 3).expect("schema")),
        Ok(SchemaDecision::Current)
    );
    assert_eq!(
        registry.resolve(&SchemaRef::new("theory.ir", 2).expect("schema")),
        Ok(SchemaDecision::AdditiveCompatible)
    );
    assert!(registry
        .resolve(&SchemaRef::new("theory.ir", 1).expect("schema"))
        .is_err());
}

#[test]
fn complete_envelope_round_trips_through_strict_decoder() {
    let envelope = envelope("worker", 12);
    let bytes = envelope.encode_envelope().expect("encode envelope");
    let decoded = ArtifactEnvelope::decode_envelope(
        &bytes,
        &test_registry(1),
        &BTreeSet::from([SemanticsVersion::new(1, 0, 0)]),
        CanonicalLimits::default(),
    )
    .expect("decode envelope");
    assert_eq!(decoded, envelope);
    assert_eq!(decoded.encode_envelope().expect("reencode"), bytes);
}

#[test]
fn envelope_rejects_unknown_schema_and_semantics() {
    let envelope = envelope("worker", 12);
    let bytes = envelope.encode_envelope().expect("encode envelope");
    assert!(matches!(
        ArtifactEnvelope::decode_envelope(
            &bytes,
            &SchemaRegistry::new(),
            &BTreeSet::from([SemanticsVersion::new(1, 0, 0)]),
            CanonicalLimits::default(),
        ),
        Err(ArtifactError::UnsupportedSchema(_))
    ));
    assert!(matches!(
        ArtifactEnvelope::decode_envelope(
            &bytes,
            &test_registry(1),
            &BTreeSet::new(),
            CanonicalLimits::default(),
        ),
        Err(ArtifactError::UnsupportedSemantics(_))
    ));
}

#[test]
fn envelope_payload_tampering_is_detected_after_canonical_decode() {
    let envelope = envelope("worker", 12);
    let bytes = envelope.encode_envelope().expect("encode envelope");
    let mut outer = decode_canonical(&bytes, CanonicalLimits::default()).expect("decode outer");
    let CanonicalValue::Map(entries) = &mut outer else {
        panic!("envelope must be map");
    };
    let payload = entries
        .iter_mut()
        .find_map(|(key, value)| {
            (key == &CanonicalValue::Text("payload".to_owned())).then_some(value)
        })
        .expect("payload field");
    *payload = CanonicalValue::Bytes(
        CanonicalValue::Unsigned(99)
            .encode_canonical()
            .expect("payload"),
    );
    let tampered = outer.encode_canonical().expect("tampered envelope");
    assert!(matches!(
        ArtifactEnvelope::decode_envelope(
            &tampered,
            &test_registry(1),
            &BTreeSet::from([SemanticsVersion::new(1, 0, 0)]),
            CanonicalLimits::default(),
        ),
        Err(ArtifactError::PayloadHashMismatch { .. })
    ));
}

#[test]
fn additive_reader_preserves_unknown_fields_and_requires_declared_ones() {
    let mut registry = SchemaRegistry::new();
    registry
        .register(SchemaDescriptor {
            name: "test.value".to_owned(),
            current_version: 2,
            additive_read_versions: BTreeSet::from([1]),
            maximum_payload_bytes: 1024,
        })
        .expect("registry");
    let schema = SchemaRef::new("test.value", 1).expect("schema");
    let payload = CanonicalValue::Map(vec![
        (
            CanonicalValue::Text("required".to_owned()),
            CanonicalValue::Unsigned(1),
        ),
        (
            CanonicalValue::Text("future".to_owned()),
            CanonicalValue::Unsigned(2),
        ),
    ]);
    let (decision, reader) = registry
        .additive_reader(&schema, &payload)
        .expect("additive reader");
    assert_eq!(decision, SchemaDecision::AdditiveCompatible);
    assert_eq!(
        reader.required("required"),
        Ok(&CanonicalValue::Unsigned(1))
    );
    assert_eq!(reader.unknown_fields(&["required"]), vec!["future"]);
    assert!(matches!(
        reader.required("absent"),
        Err(ArtifactError::MissingField("absent"))
    ));
}

fn envelope(component: &str, work_units: u64) -> ArtifactEnvelope<CanonicalValue> {
    ArtifactEnvelope::new(
        SchemaRef::new("test.value", 1).expect("schema"),
        ArtifactKind::new("test.value").expect("kind"),
        SemanticsVersion::new(1, 0, 0),
        Vec::new(),
        None,
        ProducerRecord {
            component: component.to_owned(),
            version: "0.1.0".to_owned(),
            commit: "abcdef0".to_owned(),
            host_run_id: "run".to_owned(),
            agent_id: None,
        },
        BudgetRecord {
            work_units,
            cpu_nanos: work_units,
            peak_rss_bytes: 1024,
        },
        work_units,
        CanonicalValue::Map(vec![(
            CanonicalValue::Text("answer".to_owned()),
            CanonicalValue::Unsigned(42),
        )]),
    )
    .expect("envelope")
}

fn test_registry(version: u32) -> SchemaRegistry {
    let mut registry = SchemaRegistry::new();
    registry
        .register(SchemaDescriptor {
            name: "test.value".to_owned(),
            current_version: version,
            additive_read_versions: BTreeSet::new(),
            maximum_payload_bytes: 1024,
        })
        .expect("register test schema");
    registry
}

fn random_value(state: &mut u64, depth: usize) -> CanonicalValue {
    let choice = next(state) % if depth < 3 { 9 } else { 7 };
    match choice {
        0 => CanonicalValue::Null,
        1 => CanonicalValue::Bool(next(state) & 1 == 1),
        2 => CanonicalValue::Unsigned(next(state)),
        3 => {
            let magnitude = i64::try_from(next(state) & i64::MAX as u64).expect("bounded");
            CanonicalValue::Negative(-1 - magnitude)
        }
        4 => CanonicalValue::Float(f64::from_bits(next(state) & 0x7fef_ffff_ffff_ffff)),
        5 => {
            let bytes = next(state).to_be_bytes();
            let length = usize::from(bytes[0] & 7);
            CanonicalValue::Bytes(bytes[..length].to_vec())
        }
        6 => CanonicalValue::Text(format!("v{:x}", next(state))),
        7 => CanonicalValue::Array(
            (0..next(state) % 4)
                .map(|_| random_value(state, depth + 1))
                .collect(),
        ),
        _ => {
            let count = next(state) % 4;
            CanonicalValue::Map(
                (0..count)
                    .map(|index| {
                        (
                            CanonicalValue::Text(format!("k{index}")),
                            random_value(state, depth + 1),
                        )
                    })
                    .collect(),
            )
        }
    }
}

fn next(state: &mut u64) -> u64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::new(), |mut output, byte| {
        write!(output, "{byte:02x}").expect("write to string");
        output
    })
}
