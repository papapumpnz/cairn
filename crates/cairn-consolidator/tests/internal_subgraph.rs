use cairn_consolidator::{
    ConsolidationError, Consolidator, InternalEpisodeBindings, InternalMiningLimits,
    InternalPatternInput, ProofDatum, ProofGraph, ProofNode, ProofValueType,
};
use cairn_core::{CanonicalEncode, Hash32};

#[test]
fn mines_and_rebinds_an_internal_mechanism_across_different_hosts() {
    let sources = [
        host_graph(1, 1, 1, "triangle", "red"),
        host_graph(2, 2, 2, "circle", "blue"),
        host_graph(3, 3, 1, "segment", "green"),
        host_graph(4, 1, 3, "polygon", "yellow"),
    ];
    assert_eq!(
        Consolidator.compress_proof_subgraphs(&sources),
        Err(ConsolidationError::IncompatibleGraphShapes)
    );
    let limits = InternalMiningLimits {
        minimum_nodes: 4,
        maximum_nodes: 4,
        minimum_independent_episodes: 4,
        maximum_windows_per_episode: 64,
    };
    let proposal = Consolidator
        .mine_internal_proof_subgraphs(&sources, limits)
        .expect("embedded repeated mechanism");
    assert_eq!(proposal.pattern().len(), 4);
    assert_eq!(proposal.boundary_types(), [ProofValueType::EntitySet]);
    assert_eq!(proposal.bindings().len(), 4);
    assert_eq!(proposal.bindings()[0].values.len(), 2);
    assert!(proposal.pattern()[0]
        .inputs
        .contains(&InternalPatternInput::Boundary(0)));
    assert!(proposal.compressed_nodes() < proposal.uncompressed_nodes());
    assert_eq!(proposal.verified_replays(), 4);

    let held_out = host_graph(9, 4, 4, "star", "purple");
    let matches = Consolidator
        .match_internal_proof_subgraph(&proposal, &held_out, 64)
        .expect("held-out host rebinding");
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].values.len(), 2);
    assert_eq!(matches[0].boundary_nodes.len(), 1);
    assert_eq!(matches[0].selected_nodes, vec![4, 5, 6, 7]);

    let reversed = sources.iter().cloned().rev().collect::<Vec<_>>();
    let replayed = Consolidator
        .mine_internal_proof_subgraphs(&reversed, limits)
        .expect("order-independent mining");
    assert_eq!(proposal.semantic_hash(), replayed.semantic_hash());
    eprintln!(
        "INTERNAL_SUBGRAPH proposal={} hosts=4 held_out=1 nodes=4 boundary_parameters=1 value_parameters=2 cost={}->{}",
        proposal.semantic_hash(),
        proposal.uncompressed_nodes(),
        proposal.compressed_nodes()
    );
}

#[test]
fn compiles_a_source_independent_schema_and_expands_unseen_bindings() {
    let limits = InternalMiningLimits {
        minimum_nodes: 4,
        maximum_nodes: 4,
        minimum_independent_episodes: 4,
        maximum_windows_per_episode: 64,
    };
    let first_sources = [
        host_graph(1, 1, 1, "triangle", "red"),
        host_graph(2, 2, 2, "circle", "blue"),
        host_graph(3, 3, 1, "segment", "green"),
        host_graph(4, 1, 3, "polygon", "yellow"),
    ];
    let second_sources = [
        host_graph(11, 4, 1, "arc", "cyan"),
        host_graph(12, 2, 4, "box", "magenta"),
        host_graph(13, 3, 2, "line", "amber"),
        host_graph(14, 1, 5, "point", "white"),
    ];
    let first_proposal = Consolidator
        .mine_internal_proof_subgraphs(&first_sources, limits)
        .expect("first evidence cohort");
    let second_proposal = Consolidator
        .mine_internal_proof_subgraphs(&second_sources, limits)
        .expect("independent evidence cohort");
    assert_ne!(
        first_proposal.semantic_hash(),
        second_proposal.semantic_hash(),
        "evidence-bearing proposal identities must retain distinct lineage"
    );
    let first = Consolidator
        .compile_internal_proof_mechanism(&first_proposal)
        .expect("compiled first cohort");
    let second = Consolidator
        .compile_internal_proof_mechanism(&second_proposal)
        .expect("compiled second cohort");
    assert_eq!(first.semantic_hash(), second.semantic_hash());
    assert_eq!(
        first.semantic_hash(),
        Hash32::digest(&first.encode_canonical().expect("canonical mechanism"))
    );
    assert_eq!(
        first.encode_canonical().expect("first canonical mechanism"),
        second
            .encode_canonical()
            .expect("second canonical mechanism"),
        "source proposal lineage must not enter reusable mechanism semantics"
    );
    assert_ne!(first.source_proposal(), second.source_proposal());
    assert_eq!(first.parameter_types(), [ProofValueType::Atom; 2]);

    let held_out = host_graph(21, 5, 3, "spiral", "violet");
    let bindings = Consolidator
        .match_compiled_internal_mechanism(&first, &held_out, 64)
        .expect("compiled schema rebinding");
    assert_eq!(bindings.len(), 1);
    let expanded = Consolidator
        .expand_compiled_internal_mechanism(&first, &bindings[0])
        .expect("reference expansion");
    assert_eq!(expanded, held_out.nodes[5..9]);

    let mut mistyped = bindings[0].clone();
    mistyped.values[0] = datum(ProofValueType::Integer, "7");
    assert_eq!(
        Consolidator.expand_compiled_internal_mechanism(&first, &mistyped),
        Err(ConsolidationError::ReplayMismatch)
    );
    let missing_boundary = InternalEpisodeBindings {
        boundary_nodes: Vec::new(),
        ..bindings[0].clone()
    };
    assert_eq!(
        Consolidator.expand_compiled_internal_mechanism(&first, &missing_boundary),
        Err(ConsolidationError::ReplayMismatch)
    );
    eprintln!(
        "COMPILED_INTERNAL mechanism={} proposals=2 source_cohorts=8 held_out=1 exact_expansion=1",
        first.semantic_hash()
    );
}

