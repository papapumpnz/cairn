use cairn_core::{Hash32, RelationRole};
use cairn_invent::InventionLimits;
use cairn_theory::{GroupingHypothesisTrace, GroupingRelationInventor, GroupingSelectionError};
use std::collections::{BTreeMap, BTreeSet};

#[test]
fn selects_the_only_grouping_supporting_one_executable_theory() {
    let traces = [
        episode("vector", &[&[2, 5], &[10, 17]]),
        episode("sequence", &[&[3, 9, 15], &[20, 26], &[31, 41]]),
    ];
    let learned = GroupingRelationInventor
        .invent(&traces, InventionLimits::default())
        .expect("joint grouping");
    assert_eq!(
        learned.relation,
        RelationRole("structural-membership".to_owned())
    );
    assert_eq!(learned.relations_evaluated, 2);

    let held_out = candidates(&[&[1, 8, 13, 25], &[30, 39], &[50, 64], &[80, 99]]);
    assert_eq!(
        GroupingRelationInventor
            .predict(&learned, &held_out)
            .expect("rebind"),
        BTreeSet::from([9, 14, 19, 24])
    );
    let mut missing = held_out;
    missing.remove(&learned.relation);
    assert_eq!(
        GroupingRelationInventor.predict(&learned, &missing),
        Err(GroupingSelectionError::MissingRelation)
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
