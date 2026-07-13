use cairn_core::Hash32;
use cairn_invent::InventionLimits;
use cairn_memory::{GroupMergePromotion, GroupMergeStore, ModuleStatus};
use cairn_rkl::ExecLimits;
use cairn_runtime::{CognitionError, CognitionRuntime, GroupMergeMemoryRuntimeError};
use cairn_theory::{GroupMergeInventor, GroupMergeTrace};
use cairn_verifier::{GroupMergeVerifier, GroupMergeVerifyError};
use std::collections::BTreeSet;

#[test]
fn many_to_one_aggregate_promotes_and_removes_future_examples() {
    let first = [
        trace("vector-a", &[&[2, 5], &[10, 14, 17]]),
        trace("sequence-a", &[&[3, 9, 15], &[20, 26], &[31, 38, 41]]),
    ];
    let second = [
        trace(
            "table-a",
            &[&[4, 12, 19], &[25, 29, 41], &[50, 61], &[70, 73]],
        ),
        trace(
            "workflow-a",
            &[
                &[5, 14],
                &[30, 37, 45],
                &[60, 72],
                &[80, 85, 91],
                &[100, 119],
            ],
        ),
    ];
    let first_learned = GroupMergeInventor
        .invent(&first, InventionLimits::default())
        .expect("first");
    let second_learned = GroupMergeInventor
        .invent(&second, InventionLimits::default())
        .expect("second");
    assert_eq!(
        first_learned.semantic_hash().expect("id"),
        second_learned.semantic_hash().expect("id")
    );
    let first_receipt = GroupMergeVerifier
        .verify(&first_learned, &first, ExecLimits::default())
        .expect("verify first");
    let second_receipt = GroupMergeVerifier
        .verify(&second_learned, &second, ExecLimits::default())
        .expect("verify second");
    let mut store = GroupMergeStore::new();
    assert_eq!(
        store
            .record_verified_cohort(&first_learned, first_receipt)
            .expect("provisional"),
        GroupMergePromotion::Provisional {
            verified_cohorts: 1
        }
    );
    assert_eq!(
        store
            .record_verified_cohort(&second_learned, second_receipt)
            .expect("persistent"),
        GroupMergePromotion::Persistent {
            verified_cohorts: 2
        }
    );
    let id = first_learned.semantic_hash().expect("id");
    assert_eq!(
        store.module(id).expect("module").status(),
        ModuleStatus::Persistent
    );
    assert_eq!(store.module(id).expect("module").verified_experiences(), 4);

    let held_out = groups(&[
        &[1, 8, 13, 21, 25, 30],
        &[40, 49, 55],
        &[70, 84],
        &[100, 107, 116, 125],
        &[150, 166],
        &[200, 203, 211],
    ]);
    let runtime = CognitionRuntime::with_group_merge_memory(store);
    let outcome = runtime
        .propose_group_merges_from_memory(&held_out)
        .expect("reuse");
    assert_eq!(outcome.persistent_mechanisms_attempted, 1);
    assert_eq!(
        outcome.proposals,
        vec![BTreeSet::from([11, 14, 15, 16, 25, 29])]
    );
    assert!(first_learned.constructor.proposal.candidates_evaluated > 1);
    assert!(matches!(
        CognitionRuntime::new().propose_group_merges_from_memory(&held_out),
        Err(CognitionError::GroupMergeMemory(
            GroupMergeMemoryRuntimeError::NoVerifiedProposal
        ))
    ));

    let wrong = [
        GroupMergeTrace {
            episode: Hash32::digest(b"wrong-a"),
            groups: groups(&[&[2, 5], &[10, 14, 17]]),
            outputs: BTreeSet::from([7, 17]),
        },
        GroupMergeTrace {
            episode: Hash32::digest(b"wrong-b"),
            groups: groups(&[&[3, 9, 15], &[20, 26], &[31, 38, 41]]),
            outputs: BTreeSet::from([15, 26, 41]),
        },
    ];
    assert_eq!(
        GroupMergeVerifier.verify(&first_learned, &wrong, ExecLimits::default()),
        Err(GroupMergeVerifyError::ReplayMismatch)
    );
    eprintln!("G2_GROUP_MERGE cohorts=2 experiences=4 fresh_candidates={} memory_attempts=1 memory_examples=0 heldout_groups=6 wrong_accepts=0", first_learned.constructor.proposal.candidates_evaluated);
}

fn trace(domain: &str, values: &[&[i64]]) -> GroupMergeTrace {
    let groups = groups(values);
    let outputs = groups
        .iter()
        .map(|group| group.last().expect("max") - group.first().expect("min"))
        .collect();
    GroupMergeTrace {
        episode: Hash32::digest(domain.as_bytes()),
        groups,
        outputs,
    }
}

fn groups(values: &[&[i64]]) -> Vec<BTreeSet<i64>> {
    values
        .iter()
        .map(|group| group.iter().copied().collect())
        .collect()
}
