use cairn_core::{EntityId, Hash32};
use cairn_invent::InventionLimits;
use cairn_memory::ObservationPredicateStore;
use cairn_rkl::ExecLimits;
use cairn_runtime::{CognitionError, CognitionRuntime, RawRelationObservationMemoryRuntimeError};
use cairn_theory::{
    EntityObservationLimits, ObservationPredicateLimits, PredicateGroupingLimits,
    RawAttributedObservationPrimitive, RawRelationObservationInventor, RawRelationObservationTrace,
    RelationAttributeBinding, RelationKeyOperator, RelationObservationLimits,
};
use cairn_verifier::{
    ObservationPredicatePromotionConsolidator, ObservationPredicateVerifier,
    RawRelationObservationVerifier,
};
use std::collections::{BTreeMap, BTreeSet};

#[test]
#[allow(clippy::too_many_lines)] // Covers learning, promotion, raw rebinding, and acceptance.
fn runtime_reuses_mechanisms_while_rebinding_relations_and_entities() {
    let relation_limits = RelationObservationLimits::default();
    let entity_limits = EntityObservationLimits::default();
    let observation_limits = ObservationPredicateLimits::default();
    let predicate_limits = PredicateGroupingLimits::default();
    let cohorts = [
        [
            episode("raw-memory-sequence-a", &[2, 3, 4], 100, 0),
            episode("raw-memory-table-a", &[2, 3, 4, 5], 300, 2),
        ],
        [
            episode("raw-memory-vector-b", &[3, 4, 5], -500, 1),
            episode("raw-memory-world-b", &[2, 3, 5, 6], 700, 0),
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
                .expect("independent raw-relation invention")
        })
        .collect::<Vec<_>>();
    assert_eq!(learned[0].mechanism_hash(), learned[1].mechanism_hash());
    let lowered = cohorts
        .iter()
        .zip(&learned)
        .map(|(cohort, mechanism)| {
            cohort
                .iter()
                .map(|trace| {
                    RawRelationObservationInventor
                        .lower(
                            trace,
                            &mechanism.selected_bindings[&trace.episode],
                            relation_limits,
                            entity_limits,
                            observation_limits,
                        )
                        .expect("selected raw relation lowering")
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let receipts = learned
        .iter()
        .zip(&lowered)
        .map(|(mechanism, traces)| {
            ObservationPredicateVerifier
                .verify_reuse(
                    mechanism.mechanism(),
                    traces,
                    observation_limits,
                    predicate_limits,
                    ExecLimits::default(),
                )
                .expect("independent reuse receipt")
        })
        .collect::<Vec<_>>();
    let fresh_feature_work = receipts[0].candidate_executions_without();
    let promotion = ObservationPredicatePromotionConsolidator
        .consolidate(learned[0].mechanism(), &receipts)
        .expect("promotion certificate");
    let mut store = ObservationPredicateStore::new();
    let id = store
        .insert_provisional(learned[0].mechanism().clone())
        .expect("provisional mechanism");
    for receipt in receipts {
        store
            .record_verified_reuse(id, receipt)
            .expect("disjoint verified reuse");
    }
    assert!(store.apply_verified_promotion(id, &promotion));

    let held_out = [
        episode("raw-memory-graphics", &[2, 3, 4, 5], 1_100, 2),
        episode("raw-memory-control", &[2, 3, 4, 5, 6], -900, 1),
    ];
    assert_eq!(
        CognitionRuntime::new().propose_raw_relation_observations_from_memory(
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
    let runtime = CognitionRuntime::with_observation_predicate_memory(store);
    let expected_bindings = [projection(2), projection(1)];
    let mut selected_bindings = BTreeMap::new();
    for (trace, expected_binding) in held_out.iter().zip(expected_bindings) {
        let outcome = runtime
            .propose_raw_relation_observations_from_memory(
                trace,
                relation_limits,
                entity_limits,
                observation_limits,
                predicate_limits,
            )
            .expect("raw relation memory proposals");
        assert_eq!(outcome.persistent_mechanisms_considered, 1);
        assert_eq!(outcome.relation_bindings_attempted, 15);
        let accepted_candidate = outcome
            .proposals
            .iter()
            .find(|proposal| proposal.binding == expected_binding)
            .expect("explanatory attribute binding proposal");
        assert_eq!(accepted_candidate.output, trace.outputs);
        selected_bindings.insert(trace.episode, expected_binding);
    }
    let mut rebound = learned[0].clone();
    rebound.selected_bindings = selected_bindings;
    rebound.entity_grouping.mechanism = runtime
        .observation_predicate_memory()
        .persistent_mechanisms()
        .next()
        .expect("persistent mechanism")
        .clone();
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
        .expect("independent current raw-evidence acceptance");
    assert_eq!(receipt.replayed_groups(), 9);
    eprintln!(
        "RAW_RELATION_MEMORY feature_candidates={fresh_feature_work}->0 source_examples=4->0 current_relation_bindings=15 supplied_relations=0 supplied_entities=0 heldout_groups=9 exact=9 blank_proposals=0"
    );
}

fn projection(attribute: usize) -> RelationAttributeBinding {
    RelationAttributeBinding {
        operator: RelationKeyOperator::Projection,
        attributes: vec![attribute],
    }
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
            let first = signature_offset * 3 + i64::try_from(component).expect("component") * 37;
            let second = signature_offset * -2 + i64::try_from(component).expect("component") * 19;
            for (member, observation) in [first, second, signature - first - second]
                .into_iter()
                .enumerate()
            {
                let id = EntityId(Hash32::digest(
                    format!("{domain}:{component}:{member}").as_bytes(),
                ));
                let correct = i64::try_from(component).expect("correct key");
                let distractor =
                    i64::try_from((component + member) % component_count).expect("distractor key");
                let unique = i64::try_from(component * 3 + member).expect("unique key") + 90_000;
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
