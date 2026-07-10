use cairn_core::{CanonicalEncode, Hash32};
use cairn_reg::{
    AttrKey, AttrValue, Attribute, AttributeFilter, DerivationId, DerivationKind, DerivationRecord,
    EvidenceWeight, LogicalTime, NodeConstraint, NodeKind, PathConstraint, QueryPlan, RegError,
    RegLimits, RelGraphBuilder, RelationDefinition, RelationPattern, Term,
};
use std::collections::BTreeSet;

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
        attribute_filters: Vec::new(),
        path_constraints: Vec::new(),
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
        attribute_filters: Vec::new(),
        path_constraints: Vec::new(),
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
fn temporal_and_fact_attribute_indexes_are_exact() {
    let (mut builder, observed) = builder_with_observation();
    let relation = builder
        .declare_relation(RelationDefinition {
            name: "changes".to_owned(),
            arguments: vec![NodeConstraint::Any, NodeConstraint::Any],
        })
        .expect("relation");
    let before = builder
        .add_node_at(NodeKind::State, Vec::new(), observed, Some(LogicalTime(10)))
        .expect("before");
    let after = builder
        .add_node_at(NodeKind::State, Vec::new(), observed, Some(LogicalTime(11)))
        .expect("after");
    let fact = builder
        .add_fact_at(
            relation,
            vec![before, after],
            vec![Attribute {
                key: AttrKey(9),
                value: AttrValue::Atom(3),
            }],
            EvidenceWeight::Exact,
            observed,
            Some(LogicalTime(11)),
        )
        .expect("change");
    let graph = builder.publish();
    assert_eq!(graph.graph().nodes_at_time(LogicalTime(10)), &[before]);
    assert_eq!(graph.graph().facts_at_time(LogicalTime(11)), &[fact]);
    assert_eq!(
        graph
            .graph()
            .facts_with_attribute(AttrKey(9), &AttrValue::Atom(3)),
        &[fact]
    );
    assert_eq!(graph.graph().derivations().len(), 1);
}

#[test]
fn bounded_paths_filters_and_aggregates_are_deterministic() {
    let (mut builder, observed) = builder_with_observation();
    let relation = builder
        .declare_relation(RelationDefinition {
            name: "causally_precedes".to_owned(),
            arguments: vec![NodeConstraint::Any, NodeConstraint::Any],
        })
        .expect("relation");
    let nodes = [
        builder
            .add_node(
                NodeKind::Event,
                vec![Attribute {
                    key: AttrKey(1),
                    value: AttrValue::Text("cause".to_owned()),
                }],
                observed,
            )
            .expect("a"),
        builder
            .add_node(NodeKind::Event, Vec::new(), observed)
            .expect("b"),
        builder
            .add_node(NodeKind::State, Vec::new(), observed)
            .expect("c"),
    ];
    for arguments in [vec![nodes[0], nodes[1]], vec![nodes[1], nodes[2]]] {
        builder
            .add_fact(
                relation,
                arguments,
                Vec::new(),
                EvidenceWeight::Exact,
                observed,
            )
            .expect("edge");
    }
    let graph = builder.publish();
    let too_short = QueryPlan {
        variable_count: 0,
        patterns: Vec::new(),
        attribute_filters: Vec::new(),
        path_constraints: vec![PathConstraint {
            relation,
            from: Term::Node(nodes[0]),
            to: Term::Node(nodes[2]),
            maximum_hops: 1,
            maximum_visited: 8,
        }],
        result_variables: Vec::new(),
    };
    assert!(too_short
        .execute(graph.graph())
        .expect("bounded miss")
        .rows
        .is_empty());
    let within_bound = QueryPlan {
        path_constraints: vec![PathConstraint {
            maximum_hops: 2,
            ..too_short.path_constraints[0]
        }],
        ..too_short
    };
    assert_eq!(
        within_bound
            .execute(graph.graph())
            .expect("bounded hit")
            .rows,
        vec![Vec::new()]
    );

    let filtered = QueryPlan {
        variable_count: 2,
        patterns: vec![RelationPattern {
            relation,
            arguments: vec![Term::Variable(0), Term::Variable(1)],
        }],
        attribute_filters: vec![AttributeFilter {
            variable: 0,
            key: AttrKey(1),
            value: AttrValue::Text("cause".to_owned()),
        }],
        path_constraints: Vec::new(),
        result_variables: vec![0, 1],
    }
    .execute(graph.graph())
    .expect("filtered");
    assert_eq!(filtered.rows, vec![vec![nodes[0], nodes[1]]]);
    assert_eq!(filtered.group_count(&[0]).expect("count")[0].count, 1);
}

