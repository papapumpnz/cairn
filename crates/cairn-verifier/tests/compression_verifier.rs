use cairn_cas::{Cas, CasLimits};
use cairn_consolidator::{
    Consolidator, ProofDatum, ProofGraph, ProofGraphArchive, ProofGraphArtifactMetadata, ProofNode,
    ProofValueType,
};
use cairn_core::{ArtifactId, BudgetRecord, Hash32, ProducerRecord};
use cairn_verifier::{CompressionVerifier, CompressionVerifyError};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

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

#[test]
fn independently_verifies_compression_from_restarted_canonical_sources() {
    let directory = std::env::temp_dir().join(format!(
        "cairn-verifier-proof-restart-{}-{}",
        std::process::id(),
        NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed)
    ));
    let cas = Cas::open(&directory, CasLimits::default()).expect("open CAS");
    let sources = [graph(1, "triangle"), graph(2, "circle"), graph(3, "line")];
    let proposal = Consolidator
        .compress_proof_subgraphs(&sources)
        .expect("producer proposal");
    let archive = ProofGraphArchive;
    let stored = sources
        .iter()
        .enumerate()
        .map(|(index, source)| {
            archive
                .persist(
                    &cas,
                    source,
                    ProofGraphArtifactMetadata {
                        parent_ids: vec![ArtifactId::from_hash(Hash32::digest(
                            format!("source-certificate-{index}").as_bytes(),
                        ))],
                        producer: ProducerRecord {
                            component: "compression-producer".to_owned(),
                            version: "0.1.0".to_owned(),
                            commit: "test".to_owned(),
                            host_run_id: "before-restart".to_owned(),
                            agent_id: Some("producer".to_owned()),
                        },
                        budget: BudgetRecord {
                            work_units: 3,
                            cpu_nanos: 0,
                            peak_rss_bytes: 0,
                        },
                        created_at_logical: u64::try_from(index).expect("small"),
                    },
                )
                .expect("persist source")
        })
        .collect::<Vec<_>>();
    drop(cas);
    let cas = Cas::open(&directory, CasLimits::default()).expect("reopen CAS");
    let reloaded = stored
        .iter()
        .map(|stored| {
            archive
                .load(&cas, stored.envelope_blob_id)
                .expect("reload source")
        })
        .collect::<Vec<_>>();
    let verified = CompressionVerifier
        .verify(&proposal, &reloaded)
        .expect("independent restart verification");
    assert_eq!(verified.proposal(), proposal.semantic_hash());
    assert_eq!(verified.replayed_sources(), 3);
    drop(cas);
    std::fs::remove_dir_all(directory).expect("clean temporary CAS");
}
