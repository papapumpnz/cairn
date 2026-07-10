use cairn_core::{CanonicalEncode, Hash32};
use cairn_reg::{
    AttrKey, AttrValue, Attribute, DerivationId, DerivationKind, DerivationRecord, EvidenceWeight,
    NodeConstraint, NodeKind, QueryPlan, RegError, RegLimits, RelGraphBuilder, RelationDefinition,
    RelationPattern, Term,
};

#[test]
fn typed_relations_reject_wrong_kinds_and_arity() {
    let (mut builder, observed) = builder_with_observation();
    let relation = builder
        .declare_relation(RelationDefinition {
            name: "causes".to_owned(),
            arguments: vec![
                NodeConstraint::Kind(NodeKind::Action),
                NodeConstraint::Kind(NodeKind::Event),
            ],
        })
        .expect("relation");
    let action = builder
        .add_node(NodeKind::Action, Vec::new(), observed)
        .expect("action");
    let state = builder
        .add_node(NodeKind::State, Vec::new(), observed)
        .expect("state");
    assert!(matches!(
        builder.add_fact(
            relation,
            vec![action],
            Vec::new(),
            EvidenceWeight::Exact,
            observed,
        ),
        Err(RegError::InvalidArity {
            expected: 2,
            actual: 1,
            ..
        })
    ));
    assert!(matches!(
        builder.add_fact(
            relation,
            vec![action, state],
            Vec::new(),
            EvidenceWeight::Exact,
            observed,
        ),
        Err(RegError::NodeKindMismatch {
            position: 1,
            expected: NodeKind::Event,
            actual: NodeKind::State,
            ..
        })
    ));
}

#[test]
fn conjunctive_join_is_exact_indexed_and_deterministic() {
    let (graph, edge, nodes) = chain_graph().snapshot_parts();
    let plan = QueryPlan {
        variable_count: 3,
        patterns: vec![
            RelationPattern {
                relation: edge,
                arguments: vec![Term::Variable(0), Term::Variable(1)],
            },
            RelationPattern {
                relation: edge,
                arguments: vec![Term::Variable(1), Term::Variable(2)],
            },
        ],
        result_variables: vec![0, 1, 2],
    };
    let result = plan.execute(graph.graph()).expect("join");
    assert_eq!(result.rows, vec![vec![nodes[0], nodes[1], nodes[2]]]);
    assert_eq!(result.candidate_facts_examined, 4);
    assert_eq!(plan.execute(graph.graph()).expect("repeat"), result);
}

#[test]
fn canonical_graph_ignores_attribute_insertion_order() {
    let first = graph_with_attributes(vec![
        Attribute {
            key: AttrKey(2),
            value: AttrValue::Text("second".to_owned()),
        },
        Attribute {
            key: AttrKey(1),
            value: AttrValue::Int(1),
        },
    ]);
    let second = graph_with_attributes(vec![
        Attribute {
            key: AttrKey(1),
            value: AttrValue::Int(1),
        },
        Attribute {
            key: AttrKey(2),
            value: AttrValue::Text("second".to_owned()),
        },
    ]);
    assert_eq!(
        first.encode_canonical().expect("first"),
        second.encode_canonical().expect("second")
    );
    assert_eq!(
        first
            .graph()
            .nodes_with_attribute(AttrKey(1), &AttrValue::Int(1))
            .len(),
        1
    );
}

#[test]
fn published_snapshots_support_concurrent_deterministic_queries() {
    let (snapshot, relation, nodes) = chain_graph().snapshot_parts();
    let plan = QueryPlan {
        variable_count: 1,
        patterns: vec![RelationPattern {
            relation,
            arguments: vec![Term::Node(nodes[0]), Term::Variable(0)],
        }],
        result_variables: vec![0],
    };
    let expected = plan.execute(snapshot.graph()).expect("expected");
    std::thread::scope(|scope| {
        for _ in 0..8 {
            let snapshot = snapshot.clone();
            let plan = plan.clone();
            let expected = expected.clone();
            scope.spawn(move || {
                assert_eq!(plan.execute(snapshot.graph()).expect("query"), expected);
            });
        }
    });
}

#[test]
fn derivations_are_append_only_and_acyclic() {
    let mut builder = RelGraphBuilder::new(RegLimits::default());
    assert!(matches!(
        builder.add_derivation(DerivationRecord {
            kind: DerivationKind::Hypothesis,
            operation: Hash32::digest(b"future"),
            inputs: vec![DerivationId(0)],
            parameters: Vec::new(),
            semantics_version: "test/1".to_owned(),
        }),
        Err(RegError::UnknownDerivation(DerivationId(0)))
    ));
    let root = observed_derivation(&mut builder);
    let child = builder
        .add_derivation(DerivationRecord {
            kind: DerivationKind::DeterministicKernel,
            operation: Hash32::digest(b"derive"),
            inputs: vec![root],
            parameters: Vec::new(),
            semantics_version: "test/1".to_owned(),
        })
        .expect("child");
    assert_eq!(child, DerivationId(1));
}

struct ChainGraph {
    builder: RelGraphBuilder,
    relation: cairn_reg::RelationId,
    nodes: [cairn_reg::NodeId; 3],
}

impl ChainGraph {
    fn snapshot_parts(
        self,
    ) -> (
        cairn_reg::RelGraphSnapshot,
        cairn_reg::RelationId,
        [cairn_reg::NodeId; 3],
    ) {
        (self.builder.publish(), self.relation, self.nodes)
    }
}

fn chain_graph() -> ChainGraph {
    let (mut builder, observed) = builder_with_observation();
    let relation = builder
        .declare_relation(RelationDefinition {
            name: "precedes".to_owned(),
            arguments: vec![
                NodeConstraint::Kind(NodeKind::EntityHypothesis),
                NodeConstraint::Kind(NodeKind::EntityHypothesis),
            ],
        })
        .expect("relation");
    let nodes = [
        builder
            .add_node(NodeKind::EntityHypothesis, Vec::new(), observed)
            .expect("a"),
        builder
            .add_node(NodeKind::EntityHypothesis, Vec::new(), observed)
            .expect("b"),
        builder
            .add_node(NodeKind::EntityHypothesis, Vec::new(), observed)
            .expect("c"),
    ];
    for arguments in [
        vec![nodes[0], nodes[1]],
        vec![nodes[1], nodes[2]],
        vec![nodes[0], nodes[2]],
    ] {
        builder
            .add_fact(
                relation,
                arguments,
                Vec::new(),
                EvidenceWeight::Exact,
                observed,
            )
            .expect("fact");
    }
    ChainGraph {
        builder,
        relation,
        nodes,
    }
}

fn graph_with_attributes(attributes: Vec<Attribute>) -> cairn_reg::RelGraphSnapshot {
    let (mut builder, observed) = builder_with_observation();
    builder
        .add_node(NodeKind::Observation, attributes, observed)
        .expect("node");
    builder.publish()
}

fn builder_with_observation() -> (RelGraphBuilder, DerivationId) {
    let mut builder = RelGraphBuilder::new(RegLimits::default());
    let observed = observed_derivation(&mut builder);
    (builder, observed)
}

fn observed_derivation(builder: &mut RelGraphBuilder) -> DerivationId {
    builder
        .add_derivation(DerivationRecord {
            kind: DerivationKind::Observed,
            operation: Hash32::digest(b"observe"),
            inputs: Vec::new(),
            parameters: Vec::new(),
            semantics_version: "test/1".to_owned(),
        })
        .expect("observed derivation")
}
