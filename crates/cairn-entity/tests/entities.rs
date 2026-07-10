use cairn_core::Hash32;
use cairn_entity::{EntityDemand, EntityExpr, EntityGenerator};
use cairn_reg::{
    AttrKey, AttrValue, Attribute, DerivationKind, DerivationRecord, EvidenceWeight,
    NodeConstraint, NodeKind, RegLimits, RelGraphBuilder, RelationDefinition,
};
use std::collections::BTreeSet;

#[test]
fn relation_defined_components_and_attribute_groups_are_demanded() {
    let mut builder = RelGraphBuilder::new(RegLimits::default());
    let derivation = builder
        .add_derivation(DerivationRecord {
            kind: DerivationKind::Observed,
            operation: Hash32::digest(b"fixture"),
            inputs: Vec::new(),
            parameters: Vec::new(),
            semantics_version: "test/1".to_owned(),
        })
        .expect("derivation");
    let relation = builder
        .declare_relation(RelationDefinition {
            name: "frontend.adjacent".to_owned(),
            arguments: vec![NodeConstraint::Any, NodeConstraint::Any],
        })
        .expect("relation");
    let key = AttrKey(1);
    let nodes = [0_i64, 0, 1]
        .into_iter()
        .map(|group| {
            builder
                .add_node(
                    NodeKind::Scalar,
                    vec![Attribute {
                        key,
                        value: AttrValue::Int(group),
                    }],
                    derivation,
                )
                .expect("node")
        })
        .collect::<Vec<_>>();
    builder
        .add_fact(
            relation,
            vec![nodes[0], nodes[1]],
            Vec::new(),
            EvidenceWeight::Exact,
            derivation,
        )
        .expect("edge");
    let graph = builder.publish();
    let result = EntityGenerator
        .generate(
            graph.graph(),
            &EntityDemand {
                members: nodes.iter().copied().collect(),
                adjacency_relations: vec![relation],
                grouping_attributes: vec![key],
                maximum_cost: 2,
                maximum_hypotheses: 10,
            },
        )
        .expect("entities");
    assert!(result.hypotheses.iter().any(|entity| {
        matches!(entity.expression, EntityExpr::Connected { .. })
            && entity.members == BTreeSet::from([nodes[0], nodes[1]])
    }));
    assert!(result.hypotheses.iter().any(|entity| {
        matches!(entity.expression, EntityExpr::GroupByAttribute { .. })
            && entity.members == BTreeSet::from([nodes[0], nodes[1]])
    }));
}
