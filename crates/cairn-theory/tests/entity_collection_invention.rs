use cairn_core::{
    EntityId, EntityRecord, EntitySelectionDemonstration, EntitySelectionProblem, FeatureId,
    Hash32, RelationRecord, RelationRole, RelationalEntityCollection,
};
use cairn_invent::InventionLimits;
use cairn_theory::RelationalEntitySelectorInventor;
use std::collections::{BTreeMap, BTreeSet};

#[test]
fn relational_entity_collection_selector_transfers_across_domains() {
    let vector = collection(
        "vector",
        &["group", "shape-a", "shape-b", "loose"],
        &[("group", "shape-a"), ("group", "shape-b")],
    );
    let sequence = collection(
        "sequence",
        &["segment", "token-a", "token-b", "token-c", "orphan"],
        &[
            ("segment", "token-a"),
            ("segment", "token-b"),
            ("segment", "token-c"),
        ],
    );
    let diagram = collection(
        "diagram",
        &["cluster", "node-a", "node-b", "isolated"],
        &[("cluster", "node-a"), ("cluster", "node-b")],
    );
    let problem = EntitySelectionProblem {
        demonstrations: vec![
            EntitySelectionDemonstration {
                collection: vector,
                selected: BTreeSet::from([id("vector:group")]),
            },
            EntitySelectionDemonstration {
                collection: sequence,
                selected: BTreeSet::from([id("sequence:segment")]),
            },
        ],
        prediction: diagram.clone(),
    };
    let learned = RelationalEntitySelectorInventor
        .invent(&problem, InventionLimits::default())
        .expect("relation-role selector");
    let selected = RelationalEntitySelectorInventor
        .predict(&learned, &diagram)
        .expect("cross-domain rebinding");
    assert_eq!(selected, BTreeSet::from([id("diagram:cluster")]));
    assert_eq!(learned.source_collections, 2);
}

fn collection(
    domain: &str,
    names: &[&str],
    membership: &[(&str, &str)],
) -> RelationalEntityCollection {
    RelationalEntityCollection {
        entities: names
            .iter()
            .enumerate()
            .map(|(index, name)| EntityRecord {
                id: id(&format!("{domain}:{name}")),
                attributes: BTreeMap::from([(
                    FeatureId("distractor".to_owned()),
                    i64::try_from((index + domain.len()) % 3).expect("small"),
                )]),
            })
            .collect(),
        relations: membership
            .iter()
            .map(|(subject, object)| RelationRecord {
                role: RelationRole("membership".to_owned()),
                subject: id(&format!("{domain}:{subject}")),
                object: id(&format!("{domain}:{object}")),
            })
            .collect(),
    }
}

fn id(value: &str) -> EntityId {
    EntityId(Hash32::digest(value.as_bytes()))
}
