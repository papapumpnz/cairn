use cairn_core::{EntityId, Hash32};
use cairn_invent::InventionLimits;
use cairn_rkl::ExecLimits;
use cairn_theory::{
    EntityObservationLimits, ObservationPredicateLimits, PredicateGroupingLimits,
    RawAttributedObservationPrimitive, RawRelationObservationInventor, RawRelationObservationTrace,
    RelationAttributeBinding, RelationFoldMask, RelationKeyOperator, RelationObservationError,
    RelationObservationLimits,
};
use cairn_verifier::{RawRelationObservationVerifier, RawRelationObservationVerifyError};
use std::collections::{BTreeMap, BTreeSet};

#[test]
#[allow(clippy::too_many_lines)] // One causal test covers invention, replay, transfer, and ablation.
fn invents_relations_and_entities_from_anonymous_attributes() {
    let relation_limits = RelationObservationLimits::default();
    let entity_limits = EntityObservationLimits::default();
    let observation_limits = ObservationPredicateLimits::default();
    let predicate_limits = PredicateGroupingLimits::default();
    let source = [
        episode("raw-rel-sequence-a", &[2, 3, 4], 100, 0),
        episode("raw-rel-table-a", &[2, 3, 4, 5], 300, 2),
    ];
    let learned = RawRelationObservationInventor
        .invent(
            &source,
            relation_limits,
            entity_limits,
            observation_limits,
            predicate_limits,
            InventionLimits::default(),
        )
        .expect("relation, entity, and mechanism invention");
    assert_eq!(learned.selected_bindings[&source[0].episode], projection(0));
    assert_eq!(learned.selected_bindings[&source[1].episode], projection(2));

    let independently_renamed = [
        episode("raw-rel-vector-b", &[3, 4, 5], -500, 1),
        episode("raw-rel-world-b", &[2, 3, 5, 6], 700, 0),
    ];
    let corroborated = RawRelationObservationInventor
        .invent(
            &independently_renamed,
            relation_limits,
            entity_limits,
            observation_limits,
            predicate_limits,
            InventionLimits::default(),
        )
        .expect("independent renamed invention");
    assert_eq!(learned.mechanism_hash(), corroborated.mechanism_hash());
    assert_ne!(learned.selected_bindings, corroborated.selected_bindings);

    let held_out = [
        episode("raw-rel-graphics-heldout", &[2, 3, 4, 5], 1_100, 2),
        episode("raw-rel-control-heldout", &[2, 3, 4, 5, 6], -900, 1),
    ];
    let rebound = RawRelationObservationInventor
        .invent(
            &held_out,
            relation_limits,
            entity_limits,
            observation_limits,
            predicate_limits,
            InventionLimits::default(),
        )
        .expect("held-out relation rebinding");
    assert_eq!(learned.mechanism_hash(), rebound.mechanism_hash());
    let receipt = RawRelationObservationVerifier
        .verify(
            &rebound,
            &held_out,
            relation_limits,
            entity_limits,
            observation_limits,
            predicate_limits,
            ExecLimits::default(),
        )
        .expect("independent RKL relation and entity replay");
    assert_eq!(receipt.replayed_groups(), 9);

    let mut corrupted = source.clone();
    corrupted[0]
        .primitives
        .values_mut()
        .next()
        .expect("primitive")
        .attributes[0] = i64::MAX;
    assert_eq!(
        RawRelationObservationVerifier
            .verify(
                &learned,
                &corrupted,
                relation_limits,
                entity_limits,
                observation_limits,
                predicate_limits,
                ExecLimits::default(),
            )
            .expect_err("independent attribute bound"),
        RawRelationObservationVerifyError::InvalidEvidence
    );
    for trace in &held_out {
        let binding = &rebound.selected_bindings[&trace.episode];
        assert_eq!(
            RawRelationObservationInventor
                .predict(
                    learned.mechanism(),
                    trace,
                    binding,
                    relation_limits,
                    entity_limits,
                    observation_limits,
                    predicate_limits,
                )
                .expect("source-independent proposal"),
            trace.outputs
        );
    }
    let mut ablated = rebound.clone();
    for trace in &held_out {
        let correct = ablated.selected_bindings[&trace.episode].attributes[0];
        ablated
            .selected_bindings
            .insert(trace.episode, projection((correct + 1) % 3));
    }
    assert!(RawRelationObservationVerifier
        .verify(
            &ablated,
            &held_out,
            relation_limits,
            entity_limits,
            observation_limits,
            predicate_limits,
            ExecLimits::default(),
        )
        .is_err());
    eprintln!(
        "RAW_RELATION supplied_relations=0 supplied_entities=0 relation_programs=1 anonymous_attributes=3 source_domains=2 heldout_domains=2 heldout_groups=9 exact=9 binding_ablation_accepts=0"
    );
}

