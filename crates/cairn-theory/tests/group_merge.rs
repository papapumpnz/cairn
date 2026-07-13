use cairn_core::Hash32;
use cairn_invent::InventionLimits;
use cairn_theory::{GroupMergeInventor, GroupMergeTrace};
use std::collections::BTreeSet;

#[test]
fn invents_many_to_one_span_aggregation_without_group_output_pairing() {
    let traces = [
        trace("vector", &[&[2, 5], &[10, 14, 17]]),
        trace("sequence", &[&[3, 9, 11, 15], &[20, 26], &[31, 38, 41]]),
        trace(
            "table",
            &[&[4, 12, 19], &[25, 29, 34, 41], &[50, 61], &[70, 73]],
        ),
    ];
    let learned = GroupMergeInventor
        .invent(&traces, InventionLimits::default())
        .expect("group merge");
    let held_out = groups(&[
        &[1, 8, 13, 21, 25],
        &[30, 32, 39],
        &[50, 64],
        &[80, 83, 91, 99],
    ]);
    assert_eq!(
        GroupMergeInventor
            .merge(&learned, &held_out)
            .expect("held-out merge"),
        BTreeSet::from([9, 14, 19, 24])
    );

    let reordered = [
        trace("vector", &[&[17, 10, 14], &[5, 2]]),
        trace("sequence", &[&[38, 31, 41], &[15, 3, 11, 9], &[26, 20]]),
        trace(
            "table",
            &[&[73, 70], &[41, 25, 34, 29], &[61, 50], &[19, 4, 12]],
        ),
    ];
    let reordered = GroupMergeInventor
        .invent(&reordered, InventionLimits::default())
        .expect("order-invariant merge");
    assert_eq!(
        learned.constructor.proposal.semantic_hash,
        reordered.constructor.proposal.semantic_hash
    );
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
