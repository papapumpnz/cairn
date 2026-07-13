use cairn_core::{EntityId, Hash32};
use cairn_invent::InventionLimits;
use cairn_memory::{ModuleStatus, PredicateGroupingStore, PredicateGroupingStoreError};
use cairn_rkl::{ExecLimits, Expr};
use cairn_theory::{
    AttributedGroupingEntity, AttributedGroupingTrace, PredicateGroupingInventor,
    PredicateGroupingLimits,
};
use cairn_verifier::{
    PredicateGroupingPromotionConsolidator, PredicateGroupingPromotionError,
    PredicateGroupingVerifier,
};
use std::collections::BTreeSet;

#[test]
fn promotes_an_invented_conjunction_and_reuses_it_as_a_proposal() {
    let limits = PredicateGroupingLimits::default();
    let cohort_a = [
        episode("vector-memory-a", &[(0, 0), (0, 1), (1, 0)], 1_000),
        episode(
            "sequence-memory-a",
            &[(2, 2), (2, 3), (3, 2), (3, 3)],
            2_000,
        ),
    ];
    let cohort_b = [
        episode("table-memory-b", &[(10, 20), (10, 21), (11, 20)], 3_000),
        episode(
            "workflow-memory-b",
            &[(30, 40), (30, 41), (31, 40), (31, 41)],
            4_000,
        ),
    ];
    let learned_a = PredicateGroupingInventor
        .invent(&cohort_a, limits, InventionLimits::default())
        .expect("first learned conjunction");
    let learned_b = PredicateGroupingInventor
        .invent(&cohort_b, limits, InventionLimits::default())
        .expect("second learned conjunction");
    assert_eq!(learned_a.semantic_hash(), learned_b.semantic_hash());
    assert!(matches!(learned_a.relation.body, Expr::Binary { .. }));
    let fresh_candidates = u64::try_from(learned_a.relation_candidates_evaluated)
        .expect("bounded candidates")
        .saturating_add(learned_a.merge.constructor.proposal.candidates_evaluated);
    let receipt_a = PredicateGroupingVerifier
        .verify_reuse(
            &learned_a,
            &cohort_a,
            limits,
            ExecLimits::default(),
            fresh_candidates,
            1,
        )
        .expect("first reuse receipt");
    let receipt_b = PredicateGroupingVerifier
        .verify_reuse(
            &learned_b,
            &cohort_b,
            limits,
            ExecLimits::default(),
            fresh_candidates,
            1,
        )
        .expect("second reuse receipt");
    assert_eq!(
        PredicateGroupingPromotionConsolidator
            .consolidate(&learned_a, std::slice::from_ref(&receipt_a)),
        Err(PredicateGroupingPromotionError::InsufficientIndependentEvidence)
    );
    let promotion = PredicateGroupingPromotionConsolidator
        .consolidate(&learned_a, &[receipt_a.clone(), receipt_b.clone()])
        .expect("disjoint consolidation");

    let mut store = PredicateGroupingStore::new();
    let id = store
        .insert_provisional(learned_a)
        .expect("provisional mechanism");
    assert_eq!(
        store.module(id).expect("module").status(),
        ModuleStatus::Provisional
    );
    store
        .record_verified_reuse(id, receipt_a.clone())
        .expect("first ledger receipt");
    assert_eq!(
        store.record_verified_reuse(id, receipt_a),
        Err(PredicateGroupingStoreError::DuplicateReceipt)
    );
    assert!(!store.apply_verified_promotion(id, &promotion));
    store
        .record_verified_reuse(id, receipt_b)
        .expect("second ledger receipt");
    assert!(store.apply_verified_promotion(id, &promotion));

    let held_out = episode(
        "graphics-memory-held-out",
        &[(50, 60), (50, 61), (51, 60), (51, 61), (52, 60)],
        5_000,
    );
    let proposals = store
        .persistent_mechanisms()
        .map(|mechanism| PredicateGroupingInventor.predict(mechanism, &held_out.entities, limits))
        .collect::<Result<Vec<_>, _>>()
        .expect("proposal-only execution");
    assert_eq!(proposals, vec![held_out.outputs]);
    eprintln!(
        "PREDICATE_MEMORY_METRICS receipts=2 candidates={fresh_candidates}->1 examples=4->0 heldout_groups=5 exact=5 duplicate_receipts=0"
    );
}

fn episode(domain: &str, keys: &[(i64, i64)], value_base: i64) -> AttributedGroupingTrace {
    let mut entities = Vec::new();
    let mut outputs = BTreeSet::new();
    for (group_index, (first, second)) in keys.iter().copied().enumerate() {
        let cardinality = group_index + 2;
        outputs.insert(i64::try_from(cardinality - 1).expect("bounded cardinality"));
        for member in 0..cardinality {
            entities.push(AttributedGroupingEntity {
                id: EntityId(Hash32::digest(
                    format!("{domain}:{group_index}:{member}").as_bytes(),
                )),
                value: value_base
                    + i64::try_from(group_index).expect("bounded group") * 100
                    + i64::try_from(member).expect("bounded member"),
                features: vec![
                    first,
                    second,
                    i64::try_from(member).expect("bounded member"),
                ],
            });
        }
    }
    AttributedGroupingTrace {
        episode: Hash32::digest(domain.as_bytes()),
        entities,
        outputs,
    }
}
