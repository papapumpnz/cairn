use cairn_consolidator::{Consolidator, ProofDatum, ProofGraph, ProofNode, ProofValueType};
use cairn_core::Hash32;
use cairn_verifier::{CompressionVerifier, CompressionVerifyError};

fn datum(value: &str) -> ProofDatum {
    ProofDatum::digest(ProofValueType::Atom, value.as_bytes())
}

fn graph(episode: u8, role: &str) -> ProofGraph {
    ProofGraph {
        episode: Hash32::digest(&[episode]),
        nodes: vec![
            ProofNode {
                operation: "observe".to_owned(),
                output_type: ProofValueType::EntitySet,
                inputs: vec![],
                arguments: vec![datum(role)],
            },
            ProofNode {
                operation: "select".to_owned(),
                output_type: ProofValueType::EntitySet,
                inputs: vec![0],
                arguments: vec![ProofDatum::digest(ProofValueType::Relation, b"inside")],
            },
            ProofNode {
                operation: "emit".to_owned(),
                output_type: ProofValueType::Effect,
                inputs: vec![1],
                arguments: vec![],
            },
        ],
        output: 2,
    }
}

#[test]
fn independent_path_replays_graphs_and_recalculates_costs() {
    let sources = [graph(1, "triangle"), graph(2, "circle"), graph(3, "line")];
    let proposal = Consolidator
        .compress_proof_subgraphs(&sources)
        .expect("producer proposal");
    let verified = CompressionVerifier
        .verify(&proposal, &sources)
        .expect("independent verification");

    assert_eq!(verified.proposal(), proposal.semantic_hash());
    assert_eq!(verified.replayed_sources(), 3);
    assert_eq!(verified.uncompressed_nodes(), proposal.uncompressed_nodes());
    assert_eq!(verified.compressed_nodes(), proposal.compressed_nodes());
}

#[test]
fn changed_source_content_or_identity_fails_closed() {
    let sources = [graph(1, "triangle"), graph(2, "circle"), graph(3, "line")];
    let proposal = Consolidator
        .compress_proof_subgraphs(&sources)
        .expect("producer proposal");

    let mut changed_content = sources.clone();
    changed_content[1].nodes[0].arguments[0] = datum("square");
    assert_eq!(
        CompressionVerifier.verify(&proposal, &changed_content),
        Err(CompressionVerifyError::ReplayMismatch)
    );

    let mut changed_identity = sources.clone();
    changed_identity[1].episode = Hash32::digest(b"different");
    assert_eq!(
        CompressionVerifier.verify(&proposal, &changed_identity),
        Err(CompressionVerifyError::SourceMismatch)
    );
}

#[test]
fn malformed_graph_fails_before_replay() {
    let sources = [graph(1, "triangle"), graph(2, "circle"), graph(3, "line")];
    let proposal = Consolidator
        .compress_proof_subgraphs(&sources)
        .expect("producer proposal");
    let mut malformed = sources.clone();
    malformed[1].nodes[1].inputs = vec![2];
    assert_eq!(
        CompressionVerifier.verify(&proposal, &malformed),
        Err(CompressionVerifyError::InvalidTopology)
    );
}
