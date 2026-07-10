use cairn_events::{EventEntity, EventGenerator, EventKind, EventLimits};
use cairn_reg::{AttrKey, AttrValue, NodeId};
use std::collections::{BTreeMap, BTreeSet};

fn entity(id: u32, members: &[u32], value: i64) -> EventEntity {
    EventEntity {
        id,
        members: members.iter().copied().map(NodeId).collect(),
        attributes: BTreeMap::from([(AttrKey(1), AttrValue::Int(value))]),
    }
}

#[test]
fn preserves_updates_creates_deletes_splits_and_merges_remain_available() {
    let before = vec![entity(1, &[1, 2], 0), entity(2, &[8], 0)];
    let after = vec![
        entity(10, &[1], 1),
        entity(11, &[2], 1),
        entity(12, &[9], 2),
    ];
    let result = EventGenerator
        .generate(
            &before,
            &after,
            EventLimits {
                maximum_hypotheses: 20,
                maximum_arity: 2,
            },
        )
        .expect("events");
    let kinds = result
        .hypotheses
        .iter()
        .map(|event| event.kind.clone())
        .collect::<BTreeSet<_>>();
    assert!(kinds.contains(&EventKind::Update));
    assert!(kinds.contains(&EventKind::Create));
    assert!(kinds.contains(&EventKind::Delete));
    assert!(kinds.contains(&EventKind::Split));
}
