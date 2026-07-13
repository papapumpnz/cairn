use cairn_core::{
    EntityId, EntityRecord, EntitySelectionDemonstration, EntitySelectionProblem, Hash32,
    RelationRecord, RelationRole, RelationalEntityCollection,
};
use cairn_invent::InventionLimits;
use cairn_theory::{EntityFeature, RelationalEntitySelectorInventor};
use std::collections::{BTreeMap, BTreeSet};

#[test]
fn invents_an_ordered_two_relation_path_against_one_edge_hard_negatives() {
    let first = family("vector", 2, 2);
    let second = family("workflow", 3, 3);
    let held_out = family("diagram", 5, 4);
    let problem = EntitySelectionProblem {
        demonstrations: vec![demonstration(&first), demonstration(&second)],
        prediction: held_out.0.clone(),
    };
    let learned = RelationalEntitySelectorInventor
        .invent(&problem, InventionLimits::default())
        .expect("two-hop selector");
    assert!(learned.features.contains(&EntityFeature::TwoHopOutDegree(
        RelationRole("links".to_owned()),
        RelationRole("activates".to_owned()),
    )));
    assert_eq!(
        RelationalEntitySelectorInventor
            .predict(&learned, &held_out.0)
            .expect("held-out path rebinding"),
        held_out.1
    );

    let mut path_removed = held_out.0.clone();
    path_removed
        .relations
        .retain(|relation| relation.role != RelationRole("activates".to_owned()));
    assert_ne!(
        RelationalEntitySelectorInventor
            .predict(&learned, &path_removed)
            .expect("path ablation"),
        held_out.1
    );
}

fn demonstration(
    pair: &(RelationalEntityCollection, BTreeSet<EntityId>),
) -> EntitySelectionDemonstration {
    EntitySelectionDemonstration {
        collection: pair.0.clone(),
        selected: pair.1.clone(),
    }
}

fn family(
    domain: &str,
    positive_count: usize,
    negative_count: usize,
) -> (RelationalEntityCollection, BTreeSet<EntityId>) {
    let mut entities = Vec::new();
    let mut relations = Vec::new();
    let mut selected = BTreeSet::new();
    for (positive, count) in [(true, positive_count), (false, negative_count)] {
        for index in 0..count {
            let root = id(&format!("{domain}:{positive}:root:{index}"));
            let bridge = id(&format!("{domain}:{positive}:bridge:{index}"));
            let endpoint = id(&format!("{domain}:{positive}:endpoint:{index}"));
            entities.extend([
                record(root.clone()),
                record(bridge.clone()),
                record(endpoint.clone()),
            ]);
            relations.push(RelationRecord {
                role: RelationRole("links".to_owned()),
                subject: root.clone(),
                object: bridge.clone(),
            });
            relations.push(RelationRecord {
                role: RelationRole(if positive { "activates" } else { "distracts" }.to_owned()),
                subject: bridge,
                object: endpoint,
            });
            if positive {
                selected.insert(root);
            }
        }
    }
    (
        RelationalEntityCollection {
            entities,
            relations,
        },
        selected,
    )
}

fn record(id: EntityId) -> EntityRecord {
    EntityRecord {
        id,
        attributes: BTreeMap::new(),
    }
}

fn id(value: &str) -> EntityId {
    EntityId(Hash32::digest(value.as_bytes()))
}
