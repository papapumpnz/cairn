use cairn_consolidator::*;
use cairn_core::Hash32;

#[test]
fn repeated_proof_structure_becomes_a_parameterized_proposal() {
    let traces = (0_u8..4)
        .map(|value| ProofTrace {
            episode: Hash32::digest(&[value]),
            operations: vec![
                "select".to_owned(),
                format!("role-{value}"),
                "order".to_owned(),
                "transform".to_owned(),
                "render".to_owned(),
            ],
        })
        .collect::<Vec<_>>();
    let proposal = Consolidator.anti_unify(&traces).expect("compressed");
    assert!(proposal.compressed_nodes < proposal.uncompressed_nodes);
    assert_eq!(proposal.pattern[1], PatternElement::Parameter(0));
}

#[test]
fn unrelated_traces_do_not_create_a_mechanism() {
    let traces = [
        ProofTrace {
            episode: Hash32::digest(b"a"),
            operations: vec!["one".to_owned(), "two".to_owned()],
        },
        ProofTrace {
            episode: Hash32::digest(b"b"),
            operations: vec!["three".to_owned(), "four".to_owned()],
        },
    ];
    assert_eq!(
        Consolidator.anti_unify(&traces),
        Err(ConsolidationError::NoSharedStructure)
    );
}

fn datum(ty: ProofValueType, value: &str) -> ProofDatum {
    ProofDatum::digest(ty, value.as_bytes())
}

fn enclosed_transform_graph(episode: u8, role: &str, color: &str) -> ProofGraph {
    ProofGraph {
        episode: Hash32::digest(&[episode]),
        nodes: vec![
            ProofNode {
                operation: "observe-entities".to_owned(),
                output_type: ProofValueType::EntitySet,
                inputs: vec![],
                arguments: vec![datum(ProofValueType::Atom, role)],
            },
            // This unrelated perception hypothesis must not enter the selected
            // ancestral proof subgraph.
            ProofNode {
                operation: "unused-contour-hypothesis".to_owned(),
                output_type: ProofValueType::Entity,
                inputs: vec![],
                arguments: vec![datum(ProofValueType::Atom, "unused")],
            },
            ProofNode {
                operation: "filter-by-relation".to_owned(),
                output_type: ProofValueType::EntitySet,
                inputs: vec![0],
                arguments: vec![datum(ProofValueType::Relation, "strictly-enclosed")],
            },
            ProofNode {
                operation: "map-effect".to_owned(),
                output_type: ProofValueType::Effect,
                inputs: vec![2],
                arguments: vec![datum(ProofValueType::Atom, color)],
            },
            ProofNode {
                operation: "render".to_owned(),
                output_type: ProofValueType::Effect,
                inputs: vec![3],
                arguments: vec![],
            },
        ],
        output: 4,
    }
}

#[test]
fn typed_proof_dags_compress_rebind_and_replay_exactly() {
    let graphs = [
        enclosed_transform_graph(1, "triangle", "red"),
        enclosed_transform_graph(2, "circle", "blue"),
        enclosed_transform_graph(3, "segment", "green"),
        enclosed_transform_graph(4, "polygon", "yellow"),
    ];
    let proposal = Consolidator
        .compress_proof_subgraphs(&graphs)
        .expect("typed proof compression");

    assert_eq!(
        proposal.pattern().len(),
        4,
        "unused graph node was excluded"
    );
    assert_eq!(proposal.bindings().len(), 4);
    assert_eq!(proposal.bindings()[0].values.len(), 2);
    assert_eq!(proposal.verified_replays(), 4);
    assert!(proposal.compressed_nodes() < proposal.uncompressed_nodes());
    let reversed = graphs.iter().cloned().rev().collect::<Vec<_>>();
    let replayed = Consolidator
        .compress_proof_subgraphs(&reversed)
        .expect("order-independent compression");
    assert_eq!(proposal.semantic_hash(), replayed.semantic_hash());
}

#[test]
fn topology_or_type_changes_cannot_be_hidden_in_a_parameter() {
    let first = enclosed_transform_graph(1, "triangle", "red");
    let mut changed_topology = enclosed_transform_graph(2, "circle", "blue");
    changed_topology.nodes[3].inputs = vec![0, 2];
    assert_eq!(
        Consolidator.compress_proof_subgraphs(&[first.clone(), changed_topology]),
        Err(ConsolidationError::IncompatibleOperation)
    );

    let mut changed_type = enclosed_transform_graph(3, "segment", "green");
    changed_type.nodes[2].output_type = ProofValueType::Entity;
    assert_eq!(
        Consolidator.compress_proof_subgraphs(&[first, changed_type]),
        Err(ConsolidationError::IncompatibleTypes)
    );
}

#[test]
fn malformed_or_duplicate_graph_evidence_is_rejected() {
    let first = enclosed_transform_graph(1, "triangle", "red");
    let mut duplicate = enclosed_transform_graph(1, "circle", "blue");
    assert_eq!(
        Consolidator.compress_proof_subgraphs(&[first.clone(), duplicate.clone()]),
        Err(ConsolidationError::DuplicateEpisode)
    );
    duplicate.episode = Hash32::digest(b"independent");
    duplicate.nodes[2].inputs = vec![4];
    assert_eq!(
        Consolidator.compress_proof_subgraphs(&[first, duplicate]),
        Err(ConsolidationError::InvalidTopology)
    );
}

#[test]
fn offline_runtime_discovers_cohorts_without_family_labels() {
    let mut runtime = ConsolidationRuntime::new(ConsolidationLimits {
        maximum_episodes: 8,
        maximum_nodes_per_episode: 8,
        minimum_independent_episodes: 3,
    });
    for graph in [
        enclosed_transform_graph(1, "triangle", "red"),
        enclosed_transform_graph(2, "circle", "blue"),
        enclosed_transform_graph(3, "segment", "green"),
    ] {
        runtime.submit(graph).expect("admitted episode");
    }
    runtime
        .submit(ProofGraph {
            episode: Hash32::digest(b"unrelated"),
            nodes: vec![
                ProofNode {
                    operation: "sense".to_owned(),
                    output_type: ProofValueType::Integer,
                    inputs: vec![],
                    arguments: vec![],
                },
                ProofNode {
                    operation: "act".to_owned(),
                    output_type: ProofValueType::Action,
                    inputs: vec![0],
                    arguments: vec![],
                },
            ],
            output: 1,
        })
        .expect("unrelated episode admitted");

    let report = runtime.consolidate().expect("offline consolidation");
    assert_eq!(report.metrics.episodes_compared, 4);
    assert_eq!(report.metrics.structural_cohorts, 2);
    assert_eq!(report.metrics.proposals_created, 1);
    assert_eq!(report.proposals[0].verified_replays(), 3);
}
