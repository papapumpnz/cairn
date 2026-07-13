use cairn_core::{EntityId, Hash32};
use cairn_invent::InventionLimits;
use cairn_memory::ObservationPredicateStore;
use cairn_rkl::ExecLimits;
use cairn_runtime::{CognitionError, CognitionRuntime, ObservationPredicateMemoryRuntimeError};
use cairn_theory::{
    ObservationPredicateInventor, ObservationPredicateLimits, PredicateGroupingLimits,
    PrimitivePairEntity, PrimitivePairTrace,
};
use cairn_verifier::{ObservationPredicatePromotionConsolidator, ObservationPredicateVerifier};
use std::collections::BTreeSet;

#[test]
fn cognition_runtime_reuses_synthesized_features_only_as_verified_proposals() {
    let observation_limits = ObservationPredicateLimits::default();
    let predicate_limits = PredicateGroupingLimits::default();
    let cohorts = [
        [
            episode("runtime-feature-a1", &[2, 3, 4], 100, 1_000),
            episode("runtime-feature-a2", &[2, 3, 4, 5], 300, 2_000),
        ],
        [
            episode("runtime-feature-b1", &[2, 4, 5], -500, 3_000),
            episode("runtime-feature-b2", &[3, 4, 5, 6], 700, 4_000),
        ],
    ];
    let learned = cohorts
        .iter()
        .map(|cohort| {
            ObservationPredicateInventor
                .invent(
                    cohort,
                    observation_limits,
                    predicate_limits,
                    InventionLimits::default(),
                )
                .expect("independent invention")
        })
        .collect::<Vec<_>>();
    assert_eq!(learned[0].semantic_hash(), learned[1].semantic_hash());
    let receipts = cohorts
        .iter()
        .zip(&learned)
        .map(|(cohort, mechanism)| {
            ObservationPredicateVerifier
                .verify_reuse(
                    mechanism,
                    cohort,
                    observation_limits,
                    predicate_limits,
                    ExecLimits::default(),
                )
                .expect("independent receipt")
        })
        .collect::<Vec<_>>();
    let promotion = ObservationPredicatePromotionConsolidator
        .consolidate(&learned[0], &receipts)
        .expect("promotion certificate");
    let mut store = ObservationPredicateStore::new();
    let id = store
        .insert_provisional(learned[0].clone())
        .expect("provisional mechanism");
    for receipt in receipts {
        store
            .record_verified_reuse(id, receipt)
            .expect("disjoint receipt");
    }
    assert!(store.apply_verified_promotion(id, &promotion));

    let held_out = episode("runtime-feature-vector", &[2, 3, 4, 5, 6], -900, 5_000);
    assert_eq!(
        CognitionRuntime::new().propose_observation_predicates_from_memory(
            &held_out.entities,
            observation_limits,
            predicate_limits,
        ),
        Err(CognitionError::ObservationPredicateMemory(
            ObservationPredicateMemoryRuntimeError::NoVerifiedProposal
        ))
    );
    let runtime = CognitionRuntime::with_observation_predicate_memory(store);
    let outcome = runtime
        .propose_observation_predicates_from_memory(
            &held_out.entities,
            observation_limits,
            predicate_limits,
        )
        .expect("runtime proposal");
    assert_eq!(outcome.persistent_mechanisms_attempted, 1);
    assert_eq!(outcome.proposals, vec![held_out.outputs.clone()]);

    let second = episode("runtime-feature-table", &[3, 4, 5, 6, 7], 1_100, 6_000);
    ObservationPredicateVerifier
        .verify(
            runtime
                .observation_predicate_memory()
                .persistent_mechanisms()
                .next()
                .expect("persistent proposal"),
            &[held_out, second],
            observation_limits,
            predicate_limits,
            ExecLimits::default(),
        )
        .expect("independent current-evidence acceptance");
}

fn episode(
    domain: &str,
    cardinalities: &[usize],
    sum_offset: i64,
    value_base: i64,
) -> PrimitivePairTrace {
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
    PrimitivePairTrace {
        episode: Hash32::digest(domain.as_bytes()),
        entities,
        outputs,
    }
}