#[test]
fn near_matches_types_and_resource_caps_fail_closed() {
    let mut sources = [
        host_graph(1, 1, 1, "triangle", "red"),
        host_graph(2, 2, 2, "circle", "blue"),
        host_graph(3, 3, 1, "segment", "green"),
        host_graph(4, 1, 3, "polygon", "yellow"),
    ];
    let limits = InternalMiningLimits {
        minimum_nodes: 4,
        maximum_nodes: 4,
        minimum_independent_episodes: 4,
        maximum_windows_per_episode: 64,
    };
    let internal_start = 3;
    sources[2].nodes[internal_start + 1].operation = "different-filter".to_owned();
    assert_eq!(
        Consolidator.mine_internal_proof_subgraphs(&sources, limits),
        Err(ConsolidationError::NoInternalCompression)
    );

    let clean = [
        host_graph(1, 1, 1, "triangle", "red"),
        host_graph(2, 2, 2, "circle", "blue"),
        host_graph(3, 3, 1, "segment", "green"),
        host_graph(4, 1, 3, "polygon", "yellow"),
    ];
    assert_eq!(
        Consolidator.mine_internal_proof_subgraphs(
            &clean,
            InternalMiningLimits {
                maximum_windows_per_episode: 1,
                ..limits
            }
        ),
        Err(ConsolidationError::Capacity)
    );
    assert_eq!(
        Consolidator.mine_internal_proof_subgraphs(
            &clean,
            InternalMiningLimits {
                minimum_nodes: 1,
                ..limits
            }
        ),
        Err(ConsolidationError::InvalidMiningLimits)
    );

    let proposal = Consolidator
        .mine_internal_proof_subgraphs(&clean, limits)
        .expect("clean proposal");
    let mut wrong_fixed_relation = host_graph(8, 2, 2, "star", "orange");
    let start = 2;
    wrong_fixed_relation.nodes[start + 1].arguments[0] =
        datum(ProofValueType::Relation, "overlaps");
    assert_eq!(
        Consolidator.match_internal_proof_subgraph(&proposal, &wrong_fixed_relation, 64),
        Err(ConsolidationError::NoInternalMatch)
    );
}

fn host_graph(
    episode: u8,
    prefix_nodes: usize,
    suffix_nodes: usize,
    role: &str,
    effect: &str,
) -> ProofGraph {
    let mut nodes = Vec::new();
    nodes.push(ProofNode {
        operation: "host-observation".to_owned(),
        output_type: ProofValueType::EntitySet,
        inputs: vec![],
        arguments: vec![datum(ProofValueType::Atom, &format!("world-{episode}"))],
    });
    for index in 1..prefix_nodes {
        nodes.push(ProofNode {
            operation: format!("host-prefix-{index}"),
            output_type: ProofValueType::Integer,
            inputs: vec![],
            arguments: vec![datum(ProofValueType::Integer, &index.to_string())],
        });
    }
    let start = nodes.len();
    nodes.extend([
        ProofNode {
            operation: "select-role".to_owned(),
            output_type: ProofValueType::EntitySet,
            inputs: vec![0],
            arguments: vec![datum(ProofValueType::Atom, role)],
        },
        ProofNode {
            operation: "filter-relation".to_owned(),
            output_type: ProofValueType::EntitySet,
            inputs: vec![u32::try_from(start).expect("small")],
            arguments: vec![datum(ProofValueType::Relation, "inside")],
        },
        ProofNode {
            operation: "map-effect".to_owned(),
            output_type: ProofValueType::Effect,
            inputs: vec![u32::try_from(start + 1).expect("small")],
            arguments: vec![datum(ProofValueType::Atom, effect)],
        },
        ProofNode {
            operation: "render-effect".to_owned(),
            output_type: ProofValueType::Effect,
            inputs: vec![u32::try_from(start + 2).expect("small")],
            arguments: vec![],
        },
    ]);
    let mut previous = start + 3;
    for index in 0..suffix_nodes {
        nodes.push(ProofNode {
            operation: format!("host-suffix-{episode}-{index}"),
            output_type: ProofValueType::Effect,
            inputs: vec![u32::try_from(previous).expect("small")],
            arguments: vec![],
        });
        previous += 1;
    }
    ProofGraph {
        episode: Hash32::digest(&[episode]),
        output: u32::try_from(nodes.len() - 1).expect("small"),
        nodes,
    }
}

fn datum(ty: ProofValueType, value: &str) -> ProofDatum {
    ProofDatum::digest(ty, value.as_bytes())
}