fn projection(attribute: usize) -> RelationAttributeBinding {
    RelationAttributeBinding {
        operator: RelationKeyOperator::Projection,
        attributes: vec![attribute],
    }
}

#[test]
fn raw_relation_ambiguity_and_capacity_fail_closed() {
    let source = [
        episode("raw-rel-negative-a", &[2, 3, 4], 100, 0),
        episode("raw-rel-negative-b", &[2, 3, 4, 5], 300, 2),
    ];
    assert_eq!(
        RawRelationObservationInventor
            .invent(
                &source,
                RelationObservationLimits {
                    maximum_relation_evaluations: 1,
                    ..RelationObservationLimits::default()
                },
                EntityObservationLimits::default(),
                ObservationPredicateLimits::default(),
                PredicateGroupingLimits::default(),
                InventionLimits::default(),
            )
            .expect_err("relation evaluation cap"),
        RelationObservationError::Capacity
    );
    let mut ambiguous = source.clone();
    for trace in &mut ambiguous {
        let correct = if trace.episode == source[0].episode {
            0
        } else {
            2
        };
        for primitive in trace.primitives.values_mut() {
            primitive.attributes.push(primitive.attributes[correct]);
        }
    }
    assert!(matches!(
        RawRelationObservationInventor.invent(
            &ambiguous,
            RelationObservationLimits::default(),
            EntityObservationLimits::default(),
            ObservationPredicateLimits::default(),
            PredicateGroupingLimits::default(),
            InventionLimits::default(),
        ),
        Err(RelationObservationError::Entity(
            cairn_theory::EntityObservationError::Ambiguous
        ))
    ));
}

#[test]
#[allow(clippy::too_many_lines)] // Composed-key invention includes transfer, replay, and ablation.
fn invents_a_composed_relation_key_when_every_atomic_key_fails() {
    let relation_limits = RelationObservationLimits::default();
    let entity_limits = EntityObservationLimits::default();
    let observation_limits = ObservationPredicateLimits::default();
    let predicate_limits = PredicateGroupingLimits::default();
    let source = [
        composed_episode("composed-sequence-a", &[2, 3, 4], 100, [0, 1, 2]),
        composed_episode("composed-table-a", &[2, 3, 4, 5], 300, [2, 0, 1]),
    ];
    let learned = RawRelationObservationInventor
        .invent(
            &source,
            relation_limits,
            entity_limits,
            observation_limits,
            predicate_limits,
            InventionLimits::default(),
        )
        .expect("composed relation-key invention");
    assert!(learned
        .selected_bindings
        .values()
        .all(|binding| binding.operator == RelationKeyOperator::Add));
    assert_eq!(
        learned.selected_bindings[&source[0].episode].attributes,
        vec![0, 1]
    );
    assert_eq!(
        learned.selected_bindings[&source[1].episode].attributes,
        vec![0, 2]
    );

    let corroborating = [
        composed_episode("composed-vector-b", &[3, 4, 5], -500, [1, 2, 0]),
        composed_episode("composed-world-b", &[2, 3, 5, 6], 700, [2, 1, 0]),
    ];
    let corroborated = RawRelationObservationInventor
        .invent(
            &corroborating,
            relation_limits,
            entity_limits,
            observation_limits,
            predicate_limits,
            InventionLimits::default(),
        )
        .expect("renamed composed-key invention");
    assert_eq!(learned.mechanism_hash(), corroborated.mechanism_hash());

    let held_out = [
        composed_episode("composed-graphics-heldout", &[2, 3, 4, 5], 1_100, [1, 0, 2]),
        composed_episode(
            "composed-control-heldout",
            &[2, 3, 4, 5, 6],
            -900,
            [2, 0, 1],
        ),
    ];
    let rebound = RawRelationObservationInventor
        .invent(
            &held_out,
            relation_limits,
            entity_limits,
            observation_limits,
            predicate_limits,
            InventionLimits::default(),
        )
        .expect("held-out composed-key rebinding");
    assert_eq!(learned.mechanism_hash(), rebound.mechanism_hash());
    let receipt = RawRelationObservationVerifier
        .verify(
            &rebound,
            &held_out,
            relation_limits,
            entity_limits,
            observation_limits,
            predicate_limits,
            ExecLimits::default(),
        )
        .expect("independent composed relation execution");
    assert_eq!(receipt.replayed_groups(), 9);

    let mut ablated = rebound.clone();
    for trace in &held_out {
        ablated
            .selected_bindings
            .insert(trace.episode, projection(2));
    }
    assert!(RawRelationObservationVerifier
        .verify(
            &ablated,
            &held_out,
            relation_limits,
            entity_limits,
            observation_limits,
            predicate_limits,
            ExecLimits::default(),
        )
        .is_err());
    eprintln!(
        "COMPOSED_RELATION supplied_relations=0 supplied_entities=0 relation_structures=5 atomic_exact=0 selected=add anonymous_attributes=3 heldout_groups=9 exact=9 ablation_accepts=0"
    );
}

