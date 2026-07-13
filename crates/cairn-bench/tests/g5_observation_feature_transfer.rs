use cairn_core::{EntityId, Hash32};
use cairn_invent::InventionLimits;
use cairn_rkl::ExecLimits;
use cairn_sequence::{SequenceAdapter, SequenceSpanObservation};
use cairn_theory::{
    ObservationPredicateInventor, ObservationPredicateLimits, PredicateGroupingLimits,
    PrimitivePairEntity, PrimitivePairTrace,
};
use cairn_vector::{VectorAdapter, VectorAxis, VectorSegmentObservation};
use cairn_verifier::ObservationPredicateVerifier;
use std::collections::BTreeSet;

#[test]
fn learned_observation_features_transfer_from_sequences_to_vectors() {
    let observation_limits = ObservationPredicateLimits::default();
    let predicate_limits = PredicateGroupingLimits {
        maximum_relation_candidates: 128,
        ..PredicateGroupingLimits::default()
    };
    let sequence = [
        sequence_episode("adapter-sequence-a", &[2, 3, 4], 0, 1_000),
        sequence_episode("adapter-sequence-b", &[2, 3, 4, 5], 10, 2_000),
    ];
    let learned = ObservationPredicateInventor
        .invent(
            &sequence,
            observation_limits,
            predicate_limits,
            InventionLimits::default(),
        )
        .expect("learn from sequence frontend");
    let vector = [
        vector_episode("adapter-vector-a", &[2, 3, 4, 5, 6], 40, 5_000),
        vector_episode("adapter-vector-b", &[2, 3, 4, 5, 6, 7], 50, 6_000),
    ];
    let receipt = ObservationPredicateVerifier
        .verify(
            &learned,
            &vector,
            observation_limits,
            predicate_limits,
            ExecLimits::default(),
        )
        .expect("independent vector replay");
    assert_eq!(receipt.replayed_groups(), 11);
    assert_eq!(
        ObservationPredicateInventor
            .predict(
                &learned,
                &vector[0].entities,
                observation_limits,
                predicate_limits,
            )
            .expect("cross-adapter prediction"),
        vector[0].outputs
    );
    eprintln!(
        "G5_OBSERVATION_FEATURE_TRANSFER source=sequence target=vector core_changes=0 supplied_features=0 heldout_groups=11 exact=11"
    );
}

#[test]
fn synthesized_addition_feature_transfers_from_sequences_to_vectors() {
    let observation_limits = ObservationPredicateLimits::default();
    let predicate_limits = PredicateGroupingLimits::default();
    let sequence = [
        sequence_sum_episode("sum-sequence-adapter-a", &[2, 3, 4], 100, 7_000),
        sequence_sum_episode("sum-sequence-adapter-b", &[2, 3, 4, 5], 300, 8_000),
    ];
    let learned = ObservationPredicateInventor
        .invent(
            &sequence,
            observation_limits,
            predicate_limits,
            InventionLimits::default(),
        )
        .expect("synthesize feature from sequence adapter");
    let vector = [
        vector_sum_episode("sum-vector-adapter-a", &[2, 3, 4, 5, 6], -700, 9_000),
        vector_sum_episode("sum-vector-adapter-b", &[2, 3, 4, 5, 6, 7], 900, 10_000),
    ];
    let receipt = ObservationPredicateVerifier
        .verify(
            &learned,
            &vector,
            observation_limits,
            predicate_limits,
            ExecLimits::default(),
        )
        .expect("independent cross-adapter feature replay");
    assert_eq!(receipt.replayed_groups(), 11);
    eprintln!(
        "G5_SYNTHESIZED_FEATURE_TRANSFER source=sequence target=vector named_templates=0 feature_candidates={} feature_sets={} heldout_groups=11 exact=11",
        learned.feature_candidates_evaluated, learned.feature_sets_evaluated
    );
}

fn sequence_episode(
    domain: &str,
    cardinalities: &[usize],
    coordinate_offset: i64,
    value_base: i64,
) -> PrimitivePairTrace {
    let (primitives, outputs) = primitives(domain, cardinalities, coordinate_offset, value_base);
    let observations = primitives
        .into_iter()
        .map(|entity| SequenceSpanObservation {
            id: entity.id,
            endpoints: (entity.observations[0], entity.observations[1]),
            value: entity.value,
        })
        .collect::<Vec<_>>();
    PrimitivePairTrace {
        episode: Hash32::digest(domain.as_bytes()),
        entities: SequenceAdapter.lower_span_observations(&observations),
        outputs,
    }
}

