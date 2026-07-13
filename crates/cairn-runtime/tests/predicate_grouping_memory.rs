use cairn_core::{EntityId, Hash32};
use cairn_invent::InventionLimits;
use cairn_memory::PredicateGroupingStore;
use cairn_rkl::ExecLimits;
use cairn_runtime::{CognitionError, CognitionRuntime, PredicateGroupingMemoryRuntimeError};
use cairn_theory::{
    AttributedGroupingEntity, AttributedGroupingTrace, PredicateGroupingInventor,
    PredicateGroupingLimits,
};
use cairn_verifier::{PredicateGroupingPromotionConsolidator, PredicateGroupingVerifier};
use std::collections::BTreeSet;

#[test]
fn shared_runtime_rebinds_only_a_proof_gated_predicate_mechanism() {
    let limits = PredicateGroupingLimits::default();
    let first = [
        episode("runtime-vector", &[(0, 0), (0, 1), (1, 0)], 1_000),
        episode("runtime-sequence", &[(2, 2), (2, 3), (3, 2), (3, 3)], 2_000),
    ];
    let second = [
        episode("runtime-table", &[(10, 20), (10, 21), (11, 20)], 3_000),
        episode(
            "runtime-workflow",
            &[(30, 40), (30, 41), (31, 40), (31, 41)],
            4_000,
        ),
    ];
    let learned = PredicateGroupingInventor
        .invent(&first, limits, InventionLimits::default())
        .expect("learned predicate");
    let corroborating = PredicateGroupingInventor
        .invent(&second, limits, InventionLimits::default())
        .expect("corroborating predicate");
    let candidates = 65;
    let receipts = [
        PredicateGroupingVerifier
            .verify_reuse(
                &learned,
                &first,
                limits,
                ExecLimits::default(),
                candidates,
                1,
            )
            .expect("first receipt"),
        PredicateGroupingVerifier
            .verify_reuse(
                &corroborating,
                &second,
                limits,
                ExecLimits::default(),
                candidates,
                1,
            )
            .expect("second receipt"),
    ];
    let promotion = PredicateGroupingPromotionConsolidator
        .consolidate(&learned, &receipts)
        .expect("promotion");
    let mut store = PredicateGroupingStore::new();
    let id = store
        .insert_provisional(learned)
        .expect("provisional mechanism");
    for receipt in receipts {
        store
            .record_verified_reuse(id, receipt)
            .expect("disjoint receipt");
    }
    assert!(store.apply_verified_promotion(id, &promotion));
    let runtime = CognitionRuntime::with_predicate_grouping_memory(store);
    let held_out = episode(
        "runtime-graphics-held-out",
        &[(50, 60), (50, 61), (51, 60), (51, 61), (52, 60)],
        5_000,
    );
    let outcome = runtime
        .propose_predicate_groupings_from_memory(&held_out.entities, limits)
        .expect("runtime proposal");
    assert_eq!(outcome.persistent_mechanisms_attempted, 1);
    assert_eq!(outcome.proposals, vec![held_out.outputs]);
    assert_eq!(
        CognitionRuntime::new().propose_predicate_groupings_from_memory(&held_out.entities, limits),
        Err(CognitionError::PredicateGroupingMemory(
            PredicateGroupingMemoryRuntimeError::NoVerifiedProposal
        ))
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