#[test]
fn invents_a_variable_arity_relation_fold_beyond_pairs() {
    let relation_limits = RelationObservationLimits {
        maximum_key_arity: 3,
        enabled_associative_folds: RelationFoldMask::ADD,
        ..RelationObservationLimits::default()
    };
    let entity_limits = EntityObservationLimits::default();
    let observation_limits = ObservationPredicateLimits::default();
    let predicate_limits = PredicateGroupingLimits::default();
    let source = [
        fold_episode("fold-sequence-a", &[2, 3, 4], 100, [0, 1, 2, 3]),
        fold_episode("fold-table-a", &[2, 3, 4, 5], 300, [3, 0, 2, 1]),
    ];
    let learned = RawRelationObservationInventor
        .invent(
            &source,
            relation_limits,
            entity_limits,
            observation_limits,
            predicate_limits,
            InventionLimits::default(),
        )
        .expect("variable-arity relation fold invention");
    assert!(learned.selected_bindings.values().all(|binding| {
        binding.operator == RelationKeyOperator::Sum && binding.attributes.len() == 3
    }));
    let held_out = [
        fold_episode("fold-vector-heldout", &[2, 3, 4, 5], -500, [1, 3, 0, 2]),
        fold_episode("fold-world-heldout", &[2, 3, 4, 5, 6], 700, [2, 0, 3, 1]),
    ];
    let rebound = RawRelationObservationInventor
        .invent(
            &held_out,
            relation_limits,
            entity_limits,
            observation_limits,
            predicate_limits,
            InventionLimits::default(),
        )
        .expect("held-out relation fold");
    assert_eq!(learned.mechanism_hash(), rebound.mechanism_hash());
    assert_eq!(
        rebound
            .source_independent()
            .expect("source-independent fold")
            .key_arity,
        3
    );
    let receipt = RawRelationObservationVerifier
        .verify(
            &rebound,
            &held_out,
            relation_limits,
            entity_limits,
            observation_limits,
            predicate_limits,
            ExecLimits::default(),
        )
        .expect("independent variable-arity fold replay");
    assert_eq!(receipt.replayed_groups(), 9);
    eprintln!(
        "VARIABLE_ARITY_RELATION maximum_key_arity=3 relation_structures=6 atomic_and_pair_exact=0 selected=sum3 heldout_groups=9 exact=9"
    );
}

fn episode(
    domain: &str,
    cardinalities: &[usize],
    signature_offset: i64,
    correct_axis: usize,
) -> RawRelationObservationTrace {
    let component_count = cardinalities.iter().sum::<usize>();
    let mut primitives = BTreeMap::new();
    let mut outputs = BTreeSet::new();
    let mut component = 0_usize;
    for (group, cardinality) in cardinalities.iter().copied().enumerate() {
        let signature = signature_offset + i64::try_from(group).expect("group") * 10_000;
        outputs.insert(i64::try_from(cardinality - 1).expect("cardinality"));
        for _ in 0..cardinality {
            let first =
                signature_offset * 3 + i64::try_from(component).expect("component") * 37 + 11;
            let second =
                signature_offset * -2 + i64::try_from(component).expect("component") * 19 - 7;
            for (member, observation) in [first, second, signature - first - second]
                .into_iter()
                .enumerate()
            {
                let id = EntityId(Hash32::digest(
                    format!("{domain}:{component}:{member}").as_bytes(),
                ));
                let correct = i64::try_from(component).expect("component key");
                let distractor =
                    i64::try_from((component + member) % component_count).expect("distractor key");
                let unique = i64::try_from(component * 3 + member).expect("unique key") + 50_000;
                let mut attributes = vec![distractor, unique, distractor];
                attributes[correct_axis] = correct;
                primitives.insert(
                    id,
                    RawAttributedObservationPrimitive {
                        observation,
                        attributes,
                    },
                );
            }
            component += 1;
        }
    }
    RawRelationObservationTrace {
        episode: Hash32::digest(domain.as_bytes()),
        primitives,
        outputs,
    }
}