fn vector_episode(
    domain: &str,
    cardinalities: &[usize],
    coordinate_offset: i64,
    value_base: i64,
) -> PrimitivePairTrace {
    let (primitives, outputs) = primitives(domain, cardinalities, coordinate_offset, value_base);
    let observations = primitives
        .into_iter()
        .enumerate()
        .map(|(index, entity)| VectorSegmentObservation {
            id: entity.id,
            first: (
                i64::try_from(index).expect("bounded index") * 7,
                entity.observations[0],
            ),
            second: (
                i64::try_from(index).expect("bounded index") * 7 + 3,
                entity.observations[1],
            ),
            value: entity.value,
        })
        .collect::<Vec<_>>();
    PrimitivePairTrace {
        episode: Hash32::digest(domain.as_bytes()),
        entities: VectorAdapter.lower_segment_observations(&observations, VectorAxis::Horizontal),
        outputs,
    }
}

fn sequence_sum_episode(
    domain: &str,
    cardinalities: &[usize],
    sum_offset: i64,
    value_base: i64,
) -> PrimitivePairTrace {
    let (primitives, outputs) = sum_primitives(domain, cardinalities, sum_offset, value_base);
    let observations = primitives
        .into_iter()
        .map(|entity| SequenceSpanObservation {
            id: entity.id,
            endpoints: (entity.observations[0], entity.observations[1]),
            value: entity.value,
        })
        .collect::<Vec<_>>();
    PrimitivePairTrace {
        episode: Hash32::digest(domain.as_bytes()),
        entities: SequenceAdapter.lower_span_observations(&observations),
        outputs,
    }
}

fn vector_sum_episode(
    domain: &str,
    cardinalities: &[usize],
    sum_offset: i64,
    value_base: i64,
) -> PrimitivePairTrace {
    let (primitives, outputs) = sum_primitives(domain, cardinalities, sum_offset, value_base);
    let observations = primitives
        .into_iter()
        .enumerate()
        .map(|(index, entity)| VectorSegmentObservation {
            id: entity.id,
            first: (
                i64::try_from(index).expect("bounded index") * 11,
                entity.observations[0],
            ),
            second: (
                i64::try_from(index).expect("bounded index") * 11 + 5,
                entity.observations[1],
            ),
            value: entity.value,
        })
        .collect::<Vec<_>>();
    PrimitivePairTrace {
        episode: Hash32::digest(domain.as_bytes()),
        entities: VectorAdapter.lower_segment_observations(&observations, VectorAxis::Horizontal),
        outputs,
    }
}

fn primitives(
    domain: &str,
    cardinalities: &[usize],
    coordinate_offset: i64,
    value_base: i64,
) -> (Vec<PrimitivePairEntity>, BTreeSet<i64>) {
    let mut entities = Vec::new();
    let mut outputs = BTreeSet::new();
    for (group_index, cardinality) in cardinalities.iter().copied().enumerate() {
        let root = coordinate_offset * 10_000;
        let group_base = root + i64::try_from(group_index).expect("bounded group") * 1_000;
        outputs.insert(i64::try_from(cardinality - 1).expect("bounded cardinality"));
        for member in 0..cardinality {
            let member = i64::try_from(member).expect("bounded member");
            let (minimum, maximum) = if group_index == 0 && member == 0 {
                (root, root + 100)
            } else if group_index == 0 {
                (root + 20 + member, root + 40 + member)
            } else if group_index == 1 && member == 0 {
                (root - 100, root + 15)
            } else if group_index == 1 {
                (root - 90 + member, root + 10 - member)
            } else if member == 0 {
                (group_base, group_base + 900)
            } else {
                (group_base + member * 20, group_base + 900 - member * 17)
            };
            let observations = if (group_index + usize::try_from(member).expect("member")) % 2 == 0
            {
                vec![maximum, minimum]
            } else {
                vec![minimum, maximum]
            };
            entities.push(PrimitivePairEntity {
                id: EntityId(Hash32::digest(
                    format!("{domain}:{group_index}:{member}").as_bytes(),
                )),
                value: value_base
                    + i64::try_from(group_index).expect("bounded group") * 100
                    + member,
                observations,
            });
        }
    }
    (entities, outputs)
}

fn sum_primitives(
    domain: &str,
    cardinalities: &[usize],
    sum_offset: i64,
    value_base: i64,
) -> (Vec<PrimitivePairEntity>, BTreeSet<i64>) {
    let mut entities = Vec::new();
    let mut outputs = BTreeSet::new();
    for (group_index, cardinality) in cardinalities.iter().copied().enumerate() {
        let sum = sum_offset + i64::try_from(group_index).expect("bounded group") * 10_000;
        outputs.insert(i64::try_from(cardinality - 1).expect("bounded cardinality"));
        for member in 0..cardinality {
            let member = i64::try_from(member).expect("bounded member");
            let first = sum_offset * 3 + member * 37 + 11;
            let second = sum - first;
            let observations = if (group_index + usize::try_from(member).expect("member")) % 2 == 0
            {
                vec![second, first]
            } else {
                vec![first, second]
            };
            entities.push(PrimitivePairEntity {
                id: EntityId(Hash32::digest(
                    format!("{domain}:{group_index}:{member}").as_bytes(),
                )),
                value: value_base
                    + i64::try_from(group_index).expect("bounded group") * 100
                    + member,
                observations,
            });
        }
    }
    (entities, outputs)
}