#[test]
fn extending_a_snapshot_preserves_snapshot_isolation() {
    let (mut builder, observed) = builder_with_observation();
    let original_node = builder
        .add_node(NodeKind::Observation, Vec::new(), observed)
        .expect("original");
    let original = builder.publish();
    let original_bytes = original.encode_canonical().expect("original bytes");

    let mut extension = RelGraphBuilder::from_snapshot(&original, RegLimits::default());
    let derived = extension
        .add_derivation(DerivationRecord {
            kind: DerivationKind::Hypothesis,
            operation: Hash32::digest(b"extend"),
            inputs: vec![observed],
            parameters: Vec::new(),
            semantics_version: "test/1".to_owned(),
        })
        .expect("derived");
    let added = extension
        .add_node(NodeKind::EntityHypothesis, Vec::new(), derived)
        .expect("added");
    let extended = extension.publish();

    assert_eq!(original.graph().node_count(), 1);
    assert!(original.graph().node(original_node).is_some());
    assert!(original.graph().node(added).is_none());
    assert_eq!(extended.graph().node_count(), 2);
    assert_eq!(
        original.encode_canonical().expect("unchanged"),
        original_bytes
    );
}

#[test]
fn randomized_joins_match_a_simple_relational_reference() {
    let mut random = 0x243f_6a88_85a3_08d3_u64;
    for _case in 0..100 {
        let (mut builder, observed) = builder_with_observation();
        let relation = builder
            .declare_relation(RelationDefinition {
                name: "linked".to_owned(),
                arguments: vec![NodeConstraint::Any, NodeConstraint::Any],
            })
            .expect("relation");
        let nodes = (0..24)
            .map(|_| {
                builder
                    .add_node(NodeKind::EntityHypothesis, Vec::new(), observed)
                    .expect("node")
            })
            .collect::<Vec<_>>();
        let mut edges = Vec::new();
        for _ in 0..96 {
            let left = nodes[usize::from(next_random(&mut random) % 24)];
            let right = nodes[usize::from(next_random(&mut random) % 24)];
            edges.push((left, right));
            builder
                .add_fact(
                    relation,
                    vec![left, right],
                    Vec::new(),
                    EvidenceWeight::Exact,
                    observed,
                )
                .expect("fact");
        }
        let graph = builder.publish();
        let actual = QueryPlan {
            variable_count: 3,
            patterns: vec![
                RelationPattern {
                    relation,
                    arguments: vec![Term::Variable(0), Term::Variable(1)],
                },
                RelationPattern {
                    relation,
                    arguments: vec![Term::Variable(1), Term::Variable(2)],
                },
            ],
            attribute_filters: Vec::new(),
            path_constraints: Vec::new(),
            result_variables: vec![0, 1, 2],
        }
        .execute(graph.graph())
        .expect("query")
        .rows;
        let expected = edges
            .iter()
            .flat_map(|(left, middle)| {
                edges.iter().filter_map(move |(candidate, right)| {
                    (middle == candidate).then_some(vec![*left, *middle, *right])
                })
            })
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        assert_eq!(actual, expected);
    }
}

#[test]
fn hyperedge_order_and_query_caps_fail_closed() {
    let (mut builder, observed) = builder_with_observation();
    let relation = builder
        .declare_relation(RelationDefinition {
            name: "action_transition".to_owned(),
            arguments: vec![
                NodeConstraint::Kind(NodeKind::State),
                NodeConstraint::Kind(NodeKind::Action),
                NodeConstraint::Kind(NodeKind::State),
            ],
        })
        .expect("relation");
    let before = builder
        .add_node(NodeKind::State, Vec::new(), observed)
        .expect("before");
    let action = builder
        .add_node(NodeKind::Action, Vec::new(), observed)
        .expect("action");
    let after = builder
        .add_node(NodeKind::State, Vec::new(), observed)
        .expect("after");
    let fact = builder
        .add_fact(
            relation,
            vec![before, action, after],
            Vec::new(),
            EvidenceWeight::Exact,
            observed,
        )
        .expect("fact");
    let graph = builder.publish();
    assert_eq!(
        graph.graph().fact(fact).expect("fact").arguments,
        &[before, action, after]
    );
    let error = QueryPlan {
        variable_count: 0,
        patterns: Vec::new(),
        attribute_filters: Vec::new(),
        path_constraints: vec![PathConstraint {
            relation,
            from: Term::Node(before),
            to: Term::Node(after),
            maximum_hops: 1,
            maximum_visited: 8,
        }],
        result_variables: Vec::new(),
    }
    .execute(graph.graph())
    .expect_err("ternary relation is not a path");
    assert_eq!(error, RegError::PathRequiresBinaryRelation(relation));
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

fn next_random(state: &mut u64) -> u8 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    state.to_le_bytes()[0]
}
