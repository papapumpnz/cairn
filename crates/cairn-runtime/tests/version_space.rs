use cairn_core::Hash32;
use cairn_runtime::*;
use std::collections::BTreeMap;

#[test]
fn intervention_discriminates_and_refines_competing_theories() {
    let action = Hash32::digest(b"action");
    let first = Hash32::digest(b"first");
    let second = Hash32::digest(b"second");
    let mut space = VersionSpace::new(vec![
        TheoryHypothesis {
            id: Hash32::digest(b"t1"),
            predicted_outcomes: BTreeMap::from([(action, first)]),
        },
        TheoryHypothesis {
            id: Hash32::digest(b"t2"),
            predicted_outcomes: BTreeMap::from([(action, second)]),
        },
    ])
    .expect("version space");
    assert_eq!(
        space
            .next_discriminating_test()
            .expect("test")
            .outcome_partitions,
        2
    );
    space.observe(action, second).expect("refined");
    assert_eq!(space.len(), 1);
}
