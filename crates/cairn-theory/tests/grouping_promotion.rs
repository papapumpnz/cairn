use cairn_core::{Hash32, RelationRole};
use cairn_invent::InventionLimits;
use cairn_memory::{GroupingMergeStore, GroupingMergeStoreError, ModuleStatus};
use cairn_rkl::ExecLimits;
use cairn_runtime::{CognitionError, CognitionRuntime, GroupingMergeMemoryRuntimeError};
use cairn_theory::{GroupingHypothesisTrace, GroupingRelationInventor};
use cairn_verifier::{GroupingMergeVerifier, GroupingPromotionConsolidator};
use std::collections::{BTreeMap, BTreeSet};

#[test]
#[allow(clippy::too_many_lines)] // Keep invention, certification, transfer, and ablation together.
fn relation_bound_merge_is_certified_promoted_and_reused() {
    let first = [
        episode("vector-a", &[&[2, 5], &[10, 14, 17]]),
        episode("sequence-a", &[&[3, 9, 15], &[20, 26], &[31, 38, 41]]),
    ];
    let second = [
        episode(
            "table-b",
            &[&[4, 12, 19], &[25, 29, 41], &[50, 61], &[70, 73]],
        ),
        episode(
            "workflow-b",
            &[
                &[5, 14],
                &[30, 37, 45],
                &[60, 72],
                &[80, 85, 91],
                &[100, 119],
            ],
        ),
    ];
    let first_learned = GroupingRelationInventor
        .invent(&first, InventionLimits::default())
        .expect("first composition");
    let second_learned = GroupingRelationInventor
        .invent(&second, InventionLimits::default())
        .expect("second composition");
    let id = first_learned.semantic_hash().expect("identity");
    assert_eq!(id, second_learned.semantic_hash().expect("same identity"));

    let blank_candidates = first_learned
        .merge
        .constructor
        .proposal
        .candidates_evaluated
        .saturating_add(u64::try_from(first_learned.relations_evaluated).expect("relation count"));
    let first_receipt = GroupingMergeVerifier
        .verify_reuse(
            &first_learned,
            &first,
            blank_candidates,
            1,
            ExecLimits::default(),
        )
        .expect("first replay");
    let second_receipt = GroupingMergeVerifier
        .verify_reuse(
            &second_learned,
            &second,
            blank_candidates,
            1,
            ExecLimits::default(),
        )
        .expect("second replay");
    let certificate = GroupingPromotionConsolidator
        .consolidate(
            &first_learned,
            &[first_receipt.clone(), second_receipt.clone()],
        )
        .expect("promotion certificate");

    let mut store = GroupingMergeStore::new();
    assert_eq!(store.insert_provisional(first_learned.clone()), Ok(id));
    assert!(!store.apply_verified_promotion(id, &certificate));
    store
        .record_verified_reuse(id, first_receipt.clone())
        .expect("first receipt");
    assert_eq!(
        store.record_verified_reuse(id, first_receipt),
        Err(GroupingMergeStoreError::DuplicateReceipt)
    );
    store
        .record_verified_reuse(id, second_receipt)
        .expect("second receipt");
    assert!(store.apply_verified_promotion(id, &certificate));
    let module = store.module(id).expect("module");
    assert_eq!(module.status(), ModuleStatus::Persistent);
    assert_eq!(module.verified_reuses(), 2);

    let held_out_traces = [
        episode(
            "graphics-held-out",
            &[
                &[1, 8, 13, 25],
                &[30, 39],
                &[50, 64],
                &[80, 99],
                &[110, 137],
                &[150, 164, 171],
            ],
        ),
        episode(
            "operations-held-out",
            &[&[-20, -3], &[7, 12, 29], &[40, 61]],
        ),
    ];
    let held_out = held_out_traces[0].candidates.clone();
    let runtime = CognitionRuntime::with_grouping_merge_memory(store);
    let outcome = runtime
        .propose_grouping_merges_from_memory(&held_out)
        .expect("memory proposal");
    assert_eq!(outcome.persistent_mechanisms_attempted, 1);
    assert_eq!(
        outcome.proposals,
        vec![BTreeSet::from([9, 14, 19, 21, 24, 27])]
    );
    assert!(matches!(
        CognitionRuntime::new().propose_grouping_merges_from_memory(&held_out),
        Err(CognitionError::GroupingMergeMemory(
            GroupingMergeMemoryRuntimeError::NoVerifiedProposal
        ))
    ));
    let held_out_verification = GroupingMergeVerifier
        .verify_reuse(
            &second_learned,
            &held_out_traces,
            blank_candidates,
            1,
            ExecLimits::default(),
        )
        .expect("independent held-out verification");
    assert_eq!(held_out_verification.replayed_groups(), 9);

    let mut altered = first_learned;
    altered.relation = RelationRole("appearance-cluster".to_owned());
    let mut altered_store = GroupingMergeStore::new();
    let altered_id = altered_store
        .insert_provisional(altered)
        .expect("altered provisional");
    assert_ne!(altered_id, id);
    assert!(!altered_store.apply_verified_promotion(altered_id, &certificate));
    eprintln!(
        "GROUPING_PROMOTION_METRICS independent_cohorts=2 candidate_executions={blank_candidates}->1 examples=4->0 held_out_groups=9 exact=9 blank_proposals=0 forged_relation_promotions=0"
    );
}

fn episode(domain: &str, values: &[&[i64]]) -> GroupingHypothesisTrace {
    let candidates = candidates(values);
    let outputs = candidates[&RelationRole("structural-membership".to_owned())]
        .iter()
        .map(|group| group.last().expect("max") - group.first().expect("min"))
        .collect();
    GroupingHypothesisTrace {
        episode: Hash32::digest(domain.as_bytes()),
        candidates,
        outputs,
    }
}

fn candidates(values: &[&[i64]]) -> BTreeMap<RelationRole, Vec<BTreeSet<i64>>> {
    let correct = values
        .iter()
        .map(|group| group.iter().copied().collect())
        .collect::<Vec<_>>();
    let duplicate = correct.first().cloned().expect("group");
    BTreeMap::from([
        (
            RelationRole("structural-membership".to_owned()),
            correct.clone(),
        ),
        (
            RelationRole("appearance-cluster".to_owned()),
            vec![duplicate; correct.len()],
        ),
    ])
}
