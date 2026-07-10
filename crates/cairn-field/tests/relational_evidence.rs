use cairn_core::{Hash32, SourceReference};
use cairn_entity::{EntityDemand, EntityExpr, EntityGenerator};
use cairn_field::FiniteField;
use cairn_reg::{AttrValue, RegLimits};
use std::collections::BTreeSet;

#[test]
fn exact_field_observation_emits_relations_for_demand_driven_entities() {
    let field = FiniteField {
        shape: vec![3, 4],
        values: vec![0, 7, 7, 0, 0, 7, 0, 3, 2, 0, 3, 3],
    };
    let observation = field
        .observation(SourceReference {
            locator: "urn:test:diagram".to_owned(),
            content_hash: Hash32::digest(b"diagram"),
        })
        .expect("observation");
    let evidence = field
        .evidence_graph(&observation, RegLimits::default())
        .expect("relational evidence");
    assert_eq!(evidence.cells.len(), 12);
    assert_eq!(
        evidence
            .graph
            .graph()
            .nodes_with_attribute(evidence.value_key, &AttrValue::Int(7))
            .len(),
        3
    );

    let non_background = field
        .values
        .iter()
        .enumerate()
        .filter_map(|(index, value)| (*value != 0).then_some(evidence.cells[index]))
        .collect::<BTreeSet<_>>();
    let entities = EntityGenerator
        .generate(
            evidence.graph.graph(),
            &EntityDemand {
                members: non_background,
                adjacency_relations: vec![evidence.same_value_adjacent],
                grouping_attributes: vec![evidence.value_key],
                maximum_cost: 2,
                maximum_hypotheses: 16,
            },
        )
        .expect("entities");
    assert!(entities.hypotheses.iter().any(|entity| {
        matches!(entity.expression, EntityExpr::Connected { .. }) && entity.members.len() == 3
    }));
    assert!(entities.hypotheses.iter().any(|entity| {
        matches!(entity.expression, EntityExpr::GroupByAttribute { .. })
            && entity.members.len() == 3
    }));
}
