use cairn_consolidator::{
    Consolidator, InternalMiningLimits, ProofDatum, ProofGraph, ProofNode, ProofValueType,
};
use cairn_core::Hash32;
use cairn_verifier::{InternalCompressionVerifier, InternalCompressionVerifyError};

#[test]
fn independently_reconstructs_internal_boundaries_costs_and_identity() {
    let sources = sources();
    let proposal = Consolidator
        .mine_internal_proof_subgraphs(&sources, limits())
        .expect("producer proposal");
    let verified = InternalCompressionVerifier
        .verify(&proposal, &sources)
        .expect("independent internal verification");
    assert_eq!(verified.proposal(), proposal.semantic_hash());
    assert_eq!(verified.replayed_sources(), 4);
    assert_eq!(verified.uncompressed_nodes(), 28);
    assert_eq!(verified.compressed_nodes(), 20);
    assert_eq!(verified.source_episodes().len(), 4);

    let reversed = sources.iter().cloned().rev().collect::<Vec<_>>();
    assert_eq!(
        InternalCompressionVerifier
            .verify(&proposal, &reversed)
            .expect("source order does not matter")
            .proposal(),
        proposal.semantic_hash()
    );
}

#[test]
fn changed_internal_content_identity_or_boundary_fails_closed() {
    let sources = sources();
    let proposal = Consolidator
        .mine_internal_proof_subgraphs(&sources, limits())
        .expect("producer proposal");

    let mut changed_content = sources.clone();
    changed_content[1].nodes[4].arguments[0] = datum(ProofValueType::Atom, "orange");
    assert_eq!(
        InternalCompressionVerifier.verify(&proposal, &changed_content),
        Err(InternalCompressionVerifyError::ReplayMismatch)
    );

    let mut changed_identity = sources.clone();
    changed_identity[1].episode = Hash32::digest(b"changed-episode");
    assert_eq!(
        InternalCompressionVerifier.verify(&proposal, &changed_identity),
        Err(InternalCompressionVerifyError::SourceMismatch)
    );

    let mut changed_boundary = sources.clone();
    changed_boundary[1].nodes[2].inputs = vec![1];
    assert_eq!(
        InternalCompressionVerifier.verify(&proposal, &changed_boundary),
        Err(InternalCompressionVerifyError::BoundaryMismatch)
    );
}

fn sources() -> [ProofGraph; 4] {
    [
        host_graph(1, 1, 1, "triangle", "red"),
        host_graph(2, 2, 2, "circle", "blue"),
        host_graph(3, 3, 1, "segment", "green"),
        host_graph(4, 1, 3, "polygon", "yellow"),
    ]
}

const fn limits() -> InternalMiningLimits {
    InternalMiningLimits {
        minimum_nodes: 4,
        maximum_nodes: 4,
        minimum_independent_episodes: 4,
        maximum_windows_per_episode: 64,
    }
}

fn host_graph(
    episode: u8,
    prefix_nodes: usize,
    suffix_nodes: usize,
    role: &str,
    effect: &str,
) -> ProofGraph {
    let mut nodes = vec![ProofNode {
        operation: "host-observation".to_owned(),
        output_type: ProofValueType::EntitySet,
        inputs: vec![],
        arguments: vec![datum(ProofValueType::Atom, &format!("world-{episode}"))],
    }];
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
