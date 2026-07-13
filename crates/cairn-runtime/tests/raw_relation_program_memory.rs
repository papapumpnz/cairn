use cairn_core::{EntityId, Hash32};
use cairn_invent::InventionLimits;
use cairn_memory::RawRelationStore;
use cairn_rkl::ExecLimits;
use cairn_runtime::{CognitionError, CognitionRuntime, RawRelationObservationMemoryRuntimeError};
use cairn_theory::{
    EntityObservationLimits, ObservationPredicateLimits, PredicateGroupingLimits,
    RawAttributedObservationPrimitive, RawRelationObservationInventor, RawRelationObservationTrace,
    RelationAttributeBinding, RelationFoldMask, RelationKeyOperator, RelationObservationLimits,
};
use cairn_verifier::{RawRelationObservationPromotionConsolidator, RawRelationObservationVerifier};
use std::collections::{BTreeMap, BTreeSet};

#[test]
#[allow(clippy::too_many_lines)] // Full relation learning, promotion, transfer, and acceptance boundary.
fn complete_relation_program_promotes_and_prunes_future_structure_search() {
    let relation_limits = RelationObservationLimits::default();
    let entity_limits = EntityObservationLimits::default();
    let observation_limits = ObservationPredicateLimits::default();
    let predicate_limits = PredicateGroupingLimits::default();
    let cohorts = [
        [
            episode("program-memory-sequence-a", &[2, 3, 4], 100, [0, 1, 2]),
            episode("program-memory-table-a", &[2, 3, 4, 5], 300, [2, 0, 1]),
        ],
        [
            episode("program-memory-vector-b", &[3, 4, 5], -500, [1, 2, 0]),
            episode("program-memory-world-b", &[2, 3, 5, 6], 700, [2, 1, 0]),
        ],
    ];
    let learned = cohorts
        .iter()
        .map(|cohort| {
            RawRelationObservationInventor
                .invent(
                    cohort,
                    relation_limits,
                    entity_limits,
                    observation_limits,
                    predicate_limits,
                    InventionLimits::default(),
                )
                .expect("independent composed relation invention")
        })
        .collect::<Vec<_>>();
    assert_eq!(learned[0].mechanism_hash(), learned[1].mechanism_hash());
    let receipts = learned
        .iter()
        .zip(&cohorts)
        .map(|(mechanism, cohort)| {
            RawRelationObservationVerifier
                .verify_reuse(
                    mechanism,
                    cohort,
                    relation_limits,
                    entity_limits,
                    observation_limits,
                    predicate_limits,
                    ExecLimits::default(),
                )
                .expect("independent relation reuse receipt")
        })
        .collect::<Vec<_>>();
    assert!(receipts
        .iter()
        .all(|receipt| receipt.bindings_without() == 30 && receipt.bindings_with() == 6));
    let promotion = RawRelationObservationPromotionConsolidator
        .consolidate(&receipts)
        .expect("complete relation promotion");
    assert_eq!(promotion.bindings_saved(), 48);
    let source_independent = learned[0]
        .source_independent()
        .expect("source-independent relation program");
    let mut store = RawRelationStore::new();
    let id = store
        .insert_provisional(source_independent)
        .expect("provisional complete relation program");
    for receipt in receipts {
        store
            .record_verified_reuse(id, receipt)
            .expect("disjoint relation receipt");
    }
    assert!(store.apply_verified_promotion(id, &promotion));

    let held_out = [
        episode("program-memory-graphics", &[2, 3, 4, 5], 1_100, [1, 0, 2]),
        episode("program-memory-control", &[2, 3, 4, 5, 6], -900, [2, 0, 1]),
    ];
    assert_eq!(
        CognitionRuntime::new().propose_raw_relation_programs_from_memory(
            &held_out[0],
            relation_limits,
            entity_limits,
            observation_limits,
            predicate_limits,
        ),
        Err(CognitionError::RawRelationObservationMemory(
            RawRelationObservationMemoryRuntimeError::NoVerifiedProposal
        ))
    );
    assert_eq!(
        CognitionRuntime::new().propose_raw_relation_programs_from_memory(
            &RawRelationObservationTrace {
                episode: Hash32::digest(b"empty-relation-evidence"),
                primitives: BTreeMap::new(),
                outputs: BTreeSet::new(),
            },
            relation_limits,
            entity_limits,
            observation_limits,
            predicate_limits,
        ),
        Err(CognitionError::RawRelationObservationMemory(
            RawRelationObservationMemoryRuntimeError::InvalidEvidence
        ))
    );
    let runtime = CognitionRuntime::with_raw_relation_memory(store);
    let expected = [add_binding(0, 1), add_binding(0, 2)];
    let mut selected_bindings = BTreeMap::new();
    for (trace, expected_binding) in held_out.iter().zip(expected) {
        let outcome = runtime
            .propose_raw_relation_programs_from_memory(
                trace,
                relation_limits,
                entity_limits,
                observation_limits,
                predicate_limits,
            )
            .expect("complete relation-program proposals");
        assert_eq!(outcome.persistent_mechanisms_considered, 1);
        assert_eq!(outcome.relation_bindings_attempted, 3);
        let proposal = outcome
            .proposals
            .iter()
            .find(|proposal| proposal.binding == expected_binding)
            .expect("correct composed binding proposal");
        assert_eq!(proposal.output, trace.outputs);
        selected_bindings.insert(trace.episode, expected_binding);
    }
    let mut rebound = learned[0].clone();
    rebound.selected_bindings = selected_bindings;
    let persistent = runtime
        .raw_relation_memory()
        .persistent_modules()
        .next()
        .expect("persistent complete relation")
        .mechanism();
    rebound.relation_definition = persistent.relation_definition.clone();
    rebound.entity_grouping.mechanism = persistent.observation_mechanism.clone();
    let verified = RawRelationObservationVerifier
        .verify(
            &rebound,
            &held_out,
            relation_limits,
            entity_limits,
            observation_limits,
            predicate_limits,
            ExecLimits::default(),
        )
        .expect("independent current-evidence acceptance");
    assert_eq!(verified.replayed_groups(), 9);
    eprintln!(
        "RELATION_PROGRAM_MEMORY structures_bindings=15->3 source_examples=4->0 promotion_receipts=2 bindings_saved=48 heldout_groups=9 exact=9 blank_proposals=0"
    );
}