fn composed_episode(
    domain: &str,
    cardinalities: &[usize],
    signature_offset: i64,
    permutation: [usize; 3],
) -> RawRelationObservationTrace {
    let component_count = cardinalities.iter().sum::<usize>();
    let mut primitives = BTreeMap::new();
    let mut outputs = BTreeSet::new();
    let mut component = 0_usize;
    for (group, cardinality) in cardinalities.iter().copied().enumerate() {
        let signature = signature_offset + i64::try_from(group).expect("group") * 10_000;
        outputs.insert(i64::try_from(cardinality - 1).expect("cardinality"));
        for _ in 0..cardinality {
            let first = signature_offset * 3 + i64::try_from(component).expect("component") * 37;
            let second = signature_offset * -2 + i64::try_from(component).expect("component") * 19;
            let key = signature_offset * 20
                + 10_000
                + i64::try_from(component).expect("component key") * 1_000;
            for (member, observation) in [first, second, signature - first - second]
                .into_iter()
                .enumerate()
            {
                let id = EntityId(Hash32::digest(
                    format!("{domain}:{component}:{member}").as_bytes(),
                ));
                let first_key = i64::try_from(component).expect("component") * 100
                    + i64::try_from(member).expect("member");
                let second_key = key - first_key;
                let distractor =
                    i64::try_from((component + member) % component_count).expect("distractor");
                let logical = [first_key, second_key, distractor];
                let mut attributes = vec![0_i64; 3];
                for (logical_index, destination) in permutation.into_iter().enumerate() {
                    attributes[destination] = logical[logical_index];
                }
                primitives.insert(
                    id,
                    RawAttributedObservationPrimitive {
                        observation,
                        attributes,
                    },
                );
            }
            component += 1;
        }
    }
    RawRelationObservationTrace {
        episode: Hash32::digest(domain.as_bytes()),
        primitives,
        outputs,
    }
}

fn fold_episode(
    domain: &str,
    cardinalities: &[usize],
    signature_offset: i64,
    permutation: [usize; 4],
) -> RawRelationObservationTrace {
    let component_count = cardinalities.iter().sum::<usize>();
    let mut primitives = BTreeMap::new();
    let mut outputs = BTreeSet::new();
    let mut component = 0_usize;
    for (group, cardinality) in cardinalities.iter().copied().enumerate() {
        let signature = signature_offset + i64::try_from(group).expect("group") * 10_000;
        outputs.insert(i64::try_from(cardinality - 1).expect("cardinality"));
        for _ in 0..cardinality {
            let first = signature_offset * 3 + i64::try_from(component).expect("component") * 37;
            let second = signature_offset * -2 + i64::try_from(component).expect("component") * 19;
            let key = signature_offset * 20_000
                + 1_000_000
                + i64::try_from(component).expect("component key") * 100_000;
            for (member, observation) in [first, second, signature - first - second]
                .into_iter()
                .enumerate()
            {
                let id = EntityId(Hash32::digest(
                    format!("{domain}:{component}:{member}").as_bytes(),
                ));
                let coordinate = i64::try_from(component * 10 + member + 1).expect("coordinate");
                let logical = [
                    coordinate,
                    coordinate * coordinate + 7,
                    key - coordinate - (coordinate * coordinate + 7),
                    i64::try_from((component * 7 + member * 13) % (component_count * 17 + 1))
                        .expect("distractor"),
                ];
                let mut attributes = vec![0_i64; 4];
                for (logical_index, destination) in permutation.into_iter().enumerate() {
                    attributes[destination] = logical[logical_index];
                }
                primitives.insert(
                    id,
                    RawAttributedObservationPrimitive {
                        observation,
                        attributes,
                    },
                );
            }
            component += 1;
        }
    }
    RawRelationObservationTrace {
        episode: Hash32::digest(domain.as_bytes()),
        primitives,
        outputs,
    }
}