#[test]
#[allow(clippy::too_many_lines)] // Covers full n-ary invention, promotion, rebinding, and replay.
fn variable_arity_relation_fold_is_promoted_and_rebound() {
    let relation_limits = RelationObservationLimits {
        maximum_key_arity: 3,
        enabled_associative_folds: RelationFoldMask::ADD,
        ..RelationObservationLimits::default()
    };
    let entity_limits = EntityObservationLimits::default();
    let observation_limits = ObservationPredicateLimits::default();
    let predicate_limits = PredicateGroupingLimits::default();
    let cohorts = [
        [
            fold_episode("fold-memory-sequence-a", &[2, 3, 4], 100, [0, 1, 2, 3]),
            fold_episode("fold-memory-table-a", &[2, 3, 4, 5], 300, [3, 0, 2, 1]),
        ],
        [
            fold_episode("fold-memory-vector-b", &[3, 4, 5], -500, [1, 3, 0, 2]),
            fold_episode("fold-memory-world-b", &[2, 3, 5, 6], 700, [2, 0, 3, 1]),
        ],
    ];
    let learned = cohorts
        .iter()
        .map(|cohort| {
            RawRelationObservationInventor
                .invent(
                    cohort,
                    relation_limits,
                    entity_limits,
                    observation_limits,
                    predicate_limits,
                    InventionLimits::default(),
                )
                .expect("independent relation-fold invention")
        })
        .collect::<Vec<_>>();
    let receipts = learned
        .iter()
        .zip(&cohorts)
        .map(|(mechanism, cohort)| {
            RawRelationObservationVerifier
                .verify_reuse(
                    mechanism,
                    cohort,
                    relation_limits,
                    entity_limits,
                    observation_limits,
                    predicate_limits,
                    ExecLimits::default(),
                )
                .expect("relation-fold reuse receipt")
        })
        .collect::<Vec<_>>();
    assert!(receipts.iter().all(|receipt| {
        receipt.bindings_without() == 64 && receipt.bindings_with() == 8 && receipt.key_arity() == 3
    }));
    let promotion = RawRelationObservationPromotionConsolidator
        .consolidate(&receipts)
        .expect("relation-fold promotion");
    assert_eq!(promotion.bindings_saved(), 112);
    assert_eq!(promotion.support().key_arity, 3);
    let mut store = RawRelationStore::new();
    let id = store
        .insert_provisional(
            learned[0]
                .source_independent()
                .expect("source-independent fold"),
        )
        .expect("provisional relation fold");
    for receipt in receipts {
        store
            .record_verified_reuse(id, receipt)
            .expect("relation-fold receipt");
    }
    assert!(store.apply_verified_promotion(id, &promotion));

    let held_out = [
        fold_episode("fold-memory-graphics", &[2, 3, 4, 5], 1_100, [1, 3, 0, 2]),
        fold_episode("fold-memory-control", &[2, 3, 4, 5, 6], -900, [2, 0, 3, 1]),
    ];
    let runtime = CognitionRuntime::with_raw_relation_memory(store);
    let mut selected_bindings = BTreeMap::new();
    for trace in &held_out {
        let outcome = runtime
            .propose_raw_relation_programs_from_memory(
                trace,
                relation_limits,
                entity_limits,
                observation_limits,
                predicate_limits,
            )
            .expect("variable-arity memory proposals");
        assert_eq!(outcome.relation_bindings_attempted, 4);
        let proposal = outcome
            .proposals
            .iter()
            .find(|proposal| proposal.output == trace.outputs)
            .expect("exact fold binding");
        assert_eq!(proposal.binding.operator, RelationKeyOperator::Sum);
        assert_eq!(proposal.binding.attributes.len(), 3);
        selected_bindings.insert(trace.episode, proposal.binding.clone());
    }
    let mut rebound = learned[0].clone();
    rebound.selected_bindings = selected_bindings;
    let persistent = runtime
        .raw_relation_memory()
        .persistent_modules()
        .next()
        .expect("persistent relation fold")
        .mechanism();
    rebound.relation_definition = persistent.relation_definition.clone();
    rebound.entity_grouping.mechanism = persistent.observation_mechanism.clone();
    assert_eq!(
        RawRelationObservationVerifier
            .verify(
                &rebound,
                &held_out,
                relation_limits,
                entity_limits,
                observation_limits,
                predicate_limits,
                ExecLimits::default(),
            )
            .expect("independent relation-fold acceptance")
            .replayed_groups(),
        9
    );
    eprintln!(
        "VARIABLE_ARITY_RELATION_MEMORY bindings=32->4 cohort_bindings=128->16 examples=4->0 promotion_receipts=2 bindings_saved=112 heldout_groups=9 exact=9"
    );
}

fn add_binding(first: usize, second: usize) -> RelationAttributeBinding {
    RelationAttributeBinding {
        operator: RelationKeyOperator::Add,
        attributes: vec![first.min(second), first.max(second)],
    }
}

fn episode(
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
                let logical = [
                    first_key,
                    key - first_key,
                    i64::try_from((component + member) % component_count).expect("distractor"),
                ];
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
